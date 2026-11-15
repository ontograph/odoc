//! HTTP-2 listener contracts over a real loopback socket (port 0): folder
//! routing and isolation, stdio parity, Origin/Host/method/size refusals,
//! fail-fast admission and drain. No production listener is exposed.

use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use ontodoc::cli::{App, MAX_RESPONSE_BYTES};
use ontodoc::http::{MAX_REQUEST_BYTES, Router, encode_segment, run};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

/// Upper bound for any single socket exchange in these tests.
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

fn share() -> tempfile::TempDir {
    let share = tempfile::tempdir().unwrap();
    for (name, body) in [
        ("alpha", "# Guide\n\nalpha needle text\n"),
        ("beta", "# Guide\n\nbeta needle text\n"),
        ("has space", "# Guide\n\nspaced needle text\n"),
    ] {
        let root = share.path().join(name);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("guide.md"), body).unwrap();
        App::new(&root).index().unwrap();
    }
    fs::create_dir(share.path().join("empty")).unwrap();
    share
}

struct Server {
    addr: SocketAddr,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}

async fn start(share: &std::path::Path) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = Arc::new(Router::new(share, &addr.to_string()).unwrap());
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(run(listener, router, shutdown.clone()));
    Server {
        addr,
        shutdown,
        task,
    }
}

/// One raw HTTP/1.1 request; returns status and body text.
async fn send(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Vec<u8>,
) -> (u16, String) {
    let stream = TcpStream::connect(addr).await.unwrap();
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .unwrap();
    tokio::spawn(conn);
    let host = addr.to_string();
    let mut all = vec![
        ("host", host.as_str()),
        ("accept", "application/json, text/event-stream"),
        ("content-type", "application/json"),
    ];
    for (k, v) in headers {
        all.retain(|(name, _)| name != k);
        all.push((k, v));
    }
    let mut request = http::Request::builder().method(method).uri(path);
    for (k, v) in all {
        request = request.header(k, v);
    }
    let response = sender
        .send_request(request.body(Full::new(Bytes::from(body))).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn rpc(addr: SocketAddr, path: &str, method: &str, params: Value) -> Value {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let (status, text) = send(addr, "POST", path, &[], body.to_string().into_bytes()).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn tool(addr: SocketAddr, path: &str, name: &str, args: Value) -> Value {
    let reply = rpc(
        addr,
        path,
        "tools/call",
        json!({ "name": name, "arguments": args }),
    )
    .await;
    reply["result"]["structuredContent"].clone()
}

#[tokio::test]
async fn folders_are_isolated_endpoints_with_stdio_parity() {
    let share = share();
    let server = start(share.path()).await;
    let a = server.addr;

    let init = rpc(
        a,
        "/alpha/mcp",
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}),
    )
    .await;
    assert_eq!(init["result"]["serverInfo"]["name"], "ontodoc");
    let tools = rpc(a, "/alpha/mcp", "tools/list", json!({})).await;
    let mut names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["read", "related", "search", "status", "verify"]);

    // Same local id, different corpus text: no cross-folder fallback.
    for (path, word) in [
        ("/alpha/mcp", "alpha"),
        ("/beta/mcp", "beta"),
        (&format!("/{}/mcp", encode_segment("has space")), "spaced"),
    ] {
        let read = tool(a, path, "read", json!({ "id": "guide.md#guide~0" })).await;
        assert!(read["text"].as_str().unwrap().contains(word), "{read}");
    }
    assert_eq!(encode_segment("has space"), "has%20space");

    // Stdio parity: the same corpus answers identically through App.
    let root = share.path().join("beta");
    let app = App::new(&root);
    let index = app.load_index().unwrap();
    let request = serde_json::from_value(json!({ "query": "needle" })).unwrap();
    let local = serde_json::to_value(app.search(&index, &request).unwrap()).unwrap();
    let remote = tool(a, "/beta/mcp", "search", json!({ "query": "needle" })).await;
    assert_eq!(remote, local);

    // An empty folder has an endpoint and reports no index, never a sibling's.
    let empty = rpc(
        a,
        "/empty/mcp",
        "tools/call",
        json!({ "name": "status", "arguments": {} }),
    )
    .await;
    assert_eq!(empty["error"]["data"]["error"], "no_index", "{empty}");

    server.shutdown.cancel();
    server.task.await.unwrap();
}

