//! MCP integration tests: an in-process client over a duplex transport, plus
//! one stdio session against the built binary.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use ontodoc::cli::{App, MAX_RESPONSE_BYTES};
use ontodoc::mcp::OntoDocServer;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph");
    for entry in walk(&src) {
        let rel = entry.strip_prefix(&src).unwrap();
        let to = dir.path().join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(&entry, to).unwrap();
    }
    App::new(dir.path()).index().unwrap();
    dir
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

async fn client(root: &Path) -> RunningService<RoleClient, ()> {
    let (server_io, client_io) = tokio::io::duplex(1 << 20);
    let server = OntoDocServer::new(root);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    ().serve(client_io).await.unwrap()
}

async fn call(
    client: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: Value,
) -> Result<CallToolResult, rmcp::ServiceError> {
    let args = args.as_object().cloned().unwrap_or_default();
    client
        .peer()
        .call_tool(CallToolRequestParams::new(name).with_arguments(args))
        .await
}

async fn ok(client: &RunningService<RoleClient, ()>, name: &'static str, args: Value) -> Value {
    let result = call(client, name, args).await.unwrap();
    assert_ne!(result.is_error, Some(true), "{name}: {result:?}");
    result.structured_content.unwrap()
}

#[tokio::test]
async fn lists_exactly_five_read_only_tools_with_schemas() {
    let dir = fixture();
    let client = client(dir.path()).await;
    let tools = client.peer().list_all_tools().await.unwrap();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort();
    assert_eq!(names, vec!["read", "related", "search", "status", "verify"]);
    for tool in &tools {
        let annotations = tool.annotations.as_ref().unwrap();
        assert_eq!(annotations.read_only_hint, Some(true), "{}", tool.name);
        assert_eq!(tool.input_schema.get("type"), Some(&json!("object")));
    }
    let search = tools.iter().find(|t| t.name == "search").unwrap();
    let props = search.input_schema["properties"].as_object().unwrap();
    for key in ["query", "mode", "top_k", "scope", "explain", "budget_bytes"] {
        assert!(props.contains_key(key), "search schema lacks {key}");
    }
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn tools_return_bounded_cited_fresh_json() {
    let dir = fixture();
    let client = client(dir.path()).await;

    let status = ok(&client, "status", json!({})).await;
    assert_eq!(status["fresh"], true);
    assert_eq!(status["chunks"], 8);
    let generation = status["generation_id"].as_str().unwrap().to_string();

    // Default mode is lexical.
    let search = ok(&client, "search", json!({ "query": "beta setup" })).await;
    assert_eq!(search["mode"], "lexical");
    assert_eq!(search["fresh"], true);
    assert_eq!(search["truncated"], false);
    let top = &search["results"][0];
    assert_eq!(top["citation"]["generation_id"], generation.as_str());
    assert!(top["citation"]["sha256"].as_str().unwrap().len() == 64);

    for mode in ["graph", "hier"] {
        let r = ok(
            &client,
            "search",
            json!({ "query": "cfg", "mode": mode, "scope": "b.md", "explain": true, "budget_bytes": 500 }),
        )
        .await;
        assert_eq!(r["mode"], mode);
        for result in r["results"].as_array().unwrap() {
            assert!(result["chunk_id"].as_str().unwrap().starts_with("b.md#"));
            assert!(result["explain"].is_object());
        }
    }

    let read = ok(&client, "read", json!({ "id": "a.md#details" })).await;
    assert_eq!(read["kind"], "section");
    let related = ok(&client, "related", json!({ "chunk_id": "a.md#alpha~0" })).await;
    assert!(!related["neighbours"].as_array().unwrap().is_empty());

    let b_citation = search["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["citation"]["path"] == "b.md")
        .unwrap()["citation"]
        .clone();
    let verify = ok(
        &client,
        "verify",
        json!({ "citations": [b_citation.clone()] }),
    )
    .await;
    assert_eq!(verify["results"][0]["state"], "valid");

    // An edit makes responses report staleness without reindexing.
    fs::write(dir.path().join("b.md"), "Changed.\n").unwrap();
    let status = ok(&client, "status", json!({})).await;
    assert_eq!(status["fresh"], false);
    assert_eq!(status["modified"], json!(["b.md"]));
    let verify = ok(&client, "verify", json!({ "citations": [b_citation] })).await;
    assert_eq!(verify["results"][0]["state"], "stale");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn verify_pins_receiving_root_and_reloads_only_its_active_generation() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    let twin = parent.path().join("twin");
    for dir in [&root, &twin] {
        fs::create_dir(dir).unwrap();
        fs::write(dir.join("a.md"), "# A\n\noriginal\n").unwrap();
        App::new(dir).index().unwrap();
    }
    let retained = client(&root).await;
    let receiving = client(&twin).await;
    let read = ok(&retained, "read", json!({"id":"a.md#a"})).await;
    let citation = read["citation"].clone();
    assert_eq!(
        ok(&receiving, "read", json!({"id":"a.md#a"})).await["citation"],
        citation
    );
    let moved = parent.path().join("moved");
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.md"), "# A\n\nreplacement\n").unwrap();
    App::new(&root).index().unwrap();
    assert_eq!(
        ok(&retained, "verify", json!({"citations":[citation.clone()]})).await["results"][0]["state"],
        "valid"
    );
    fs::remove_file(twin.join("a.md")).unwrap();
    assert_eq!(
        ok(
            &receiving,
            "verify",
            json!({"citations":[citation.clone()]})
        )
        .await["results"][0]["state"],
        "missing"
    );
    // Retained cache and current bytes still use the original root, not its replacement.
    fs::write(moved.join("a.md"), "# A\n\nnew generation\n").unwrap();
    App::new(&moved).index().unwrap();
    let error = call(&retained, "verify", json!({"citations":[citation]}))
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("generation_unavailable"),
        "{error}"
    );
    let next = ok(&retained, "read", json!({"id":"a.md#a"})).await;
    assert!(next["text"].as_str().unwrap().contains("new generation"));
    assert_eq!(
        ok(
            &retained,
            "verify",
            json!({"citations":[next["citation"].clone()]})
        )
        .await["results"][0]["state"],
        "valid"
    );
    retained.cancel().await.unwrap();
    receiving.cancel().await.unwrap();
}

#[tokio::test]
async fn verify_errors_are_typed_and_lower_tiers_keep_full_citations() {
    let dir = tempfile::tempdir().unwrap();
    let missing = client(dir.path()).await;
    let error = call(&missing, "verify", json!({"citations":[]}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no_index"), "{error}");
    missing.cancel().await.unwrap();
    fs::write(
        dir.path().join("a.md"),
        format!(
            "# A\n\nFirst sentence.\n\n{}\n\n## Child\n\nChild sentence.\n",
            "needle body ".repeat(80)
        ),
    )
    .unwrap();
    App::new(dir.path()).index().unwrap();
    let client = client(dir.path()).await;
    let generation = ontodoc::store::Store::new(dir.path().join(".ontodoc"))
        .load()
        .unwrap();
    let chunk = &generation.chunks[0];
    for (budget, tier) in [(chunk.l0.len(), "l0"), (chunk.l1.len(), "l1")] {
        let result = ok(
            &client,
            "search",
            json!({"query":"needle","top_k":1,"budget_bytes":budget}),
        )
        .await;
        let hit = &result["results"][0];
        assert_eq!(hit["tier"], tier);
        assert_eq!(
            ok(
                &client,
                "verify",
                json!({"citations":[hit["citation"].clone()]})
            )
            .await["results"][0]["state"],
            "valid"
        );
    }
    let read = ok(&client, "read", json!({"id":"a.md#a~0"})).await;
    let citation = read["citation"].clone();
    let mut forged = citation.clone();
    forged["byte_end"] = json!(1);
    let error = call(
        &client,
        "verify",
        json!({"citations":[citation.clone(),forged]}),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("invalid_citation"), "{error}");
    let error = call(&client, "verify", json!({"citations":vec![citation;101]}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("cap_exceeded"), "{error}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn invalid_arguments_are_mcp_errors_and_large_results_truncate() {
    let dir = fixture();
    // One large document of many sections so a search would exceed the cap.
    let mut big = String::new();
    for i in 0..60 {
        big.push_str(&format!(
            "# Topic {i}\n\n{}\n\n",
            "needle word ".repeat(400)
        ));
    }
    fs::write(dir.path().join("big.md"), big).unwrap();
    App::new(dir.path()).index().unwrap();
    let client = client(dir.path()).await;

    for (name, args) in [
        ("search", json!({ "query": "x", "top_k": 0 })),
        (
            "search",
            json!({ "query": "x", "mode": "graph", "alpha": 2.0 }),
        ),
        ("search", json!({ "query": "x", "mode": "bogus" })),
        ("search", json!({})),
        ("read", json!({ "id": "nope.md#x" })),
        ("related", json!({ "chunk_id": "nope" })),
        ("verify", json!({ "citations": [{ "path": "a.md" }] })),
    ] {
        let result = call(&client, name, args.clone()).await;
        assert!(result.is_err(), "{name} {args}: {result:?}");
    }

    let search = ok(&client, "search", json!({ "query": "needle", "top_k": 50 })).await;
    assert_eq!(search["truncated"], true);
    let len = serde_json::to_vec(&search).unwrap().len();
    assert!(len <= MAX_RESPONSE_BYTES, "{len}");
    assert!(!search["results"].as_array().unwrap().is_empty());
    client.cancel().await.unwrap();
}

#[test]
fn stdio_server_writes_only_protocol_frames() {
    let dir = fixture();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ontodoc"))
        .arg("--root")
        .arg(dir.path())
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut send = move |v: Value| {
        writeln!(stdin, "{v}").unwrap();
        stdin.flush().unwrap();
    };
    let mut recv = || {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["jsonrpc"], "2.0", "{line}");
        v
    };
    send(
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": { "name": "test", "version": "0" } } }),
    );
    let init = recv();
    assert_eq!(init["result"]["serverInfo"]["name"], "ontodoc");
    send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": "search", "arguments": { "query": "alpha" } } }));
    let reply = recv();
    assert_eq!(reply["id"], 2);
    assert_eq!(reply["result"]["structuredContent"]["mode"], "lexical");
    send(json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }));
    assert_eq!(recv()["result"]["tools"].as_array().unwrap().len(), 5);
    // Dropping the closure closes stdin, which ends the session.
    drop(send);
    let status = child.wait().unwrap();
    assert!(status.success(), "{status}");
    let mut rest = String::new();
    std::io::Read::read_to_string(&mut stdout, &mut rest).unwrap();
    assert!(rest.trim().is_empty(), "unexpected stdout: {rest}");
}

/// Observe the native codec's complete lines, not just structuredContent.
struct WireClient {
    child: std::process::Child,
    input: Option<std::process::ChildStdin>,
    output: BufReader<std::process::ChildStdout>,
}

impl WireClient {
    fn new(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ontodoc"))
            .arg("--root")
            .arg(root)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }

    fn raw(&mut self, line: &str) {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{line}").unwrap();
        input.flush().unwrap();
    }

    fn send(&mut self, value: Value) {
        self.raw(&value.to_string());
    }

    fn recv(&mut self) -> Value {
        let mut bytes = Vec::new();
        assert!(self.output.read_until(b'\n', &mut bytes).unwrap() > 0);
        assert!(
            bytes.len() <= MAX_RESPONSE_BYTES,
            "complete wire frame: {}",
            bytes.len()
        );
        assert_eq!(bytes.last(), Some(&b'\n'));
        serde_json::from_slice(&bytes).unwrap()
    }

    fn init(&mut self) {
        self.send(
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"wire-test","version":"0"}}}),
        );
        assert_eq!(self.recv()["result"]["serverInfo"]["name"], "ontodoc");
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    }

    fn call(&mut self, name: &str, args: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":name,"arguments":args}}));
        self.recv()
    }

    fn finish(mut self, success: bool) -> String {
        self.input.take();
        let mut tail = String::new();
        std::io::Read::read_to_string(&mut self.output, &mut tail).unwrap();
        assert!(tail.is_empty(), "unexpected trailing output: {tail}");
        let status = self.child.wait().unwrap();
        assert_eq!(status.success(), success);
        let mut stderr = String::new();
        std::io::Read::read_to_string(self.child.stderr.as_mut().unwrap(), &mut stderr).unwrap();
        stderr
    }

    fn assert_server_closed(&mut self) {
        // Keep client stdin open: client EOF must not cause the closure.
        assert!(self.input.is_some());
        let mut bytes = Vec::new();
        assert_eq!(self.output.read_until(b'\n', &mut bytes).unwrap(), 0);
    }
}