#[tokio::test]
async fn complete_http_responses_match_stdio_for_large_and_escaped_ids() {
    let share = share();
    let root = share.path().join("empty");
    fs::write(
        root.join("large.md"),
        format!("# Large\n\n```text\n{}\n```\n", "needle ".repeat(12000)),
    )
    .unwrap();
    fs::write(root.join("small.md"), "# Small\n\nneedle supporting text\n").unwrap();
    App::new(&root).index().unwrap();
    let server = start(share.path()).await;
    let cap = json!({"jsonrpc":"2.0","id":"","error":{
        "code":-32603,"message":"Response exceeds the byte limit",
        "data":{"error":"cap_exceeded"}}});
    let room = MAX_RESPONSE_BYTES - 1 - cap.to_string().len();
    let escaped = format!("{}{}", "\u{1}".repeat(room / 6), "a".repeat(room % 6));
    let requests = vec![
        json!({"jsonrpc":"2.0","id":7,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":"ordinary","method":"tools/call",
            "params":{"name":"status","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":"a".repeat(room),"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":escaped,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":escaped,"method":"tools/call",
            "params":{"name":"status","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":"search","method":"tools/call",
            "params":{"name":"search","arguments":{"query":"needle"}}}),
        json!({"jsonrpc":"2.0","id":escaped,"method":"tools/call",
            "params":{"name":"search","arguments":{"query":"needle"}}}),
    ];
    // Exercise the actual stdio boundary, including its reserved newline.
    let native_requests = requests.clone();
    let native = tokio::task::spawn_blocking(move || {
        use std::io::{BufRead, BufReader, Write};
        use std::process::{Command, Stdio};
        let mut child = Command::new(env!("CARGO_BIN_EXE_ontodoc"))
            .arg("--root")
            .arg(root)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"http-parity","version":"0"}}});
        writeln!(input, "{init}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        assert!(output.read_line(&mut line).unwrap() > 0);
        assert!(serde_json::from_str::<Value>(&line).unwrap()["result"].is_object());
        writeln!(
            input,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        let mut replies = Vec::new();
        for request in native_requests {
            writeln!(input, "{request}").unwrap();
            input.flush().unwrap();
            let mut line = String::new();
            assert!(output.read_line(&mut line).unwrap() > 0);
            assert!(line.len() <= MAX_RESPONSE_BYTES);
            replies.push(serde_json::from_str::<Value>(&line).unwrap());
        }
        drop(input);
        assert!(child.wait().unwrap().success());
        replies
    })
    .await
    .unwrap();
    for (index, (request, expected)) in requests.iter().zip(native).enumerate() {
        let body = request.to_string().into_bytes();
        assert!(body.len() <= MAX_REQUEST_BYTES);
        let (status, text) = send(server.addr, "POST", "/empty/mcp", &[], body).await;
        assert_eq!(status, 200);
        assert!(text.len() < MAX_RESPONSE_BYTES);
        let response: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response, expected);
        assert_eq!(response["id"], request["id"]);
        if (2..=4).contains(&index) || index == 6 {
            let mut expected_cap = cap.clone();
            expected_cap["id"] = request["id"].clone();
            assert_eq!(response, expected_cap);
            assert_eq!(text.len() + 1, MAX_RESPONSE_BYTES);
        }
        if index == 5 {
            let search = &response["result"]["structuredContent"];
            assert_eq!(search["truncated"], false);
            let hits = search["results"].as_array().unwrap();
            assert_eq!(hits.len(), 2);
            assert_eq!(hits[0]["tier"], Value::Null);
            assert_eq!(hits[1]["tier"], "l2");
        }
    }
    server.shutdown.cancel();
    server.task.await.unwrap();
}

#[tokio::test]
async fn cited_related_overflow_truncates_identically_over_stdio_and_http() {
    let share = share();
    let root = share.path().join("empty");
    let mut links = String::from("# Source\n\n");
    // Punctuation keeps anchors short while citations retain the full heading.
    // All 49 neighbours fit the item cap; their citations exceed the byte cap.
    for i in 0..48 {
        let path = format!("t{i:02}.md");
        fs::write(
            root.join(&path),
            format!("# Target{i}{}\n\nbody\n", "!".repeat(3000)),
        )
        .unwrap();
        links.push_str(&format!("[t]({path}) "));
    }
    fs::write(root.join("source.md"), links).unwrap();
    App::new(&root).index().unwrap();
    let generation = ontodoc::store::Store::new(root.join(".ontodoc"))
        .load()
        .unwrap();
    let citations: Vec<_> = generation
        .chunks
        .iter()
        .enumerate()
        .filter(|(_, chunk)| chunk.path != "source.md" && chunk.ordinal == 0)
        .map(|(index, _)| ontodoc::cite::Citation::for_chunk(&generation, index).unwrap())
        .collect();
    assert_eq!(citations.len(), 48);
    assert!(serde_json::to_vec(&citations).unwrap().len() > MAX_RESPONSE_BYTES);
    let server = start(share.path()).await;
    let request = json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"related","arguments":{"chunk_id":"source.md#source~0"}}});
    let native_request = request.clone();
    let native = tokio::task::spawn_blocking(move || {
        use std::io::{BufRead, BufReader, Write};
        use std::process::{Command, Stdio};
        let mut child = Command::new(env!("CARGO_BIN_EXE_ontodoc"))
            .arg("--root")
            .arg(root)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"related-parity","version":"0"}}});
        writeln!(input, "{init}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        assert!(output.read_line(&mut line).unwrap() > 0);
        assert!(serde_json::from_str::<Value>(&line).unwrap()["result"].is_object());
        writeln!(
            input,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        writeln!(input, "{native_request}").unwrap();
        input.flush().unwrap();
        line.clear();
        assert!(output.read_line(&mut line).unwrap() > 0);
        assert!(line.ends_with('\n'));
        assert!(line.len() <= MAX_RESPONSE_BYTES);
        let reply: Value = serde_json::from_str(&line).unwrap();
        drop(input);
        assert!(child.wait().unwrap().success());
        reply
    })
    .await
    .unwrap();
    let body = request.to_string().into_bytes();
    assert!(body.len() <= MAX_REQUEST_BYTES);
    let (status, text) = send(server.addr, "POST", "/empty/mcp", &[], body).await;
    assert_eq!(status, 200);
    assert!(text.len() < MAX_RESPONSE_BYTES);
    let remote: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(remote, native);
    assert_eq!(remote["id"], request["id"]);
    assert_ne!(remote["result"]["isError"], true);
    let related = &remote["result"]["structuredContent"];
    assert_eq!(related["truncated"], true);
    let neighbours = related["neighbours"].as_array().unwrap();
    assert!(!neighbours.is_empty());
    assert!(neighbours.len() < 49);
    assert_eq!(
        neighbours.len() + related["omitted"].as_u64().unwrap() as usize,
        49
    );
    for neighbour in neighbours {
        let citation = neighbour.get("citation").unwrap();
        let verified = tool(
            server.addr,
            "/empty/mcp",
            "verify",
            json!({"citations":[citation]}),
        )
        .await;
        assert_eq!(verified["results"][0]["state"], "valid");
    }
    server.shutdown.cancel();
    server.task.await.unwrap();
}

#[tokio::test]
async fn malformed_native_ids_keep_rmcp_handling() {
    let share = share();
    let server = start(share.path()).await;
    for body in [
        r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":{},"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":1,"id":2,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":{},"method":{}}"#,
        "{broken",
    ] {
        // rmcp may classify an invalid request as a custom notification.
        // Preserve that native classification instead of inventing ID rules.
        let expected = match serde_json::from_reader::<_, rmcp::model::ClientJsonRpcMessage>(
            body.as_bytes(),
        ) {
            Ok(rmcp::model::ClientJsonRpcMessage::Request(_)) => {
                panic!("invalid ID became a native request")
            }
            Ok(_) => 202,
            Err(_) => 415,
        };
        let (status, text) = send(
            server.addr,
            "POST",
            "/empty/mcp",
            &[],
            body.as_bytes().to_vec(),
        )
        .await;
        assert_eq!(status, expected);
        assert_eq!(
            text,
            if expected == 415 {
                "request refused"
            } else {
                ""
            }
        );
    }
    server.shutdown.cancel();
    server.task.await.unwrap();
}