impl Drop for WireClient {
    fn drop(&mut self) {
        // A failed assertion must not leave a live service process.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn complete_wire_frames_bound_escaped_tools_and_recover_after_errors() {
    let dir = fixture();
    let mut text = String::new();
    for n in 0..60 {
        text.push_str(&format!(
            "# Control {n}\n\n{}\n",
            "needle \"\\\u{1}\t\n".repeat(400)
        ));
    }
    fs::write(dir.path().join("control.md"), text).unwrap();
    App::new(dir.path()).index().unwrap();
    let mut wire = WireClient::new(dir.path());
    wire.init();
    wire.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}));
    assert_eq!(wire.recv()["result"]["tools"].as_array().unwrap().len(), 5);
    for (name, args) in [
        ("read", json!({"id":"control.md#control-0"})),
        ("search", json!({"query":"needle","top_k":50})),
        ("related", json!({"chunk_id":"a.md#alpha~0"})),
        ("verify", json!({"citations":[]})),
        ("status", json!({})),
    ] {
        let frame = wire.call(name, args);
        let result = &frame["result"];
        assert!(result.is_object(), "{name}: {frame}");
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, result["structuredContent"], "text/structured parity");
        if name == "search" {
            assert_eq!(text["truncated"], true);
            assert!(!text["results"].as_array().unwrap().is_empty());
        }
    }
    // A single large section makes escaped read truncation necessary.
    fs::write(
        dir.path().join("large.md"),
        format!("# Large\n\n{}", "\"\\\u{1}é".repeat(20000)),
    )
    .unwrap();
    App::new(dir.path()).index().unwrap();
    let read = wire.call("read", json!({"id":"large.md#large"}));
    assert_eq!(read["result"]["structuredContent"]["truncated"], true);
    assert!(
        !read["result"]["structuredContent"]["text"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    // Oversized decoded fields refuse while raw frames remain admitted.
    for (name, args) in [
        ("read", json!({"id":"\u{1}".repeat(16 * 1024 + 1)})),
        ("search", json!({"query":"\u{1}".repeat(4097)})),
    ] {
        let error = wire.call(name, args);
        assert_eq!(error["error"]["data"]["error"], "cap_exceeded");
    }
    // Invalid enums still use the native bounded argument error path.
    let error = wire.call("search", json!({"query":"x","mode":"bad"}));
    assert_eq!(error["error"]["code"], -32602);
    wire.raw("{broken");
    assert_eq!(
        wire.recv(),
        json!({"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"}})
    );
    wire.send(json!({"jsonrpc":"2.0","id":5,"method":"ping"}));
    assert_eq!(wire.recv()["id"], 5);
    wire.finish(true);
}