#[tokio::test]
async fn refusals_precede_routing_and_body_reads() {
    let share = share();
    let server = start(share.path()).await;
    let a = server.addr;
    let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} })
        .to_string()
        .into_bytes();

    // Unknown, aliased or traversal paths: 404, no default corpus.
    for path in [
        "/Alpha/mcp",
        "/has space/mcp".replace(' ', "+").as_str(),
        "/../alpha/mcp",
        "/alpha%2Fbeta/mcp",
        "/mcp",
        "/alpha/mcp/extra",
    ] {
        let (status, _) = send(a, "POST", path, &[], list.clone()).await;
        assert_eq!(status, 404, "{path}");
    }
    // Browser Origin refused before routing.
    let (status, _) = send(
        a,
        "POST",
        "/alpha/mcp",
        &[("origin", "http://evil.example")],
        list.clone(),
    )
    .await;
    assert_eq!(status, 403);
    // DNS-rebinding Host refused by rmcp's allowlist.
    let (status, _) = send(
        a,
        "POST",
        "/alpha/mcp",
        &[("host", "evil.example")],
        list.clone(),
    )
    .await;
    assert_eq!(status, 403);
    // No sessions or streams: GET and DELETE are not served.
    for method in ["GET", "DELETE"] {
        let (status, _) = send(a, method, "/alpha/mcp", &[], Vec::new()).await;
        assert_eq!(status, 405, "{method}");
    }
    // Request body: exactly the limit is decoded (and rejected as JSON);
    // one byte more is 413 before decoding.
    let pad = |n: usize| {
        let mut body = b"{\"x\":\"".to_vec();
        body.resize(n - 2, b'a');
        body.extend_from_slice(b"\"}");
        body
    };
    let (status, text) = send(a, "POST", "/alpha/mcp", &[], pad(MAX_REQUEST_BYTES)).await;
    assert_ne!(status, 413);
    // rmcp parser and header errors keep their status but not their details.
    assert_eq!(text, "request refused");
    let (status, text) = send(
        a,
        "POST",
        "/alpha/mcp",
        &[("mcp-protocol-version", "secret-probe-text")],
        list.clone(),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(text, "request refused");
    let (status, _) = send(a, "POST", "/alpha/mcp", &[], pad(MAX_REQUEST_BYTES + 1)).await;
    assert_eq!(status, 413);

    server.shutdown.cancel();
    server.task.await.unwrap();
}

#[tokio::test]
async fn only_loopback_binds_and_shutdown_drains_then_stops_accepting() {
    let share = share();
    let public: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let err = ontodoc::http::serve(share.path(), public, CancellationToken::new())
        .await
        .unwrap_err();
    assert!(matches!(err, ontodoc::http::HttpError::NotLoopback));
    // Missing docs_share is an explicit configuration error.
    let missing = share.path().join("missing");
    assert!(Router::new(&missing, "127.0.0.1:1").is_err());

    let server = start(share.path()).await;
    let a = server.addr;
    let before = tool(a, "/alpha/mcp", "status", json!({})).await;
    assert!(before["generation_id"].is_string(), "{before}");
    server.shutdown.cancel();
    server.task.await.unwrap();
    // After the drain the listener is gone.
    assert!(TcpStream::connect(a).await.is_err());
}

#[tokio::test]
async fn idle_connections_count_against_admission() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let share = share();
    let server = start(share.path()).await;
    let a = server.addr;
    let mut idle = Vec::new();
    for _ in 0..ontodoc::http::MAX_IN_FLIGHT {
        idle.push(TcpStream::connect(a).await.unwrap());
    }
    // A probe with the same request on each side of the cap.
    let probe = format!("POST /zzz/mcp HTTP/1.1\r\nhost: {a}\r\ncontent-length: 0\r\n\r\n");
    let mut over = TcpStream::connect(a).await.unwrap();
    let _ = over.write_all(probe.as_bytes()).await;
    let mut buf = Vec::new();
    let read = tokio::time::timeout(DEADLINE, over.read_to_end(&mut buf))
        .await
        .expect("excess connection neither closed nor reset");
    assert!(read.is_err() || buf.is_empty(), "excess connection served");
    // Releasing one idle socket admits a new connection again (the server
    // notices the close asynchronously, so retry a bounded number of times).
    drop(idle.pop());
    let mut served = String::new();
    for _ in 0..100 {
        let mut retry = TcpStream::connect(a).await.unwrap();
        let _ = retry.write_all(probe.as_bytes()).await;
        // Accumulate until the header terminator; a status line may arrive
        // split across reads.
        let mut buf = Vec::new();
        let _ = tokio::time::timeout(DEADLINE, async {
            let mut chunk = [0u8; 256];
            while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 8192 {
                match retry.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
        })
        .await;
        served = String::from_utf8_lossy(&buf).into_owned();
        if !served.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(served.starts_with("HTTP/1.1 404"), "{served}");
    drop(idle);
    server.shutdown.cancel();
    server.task.await.unwrap();
}