#[test]
fn escaped_ids_fit_exact_refusal_or_close_before_dispatch() {
    let dir = fixture();
    let mut wire = WireClient::new(dir.path());
    wire.init();
    let mut expected = json!({"jsonrpc":"2.0","id":"","error":{
        "code":-32603,"message":"Response exceeds the byte limit",
        "data":{"error":"cap_exceeded"}}});
    let available = MAX_RESPONSE_BYTES - (expected.to_string().len() + 1);
    let id = format!(
        "{}{}",
        "\u{1}".repeat(available / 6),
        "a".repeat(available % 6)
    );
    expected["id"] = json!(id);
    assert_eq!(expected.to_string().len() + 1, MAX_RESPONSE_BYTES);
    // A successful tool response must use the same fallback, not just errors.
    wire.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
        "params":{"name":"status","arguments":{}}}));
    assert_eq!(wire.recv(), expected);
    wire.send(json!({"jsonrpc":"2.0","id":9,"method":"ping"}));
    assert_eq!(wire.recv()["id"], 9);
    // A smaller ping can carry an ID whose refusal would exceed the outbound
    // ceiling, while its inbound frame still fits the raw ceiling.
    let ping = json!({"jsonrpc":"2.0","id":format!("{id}a"),"method":"ping"});
    assert!(ping.to_string().len() < MAX_RESPONSE_BYTES);
    wire.send(ping);
    wire.assert_server_closed();
    wire.finish(true);
}

#[test]
fn raw_stdio_admits_exact_frames_and_closes_on_excess_before_decode() {
    let dir = fixture();
    for ending in ["\n", "\r\n"] {
        let mut wire = WireClient::new(dir.path());
        wire.init();
        let ping = json!({"jsonrpc":"2.0","id":7,"method":"ping"}).to_string();
        let raw = format!(
            "{ping}{}{ending}",
            " ".repeat(MAX_RESPONSE_BYTES - ping.len() - ending.len())
        );
        assert_eq!(raw.len(), MAX_RESPONSE_BYTES);
        wire.input
            .as_mut()
            .unwrap()
            .write_all(raw.as_bytes())
            .unwrap();
        wire.input.as_mut().unwrap().flush().unwrap();
        assert_eq!(wire.recv()["id"], 7);
        // Malformed exact-cap input reaches native JSON parsing, not dispatch.
        let malformed = format!(
            "{{{}{}",
            " ".repeat(MAX_RESPONSE_BYTES - 1 - ending.len()),
            ending
        );
        wire.input
            .as_mut()
            .unwrap()
            .write_all(malformed.as_bytes())
            .unwrap();
        wire.input.as_mut().unwrap().flush().unwrap();
        assert_eq!(wire.recv()["error"]["code"], -32700);
        wire.send(json!({"jsonrpc":"2.0","id":8,"method":"ping"}));
        assert_eq!(wire.recv()["id"], 8);
        wire.finish(true);
    }
    for initialized in [false, true] {
        let mut wire = WireClient::new(dir.path());
        if initialized {
            wire.init();
        }
        // No LF and stdin stays open: the extra byte, not client EOF, must
        // close the connection without interpreting or echoing private data.
        let mut raw = b"private-frame-sentinel".to_vec();
        raw.resize(MAX_RESPONSE_BYTES + 1, b'x');
        wire.input.as_mut().unwrap().write_all(&raw).unwrap();
        wire.input.as_mut().unwrap().flush().unwrap();
        wire.assert_server_closed();
        let error = wire.finish(false);
        assert!(!error.contains("private-frame-sentinel"));
        assert!(error.len() < 1024);
    }
    // Exact-cap EOF without LF is still an admitted native frame.
    let mut wire = WireClient::new(dir.path());
    wire.init();
    let ping = json!({"jsonrpc":"2.0","id":9,"method":"ping"}).to_string();
    let raw = format!("{ping}{}", " ".repeat(MAX_RESPONSE_BYTES - ping.len()));
    wire.input
        .as_mut()
        .unwrap()
        .write_all(raw.as_bytes())
        .unwrap();
    wire.input.take();
    assert_eq!(wire.recv()["id"], 9);
    wire.finish(true);
}

#[tokio::test]
async fn argument_limits_precede_missing_index_and_cover_unknown_strings() {
    let root = tempfile::tempdir().unwrap();
    let client = client(root.path()).await;
    for (tool, args) in [
        ("search", json!({"query":"x".repeat(4097)})),
        ("search", json!({"query":"é".repeat(2049)})),
        ("search", json!({"query":"x","scope":"x".repeat(4097)})),
        ("read", json!({"id":"x".repeat(16 * 1024 + 1)})),
        ("related", json!({"chunk_id":"x".repeat(16 * 1024 + 1)})),
        ("verify", json!({"citations":vec![json!({});101]})),
        (
            "status",
            json!({"ignored":{"nested":"x".repeat(64 * 1024)}}),
        ),
        ("status", json!({"x".repeat(64 * 1024 + 1):null})),
    ] {
        let error = call(&client, tool, args).await.unwrap_err();
        assert!(
            error.to_string().contains("cap_exceeded"),
            "{tool}: {error}"
        );
        assert!(
            !error
                .to_string()
                .contains(&root.path().display().to_string())
        );
    }
    // Aggregate includes the unknown key; equality is accepted, +1 refused.
    let error = call(&client, "status", json!({"x":"x".repeat(64 * 1024 - 1)}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no_index"), "{error}");
    let citation = json!({"generation_id":"0".repeat(64),"path":"a.md","heading_path":[],
        "byte_start":0,"byte_end":1,"line_start":1,"line_end":1,"sha256":"0".repeat(64)});
    for (field, value, kind) in [
        ("path", json!("x".repeat(4097)), "cap_exceeded"),
        ("heading_path", json!(vec![""; 65]), "cap_exceeded"),
        (
            "heading_path",
            json!(["x".repeat(16 * 1024 + 1)]),
            "cap_exceeded",
        ),
        ("generation_id", json!("A".repeat(64)), "invalid_argument"),
        ("sha256", json!("0".repeat(63)), "invalid_argument"),
    ] {
        let mut bad = citation.clone();
        bad[field] = value;
        let error = call(&client, "verify", json!({"citations":[bad]}))
            .await
            .unwrap_err();
        assert!(error.to_string().contains(kind), "{error}");
    }
    client.cancel().await.unwrap();
}

#[test]
fn internal_errors_do_not_expose_root_or_os_details() {
    let dir = tempfile::Builder::new()
        .prefix("private-root-sentinel")
        .tempdir()
        .unwrap();
    let mut wire = WireClient::new(dir.path());
    wire.init();
    let error = wire.call("status", json!({}));
    assert_eq!(error["error"]["data"]["error"], "no_index");
    assert!(!error.to_string().contains("private-root-sentinel"));
    fs::create_dir(dir.path().join(".ontodoc")).unwrap();
    // Reading a directory as CURRENT produces a raw OS/path-bearing store error.
    fs::create_dir(dir.path().join(".ontodoc/CURRENT")).unwrap();
    let error = wire.call("status", json!({}));
    assert_eq!(error["error"]["data"]["error"], "io");
    assert_eq!(error["error"]["message"], "Internal operation failed");
    assert!(!error.to_string().contains("private-root-sentinel"));
    wire.finish(true);
    let mut startup = WireClient::new(dir.path());
    startup.send(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"private-startup-sentinel","arguments":{}}}));
    let error = startup.finish(false);
    assert!(error.contains("MCP initialization failed"));
    assert!(!error.contains("private-startup-sentinel"));
}

/// Folder registry: each immediate subdirectory is an independent corpus with
/// its own store; identical local ids never cross, an unindexed folder
/// reports `no_index` instead of falling back, and names are exact.
#[tokio::test]
async fn docs_share_folders_are_independent_corpora() {
    use ontodoc::mcp::{Registry, RegistryError};
    let share = tempfile::tempdir().unwrap();
    for (name, body) in [
        ("alpha", "# Guide\n\nalpha needle text\n"),
        ("beta", "# Guide\n\nbeta needle text\n"),
    ] {
        let root = share.path().join(name);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("guide.md"), body).unwrap();
        App::new(&root).index().unwrap();
    }
    fs::create_dir(share.path().join("empty")).unwrap();
    fs::create_dir(share.path().join("has space")).unwrap();
    fs::create_dir(share.path().join(".git")).unwrap();
    fs::write(share.path().join("README.md"), "not a corpus").unwrap();
    fs::write(share.path().join("release notes?.txt"), "plain file").unwrap();

    let registry = Registry::discover(share.path()).unwrap();
    // Names are kept verbatim (no slugging); metadata dirs and files skipped.
    assert_eq!(
        registry.names().collect::<Vec<_>>(),
        ["alpha", "beta", "empty", "has space"]
    );
    assert!(registry.get("Alpha").is_none());
    assert!(registry.get("has%20space").is_none());
    assert!(registry.get(".git").is_none());

    let mut seen = Vec::new();
    let mut citations = Vec::new();
    for name in ["alpha", "beta"] {
        let (server_io, client_io) = tokio::io::duplex(1 << 20);
        let server = registry.get(name).unwrap().clone();
        tokio::spawn(async move {
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        let client = ().serve(client_io).await.unwrap();
        let search = ok(&client, "search", json!({ "query": "needle" })).await;
        let first = &search["results"][0];
        // Same local chunk id in both corpora, different text and generation.
        assert_eq!(first["chunk_id"], "guide.md#guide~0");
        assert!(first["text"].as_str().unwrap().contains(name), "{first}");
        let read = ok(&client, "read", json!({ "id": "guide.md#guide~0" })).await;
        assert!(read["text"].as_str().unwrap().contains(name), "{read}");
        ok(
            &client,
            "related",
            json!({ "chunk_id": "guide.md#guide~0" }),
        )
        .await;
        let status = ok(&client, "status", json!({})).await;
        assert_eq!(status["generation_id"], search["generation_id"]);
        let own = first["citation"].clone();
        let verify = ok(&client, "verify", json!({ "citations": [own.clone()] })).await;
        assert_eq!(verify["results"][0]["state"], "valid");
        // The sibling corpus's citation never verifies here.
        if let Some(other) = citations.last() {
            let crossed = call(&client, "verify", json!({ "citations": [other] })).await;
            let refused = match crossed {
                Err(_) => true,
                Ok(r) => {
                    r.is_error == Some(true)
                        || r.structured_content.unwrap()["results"][0]["state"] != "valid"
                }
            };
            assert!(refused, "cross-corpus citation verified in {name}");
        }
        citations.push(own);
        seen.push(search["generation_id"].clone());
        client.cancel().await.unwrap();
    }
    assert_ne!(seen[0], seen[1]);

    let (server_io, client_io) = tokio::io::duplex(1 << 20);
    let server = registry.get("empty").unwrap().clone();
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let client = ().serve(client_io).await.unwrap();
    let error = call(&client, "search", json!({ "query": "needle" }))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no_index"), "{error}");
    client.cancel().await.unwrap();

    // Names that cannot be one lossless segment, and symlinks (including
    // hidden ones), refuse startup explicitly.
    let bad = tempfile::tempdir().unwrap();
    fs::create_dir(bad.path().join("tab\tname")).unwrap();
    assert!(matches!(
        Registry::discover(bad.path()),
        Err(RegistryError::InvalidName(_))
    ));
    let link = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(share.path().join("alpha"), link.path().join("alias")).unwrap();
    assert!(matches!(
        Registry::discover(link.path()),
        Err(RegistryError::NotDirectory(_))
    ));
    let hidden_link = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        share.path().join("alpha"),
        hidden_link.path().join(".alias"),
    )
    .unwrap();
    assert!(matches!(
        Registry::discover(hidden_link.path()),
        Err(RegistryError::NotDirectory(_))
    ));
    assert!(matches!(
        Registry::discover(&share.path().join("missing")),
        Err(RegistryError::Unavailable(_))
    ));
    let many = tempfile::tempdir().unwrap();
    for i in 0..=ontodoc::mcp::MAX_CORPORA {
        fs::create_dir(many.path().join(format!("c{i:02}"))).unwrap();
    }
    assert!(matches!(
        Registry::discover(many.path()),
        Err(RegistryError::TooMany)
    ));
    let files = tempfile::tempdir().unwrap();
    for i in 0..=ontodoc::mcp::MAX_ENTRIES {
        fs::write(files.path().join(format!("f{i}")), "").unwrap();
    }
    assert!(matches!(
        Registry::discover(files.path()),
        Err(RegistryError::TooManyEntries)
    ));
}
