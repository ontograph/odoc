//! MCP server: five read-only tools over the shared application layer.
//!
//! The index is loaded once per server and reloaded when `CURRENT` changes.
//! Every tool result is structured JSON from [`crate::cli`], bounded by
//! `MAX_RESPONSE_BYTES` with `truncated` flags. Indexing is CLI-only.

use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::JsonRpcMessage;
use rmcp::model::{CallToolResult, ErrorData, Implementation, ServerCapabilities, ServerInfo};
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::Transport;
use rmcp::{RoleServer, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::Semaphore;

use crate::cite::Citation;
use crate::cli::{
    App, AppError, Bounded, MAX_ARGUMENT_BYTES, MAX_ID_BYTES, MAX_LIST_ITEMS, MAX_RESPONSE_BYTES,
    SearchRequest, check_input_limit, validate_citation_fields, validate_citation_json,
    validate_json_strings,
};
use crate::search::SearchIndex;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadRequest {
    /// Chunk id (`path#slug/path~0`), section id (`path#slug/path`) or `path#anchor`.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RelatedRequest {
    /// Chunk id whose graph neighbours to list.
    pub chunk_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyRequest {
    /// Citations exactly as returned by `search` or `read` (at most 100).
    pub citations: Vec<CitationInput>,
}

/// A citation as returned by `search` or `read`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CitationInput {
    pub generation_id: String,
    /// Repository-relative path.
    pub path: String,
    pub heading_path: Vec<String>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    /// SHA-256 of the cited bytes.
    pub sha256: String,
}

impl From<CitationInput> for Citation {
    fn from(c: CitationInput) -> Self {
        Self {
            generation_id: c.generation_id,
            path: c.path,
            heading_path: c.heading_path,
            byte_start: c.byte_start,
            byte_end: c.byte_end,
            line_start: c.line_start,
            line_end: c.line_end,
            sha256: c.sha256,
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct StatusRequest {}

/// Loaded index keyed by generation id.
type IndexSlot = Arc<Mutex<Option<(String, Arc<SearchIndex>)>>>;

#[derive(Clone)]
pub struct OntoDocServer {
    app: Arc<App>,
    index: IndexSlot,
    admission: Arc<Semaphore>,
    /// Process-wide work permits shared by every corpus (HTTP); `None` for stdio.
    global: Option<Arc<Semaphore>>,
    #[cfg(test)]
    work_counts: Arc<WorkCounts>,
    tool_router: ToolRouter<Self>,
}

#[cfg(test)]
#[derive(Default)]
struct WorkCounts {
    spawns: std::sync::atomic::AtomicUsize,
    current_reads: std::sync::atomic::AtomicUsize,
    loads: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl WorkCounts {
    fn snapshot(&self) -> [usize; 3] {
        [&self.spawns, &self.current_reads, &self.loads].map(|count| count.load(Ordering::Relaxed))
    }
}

const MAX_INPUT_BYTES: usize = 128 * 1024;

/// Bounds the actual native line collector before parsing or allocation.
/// Buffered bytes survive cancelled reads, and only one line is forwarded per poll.
struct BoundedInput<R> {
    inner: R,
    pending: [u8; 8192],
    start: usize,
    end: usize,
    line_bytes: usize,
    limit: usize,
    refused: Arc<AtomicBool>,
}

impl<R> BoundedInput<R> {
    fn new(inner: R, limit: usize, refused: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            pending: [0; 8192],
            start: 0,
            end: 0,
            line_bytes: 0,
            limit,
            refused,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BoundedInput<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if out.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let cap_error = || std::io::Error::other("MCP input exceeds the byte limit");
        if this.refused.load(Ordering::Relaxed) {
            return Poll::Ready(Err(cap_error()));
        }
        if this.start == this.end {
            let mut scratch = [0; 8192];
            let allowance = this.limit - this.line_bytes;
            let count = out.remaining().min(scratch.len()).min(allowance.max(1));
            let mut input = ReadBuf::new(&mut scratch[..count]);
            match Pin::new(&mut this.inner).poll_read(cx, &mut input) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(())) => {}
            }
            let bytes = input.filled();
            if bytes.is_empty() {
                return Poll::Ready(Ok(()));
            }
            if allowance == 0 {
                this.refused.store(true, Ordering::Relaxed);
                return Poll::Ready(Err(cap_error()));
            }
            this.pending[..bytes.len()].copy_from_slice(bytes);
            this.start = 0;
            this.end = bytes.len();
        }
        let pending = &this.pending[this.start..this.end];
        let count = pending
            .iter()
            .position(|b| *b == b'\n')
            .map_or(pending.len(), |i| i + 1)
            .min(out.remaining());
        let bytes = &pending[..count];
        this.line_bytes += count;
        if bytes.last() == Some(&b'\n') {
            this.line_bytes = 0;
        }
        out.put_slice(bytes);
        this.start += count;
        Poll::Ready(Ok(()))
    }
}

fn to_mcp(e: AppError) -> ErrorData {
    let data = Some(serde_json::json!({ "error": e.kind() }));
    match e {
        AppError::InvalidArgument(m) => ErrorData::invalid_params(m, data),
        AppError::NotFound(m) => ErrorData::resource_not_found(m, data),
        e => ErrorData::internal_error(e.public_message(), data),
    }
}

fn validate_tool_arguments(request: &rmcp::model::CallToolRequestParams) -> Result<(), AppError> {
    let mut bytes = 0usize;
    if let Some(arguments) = &request.arguments {
        for (key, value) in arguments {
            bytes = bytes
                .checked_add(key.len())
                .ok_or(crate::store::StoreError::CapExceeded {
                    what: "arguments",
                    cap: MAX_ARGUMENT_BYTES,
                })?;
            check_input_limit(bytes, MAX_ARGUMENT_BYTES, "arguments")?;
            validate_json_strings(value, &mut bytes)?;
        }
        let field_limit = |field, cap| {
            if let Some(value) = arguments.get(field).and_then(serde_json::Value::as_str) {
                check_input_limit(value.len(), cap, "argument field bytes")?;
            }
            Ok::<_, AppError>(())
        };
        match request.name.as_ref() {
            "search" => {
                field_limit("query", crate::cli::MAX_QUERY_BYTES)?;
                field_limit("scope", crate::cli::MAX_QUERY_BYTES)?;
            }
            "read" => field_limit("id", MAX_ID_BYTES)?,
            "related" => field_limit("chunk_id", MAX_ID_BYTES)?,
            "verify" => {
                if let Some(citations) = arguments
                    .get("citations")
                    .and_then(serde_json::Value::as_array)
                {
                    check_input_limit(citations.len(), MAX_LIST_ITEMS, "citations")?;
                    for citation in citations {
                        validate_citation_json(citation)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn structured<T: Bounded>(value: Result<T, AppError>) -> Result<CallToolResult, ErrorData> {
    let mut value = value.map_err(to_mcp)?;
    loop {
        let json = serde_json::to_value(&value).map_err(|e| to_mcp(e.into()))?;
        let result = CallToolResult::structured(json);
        // Preserve rmcp's text/structured parity. The transport checks the final
        // envelope and ID as well; a near-boundary result may still be refused.
        if fits_json(&result, MAX_RESPONSE_BYTES) {
            return Ok(result);
        }
        if !value.pop_item() {
            return Err(to_mcp(AppError::ResponseTooLarge));
        }
        value.mark_truncated();
    }
}

/// Counts the native serde encoding without allocating an oversized byte buffer.
fn fits_json(value: &impl Serialize, limit: usize) -> bool {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.0 {
                return Err(std::io::ErrorKind::FileTooLarge.into());
            }
            self.0 -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(limit), value).is_ok()
}

fn cap_error(id: Option<rmcp::model::RequestId>) -> TxJsonRpcMessage<RoleServer> {
    JsonRpcMessage::error(
        ErrorData::internal_error(
            "Response exceeds the byte limit",
            Some(serde_json::json!({ "error": "cap_exceeded" })),
        ),
        id,
    )
}

/// Final stdio boundary; rmcp still owns framing and protocol handling.
struct BoundedTransport<T> {
    inner: T,
    closed: bool,
}

impl<T: Transport<RoleServer, Error = std::io::Error>> Transport<RoleServer>
    for BoundedTransport<T>
{
    type Error = std::io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
        let item = if fits_json(&item, MAX_RESPONSE_BYTES - 1) {
            Some(item)
        } else {
            match item {
                JsonRpcMessage::Response(r) => Some(cap_error(Some(r.id))),
                JsonRpcMessage::Error(e) => Some(cap_error(e.id)),
                _ => None,
            }
        };
        let send = item
            .filter(|v| fits_json(v, MAX_RESPONSE_BYTES - 1))
            .map(|v| self.inner.send(v));
        async move {
            match send {
                Some(send) => send.await,
                None => Err(std::io::Error::other("MCP frame exceeds the byte limit")),
            }
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
        if self.closed {
            return None;
        }
        let item = self.inner.receive().await?;
        if let JsonRpcMessage::Request(r) = &item
            && !fits_json(&cap_error(Some(r.id.clone())), MAX_RESPONSE_BYTES - 1)
        {
            // Persist rejection before awaiting close (receive may be cancelled).
            self.closed = true;
            let _ = self.inner.close().await;
            return None;
        }
        Some(item)
    }

    async fn close(&mut self) -> std::io::Result<()> {
        self.closed = true;
        self.inner.close().await
    }
}

/// Upper bound on concurrently admitted tool calls per server.
pub const MAX_PERMITS: usize = 64;

impl OntoDocServer {
    /// One admitted call at a time (the stdio default).
    pub fn new(root: &Path) -> Self {
        Self::build(root, 1)
    }

    /// Admits up to `permits` concurrent calls; `1..=MAX_PERMITS`, else `None`.
    /// Calls beyond the permits fail fast with `busy`; there is no queue.
    pub fn with_permits(root: &Path, permits: usize) -> Option<Self> {
        (1..=MAX_PERMITS)
            .contains(&permits)
            .then(|| Self::build(root, permits))
    }

    fn build(root: &Path, permits: usize) -> Self {
        Self {
            app: Arc::new(App::new(root)),
            index: Arc::new(Mutex::new(None)),
            admission: Arc::new(Semaphore::new(permits)),
            global: None,
            #[cfg(test)]
            work_counts: Arc::new(WorkCounts::default()),
            tool_router: Self::tool_router(),
        }
    }

    /// Admission stays owned by the actual blocking work, including CURRENT
    /// reads, reloads, freshness and result serialization. The index mutex
    /// guards only the CURRENT comparison, reload and snapshot; tools then run
    /// on the immutable `Arc` snapshot so admitted calls can overlap.
    async fn blocking<T, F>(&self, f: F) -> Result<CallToolResult, ErrorData>
    where
        T: Bounded + Send + 'static,
        F: FnOnce(&App, &SearchIndex) -> Result<T, AppError> + Send + 'static,
    {
        // Global first, then per corpus; both fail fast and move into the work.
        let global = match &self.global {
            Some(g) => Some(
                g.clone()
                    .try_acquire_owned()
                    .map_err(|_| to_mcp(AppError::Busy))?,
            ),
            None => None,
        };
        let permit = self
            .admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| to_mcp(AppError::Busy))?;
        let app = self.app.clone();
        let slot = self.index.clone();
        #[cfg(test)]
        let counts = self.work_counts.clone();
        #[cfg(test)]
        counts.spawns.fetch_add(1, Ordering::Relaxed);
        tokio::task::spawn_blocking(move || {
            let _permits = (global, permit);
            let index = {
                let mut slot = slot.lock().map_err(|_| to_mcp(AppError::Internal))?;
                #[cfg(test)]
                counts.current_reads.fetch_add(1, Ordering::Relaxed);
                let current = app.current_id().map_err(to_mcp)?;
                if slot.as_ref().is_none_or(|(id, _)| *id != current) {
                    #[cfg(test)]
                    counts.loads.fetch_add(1, Ordering::Relaxed);
                    let index = Arc::new(app.load_index_for(&current).map_err(to_mcp)?);
                    *slot = Some((current, index));
                }
                slot.as_ref()
                    .ok_or_else(|| to_mcp(AppError::Internal))?
                    .1
                    .clone()
            };
            structured(f(&app, &index))
        })
        .await
        .map_err(|_| to_mcp(AppError::Internal))?
    }
}

#[tool_router]
impl OntoDocServer {
    /// Search documentation. Returns ranked chunks with citations, freshness and
    /// an abstention reason when nothing matches well enough.
    #[tool(annotations(
        title = "Search documentation",
        read_only_hint = true,
        open_world_hint = false
    ))]
    async fn search(
        &self,
        Parameters(req): Parameters<SearchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        req.validate().map_err(to_mcp)?;
        self.blocking(move |app, index| app.search(index, &req))
            .await
    }

    /// Read one chunk or section with its citation.
    #[tool(annotations(
        title = "Read section or chunk",
        read_only_hint = true,
        open_world_hint = false
    ))]
    async fn read(
        &self,
        Parameters(req): Parameters<ReadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        check_input_limit(req.id.len(), MAX_ID_BYTES, "id").map_err(to_mcp)?;
        self.blocking(move |app, index| app.read(index, &req.id))
            .await
    }

    /// List graph neighbours (CONTAINS, NEXT, LINKS_TO, MENTIONS) of a chunk.
    #[tool(annotations(
        title = "Related chunks",
        read_only_hint = true,
        open_world_hint = false
    ))]
    async fn related(
        &self,
        Parameters(req): Parameters<RelatedRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        check_input_limit(req.chunk_id.len(), MAX_ID_BYTES, "chunk_id").map_err(to_mcp)?;
        self.blocking(move |app, index| app.related(index, &req.chunk_id))
            .await
    }

    /// Re-check citations against current file bytes: valid, moved, stale or missing.
    #[tool(annotations(
        title = "Verify citations",
        read_only_hint = true,
        open_world_hint = false
    ))]
    async fn verify(
        &self,
        Parameters(req): Parameters<VerifyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        check_input_limit(req.citations.len(), MAX_LIST_ITEMS, "citations").map_err(to_mcp)?;
        let mut bytes = 0usize;
        for citation in &req.citations {
            let count = validate_citation_fields(
                &citation.generation_id,
                &citation.path,
                &citation.heading_path,
                &citation.sha256,
            )
            .map_err(to_mcp)?;
            bytes = bytes.checked_add(count).ok_or_else(|| {
                to_mcp(
                    crate::store::StoreError::CapExceeded {
                        what: "arguments",
                        cap: MAX_ARGUMENT_BYTES,
                    }
                    .into(),
                )
            })?;
            check_input_limit(bytes, MAX_ARGUMENT_BYTES, "arguments").map_err(to_mcp)?;
        }
        let citations: Vec<Citation> = req.citations.into_iter().map(Citation::from).collect();
        self.blocking(move |app, index| app.verify_index(index, &citations))
            .await
    }

    /// Index generation, counts, freshness (added/removed/modified paths) and diagnostics.
    #[tool(annotations(title = "Index status", read_only_hint = true, open_world_hint = false))]
    async fn status(
        &self,
        Parameters(_req): Parameters<StatusRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(|app, index| app.status(index)).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OntoDocServer {
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        validate_tool_arguments(&request).map_err(to_mcp)?;
        let context = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        self.tool_router.call(context).await
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ontodoc", env!("CARGO_PKG_VERSION")))
            .with_instructions(format!(
                "Read-only documentation search with citations. Use `search`, then `read` or \
                 `related`; check quotes with `verify` (max {MAX_LIST_ITEMS} citations). \
                 Results may be stale when `fresh` is false; run `ontodoc index` to refresh."
            ))
    }
}

/// Maximum folder corpora served from one `docs_share` directory.
pub const MAX_CORPORA: usize = 32;
/// Maximum `docs_share` entries examined during discovery, files included.
pub const MAX_ENTRIES: usize = 4096;

/// Startup refusal while building the folder registry.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("docs_share directory is unavailable")]
    Unavailable(#[source] std::io::Error),
    #[error("folder name is not a single URL-safe path segment: {0:?}")]
    InvalidName(std::ffi::OsString),
    #[error("folder entry is a symlink or not a directory: {0}")]
    NotDirectory(String),
    #[error("more than {MAX_CORPORA} corpus folders")]
    TooMany,
    #[error("more than {MAX_ENTRIES} docs_share entries")]
    TooManyEntries,
    #[error("per-corpus permits must be 1..={MAX_PERMITS}")]
    InvalidPermits,
}

/// Independent per-folder MCP servers discovered once at startup. Each
/// immediate real subdirectory is one corpus with its own `.ontodoc` store,
/// `App` and index slot; nothing is merged, shared or auto-indexed, and an
/// unknown name never falls back to another corpus.
pub struct Registry {
    servers: Vec<(String, OntoDocServer)>,
}

impl Registry {
    pub fn discover(docs_share: &Path) -> Result<Self, RegistryError> {
        Self::discover_with(docs_share, 1, None)
    }

    /// Discovery for a shared listener: `per_corpus` permits per folder plus
    /// one process-wide `global` semaphore that every corpus draws from.
    pub fn discover_with(
        docs_share: &Path,
        per_corpus: usize,
        global: Option<Arc<Semaphore>>,
    ) -> Result<Self, RegistryError> {
        if !(1..=MAX_PERMITS).contains(&per_corpus) {
            return Err(RegistryError::InvalidPermits);
        }
        let mut names = Vec::new();
        let entries = std::fs::read_dir(docs_share).map_err(RegistryError::Unavailable)?;
        for (seen, entry) in entries.enumerate() {
            if seen == MAX_ENTRIES {
                return Err(RegistryError::TooManyEntries);
            }
            let entry = entry.map_err(RegistryError::Unavailable)?;
            let raw = entry.file_name();
            // `file_type` does not follow symlinks. Classify before naming:
            // plain files are not corpora; links and special entries refuse.
            let kind = entry.file_type().map_err(RegistryError::Unavailable)?;
            if kind.is_file() {
                continue;
            }
            let display = raw.to_string_lossy().into_owned();
            if !kind.is_dir() {
                return Err(RegistryError::NotDirectory(display));
            }
            // A version-control or store directory is metadata, not a corpus.
            if raw == ".git" || raw == ".ontodoc" {
                continue;
            }
            let name = raw
                .to_str()
                .filter(|n| valid_segment(n))
                .ok_or_else(|| RegistryError::InvalidName(raw.clone()))?
                .to_string();
            if names.len() == MAX_CORPORA {
                return Err(RegistryError::TooMany);
            }
            names.push(name);
        }
        names.sort();
        let servers = names
            .into_iter()
            .map(|name| {
                let mut server = OntoDocServer::build(&docs_share.join(&name), per_corpus);
                server.global = global.clone();
                (name, server)
            })
            .collect();
        Ok(Self { servers })
    }

    /// Sorted `(name, server)` pairs.
    pub fn servers(&self) -> impl Iterator<Item = (&str, &OntoDocServer)> {
        self.servers
            .iter()
            .map(|(name, server)| (name.as_str(), server))
    }

    /// Sorted endpoint names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.servers.iter().map(|(name, _)| name.as_str())
    }

    /// Exact, case-sensitive lookup; there is no alias or default corpus.
    pub fn get(&self, name: &str) -> Option<&OntoDocServer> {
        self.servers
            .binary_search_by(|(n, _)| n.as_str().cmp(name))
            .ok()
            .map(|i| &self.servers[i].1)
    }
}

/// A UTF-8 folder name kept verbatim as the endpoint key (the listener
/// percent-encodes it as one segment). Refuses only names that cannot be one
/// lossless segment: dot segments, separators, NUL/control characters or
/// more than 255 bytes. Non-UTF-8 names refuse rather than being altered.
fn valid_segment(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control())
}

/// Serves MCP over stdio until the client disconnects. Stdout carries only
/// protocol frames.
pub async fn serve_stdio(root: &Path) -> std::io::Result<()> {
    let (input, output) = rmcp::transport::stdio();
    let refused = Arc::new(AtomicBool::new(false));
    let input = BoundedInput::new(input, MAX_INPUT_BYTES, refused.clone());
    let service = OntoDocServer::new(root)
        .serve(BoundedTransport {
            inner: rmcp::transport::async_rw::AsyncRwTransport::new_server(input, output),
            closed: false,
        })
        .await
        .map_err(|_| std::io::Error::other("MCP initialization failed"))?;
    service
        .waiting()
        .await
        .map_err(|_| std::io::Error::other("MCP session failed"))?;
    if refused.load(Ordering::Relaxed) {
        return Err(std::io::Error::other("MCP input exceeds the byte limit"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[test]
    fn decoded_admission_counts_unknown_keys_values_and_utf8_bytes() {
        let request = |arguments| {
            serde_json::from_value::<rmcp::model::CallToolRequestParams>(
                serde_json::json!({"name":"status", "arguments":arguments}),
            )
            .unwrap()
        };
        let exact =
            request(serde_json::json!({"x": "é".repeat((MAX_ARGUMENT_BYTES - 1) / 2), "": "x"}));
        validate_tool_arguments(&exact).unwrap();
        let nested = request(
            serde_json::json!({"unknown": [{"also_unknown": "x".repeat(MAX_ARGUMENT_BYTES)}]}),
        );
        assert_eq!(
            validate_tool_arguments(&nested).unwrap_err().kind(),
            "cap_exceeded"
        );
        let key = "x".repeat(MAX_ARGUMENT_BYTES);
        validate_tool_arguments(&request(serde_json::json!({key: null}))).unwrap();
        let key = "x".repeat(MAX_ARGUMENT_BYTES + 1);
        assert_eq!(
            validate_tool_arguments(&request(serde_json::json!({key: null})))
                .unwrap_err()
                .kind(),
            "cap_exceeded"
        );
    }

    #[tokio::test]
    async fn oversized_error_preserves_exact_id_at_final_transport_boundary() {
        struct Capture(Arc<Mutex<Option<TxJsonRpcMessage<RoleServer>>>>);
        impl Transport<RoleServer> for Capture {
            type Error = std::io::Error;
            fn send(
                &mut self,
                message: TxJsonRpcMessage<RoleServer>,
            ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
                *self.0.lock().unwrap() = Some(message);
                std::future::ready(Ok(()))
            }
            async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
                None
            }
            async fn close(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let base = serde_json::to_vec(&cap_error(Some(rmcp::model::RequestId::String("".into()))))
            .unwrap()
            .len()
            + 1;
        let room = MAX_RESPONSE_BYTES - base;
        let id = rmcp::model::RequestId::String(
            format!("{}{}", "\u{1}".repeat(room / 6), "x".repeat(room % 6)).into(),
        );
        let expected = cap_error(Some(id.clone()));
        assert_eq!(
            serde_json::to_vec(&expected).unwrap().len() + 1,
            MAX_RESPONSE_BYTES
        );
        let capture = Arc::new(Mutex::new(None));
        let mut transport = BoundedTransport {
            inner: Capture(capture.clone()),
            closed: false,
        };
        transport
            .send(JsonRpcMessage::error(
                ErrorData::internal_error("x".repeat(MAX_RESPONSE_BYTES), None),
                Some(id),
            ))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(capture.lock().unwrap().as_ref().unwrap()).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    #[tokio::test]
    async fn raw_input_counts_complete_lines_and_unterminated_eof() {
        for bytes in [b"abcd".as_slice(), b"abc\n", b"ab\r\n", b"abc\nabc\n"] {
            let flag = Arc::new(AtomicBool::new(false));
            let mut input = BoundedInput::new(bytes, 4, flag.clone());
            let mut read = Vec::new();
            input.read_to_end(&mut read).await.unwrap();
            assert_eq!(read, bytes);
            assert!(!flag.load(Ordering::Relaxed));
        }
        for bytes in [b"abcde".as_slice(), b"abcd\n", b"abc\r\n"] {
            let flag = Arc::new(AtomicBool::new(false));
            let mut input = BoundedInput::new(bytes, 4, flag.clone());
            let mut read = Vec::new();
            assert!(input.read_to_end(&mut read).await.is_err());
            assert_eq!(read, bytes[..4]);
            assert!(flag.load(Ordering::Relaxed));
            assert!(input.read_u8().await.is_err());
            assert_eq!(input.read(&mut []).await.unwrap(), 0);
        }
        // The first read buffers bytes past a short line. They must remain
        // charged to the following line, whose detection byte is not forwarded.
        let flag = Arc::new(AtomicBool::new(false));
        let mut input = BoundedInput::new(b"a\nabcde".as_slice(), 4, flag.clone());
        let mut read = Vec::new();
        assert!(input.read_to_end(&mut read).await.is_err());
        assert_eq!(read, b"a\nabcd");
        assert!(flag.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn raw_input_preserves_pending_and_cancelled_read_state() {
        let (mut writer, reader) = tokio::io::duplex(32);
        let flag = Arc::new(AtomicBool::new(false));
        let mut input = BoundedInput::new(reader, 4, flag.clone());
        use tokio::io::AsyncWriteExt;
        writer.write_all(b"abc").await.unwrap();
        let mut prefix = [0; 3];
        input.read_exact(&mut prefix).await.unwrap();
        let mut byte = [0];
        {
            let read = input.read(&mut byte);
            let mut read = std::pin::pin!(read);
            std::future::poll_fn(|cx| {
                assert!(read.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
        }
        assert_eq!(input.line_bytes, 3);
        writer.write_all(b"dX").await.unwrap();
        assert_eq!(input.read(&mut byte).await.unwrap(), 1);
        assert_eq!(byte, *b"d");
        assert!(input.read(&mut byte).await.is_err());
        assert!(flag.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn native_transport_never_receives_the_overflow_byte() {
        // A malformed exact-cap line reaches native parsing, but cap+1 never does.
        for bytes in [b"xxxx".as_slice(), b"xxxxx"] {
            let flag = Arc::new(AtomicBool::new(false));
            let input = BoundedInput::new(bytes, 4, flag.clone());
            let mut transport =
                rmcp::transport::async_rw::AsyncRwTransport::new_server(input, tokio::io::sink());
            assert!(transport.receive().await.is_none());
            assert_eq!(flag.load(Ordering::Relaxed), bytes.len() > 4);
        }
    }

    #[tokio::test]
    async fn all_handlers_refuse_busy_before_current_io() {
        let root = tempfile::tempdir().unwrap();
        let server = OntoDocServer::new(root.path());
        let _permit = server.admission.clone().try_acquire_owned().unwrap();
        let expected = to_mcp(AppError::Busy);
        assert_eq!(
            server
                .status(Parameters(StatusRequest {}))
                .await
                .unwrap_err(),
            expected
        );
        assert_eq!(
            server
                .read(Parameters(ReadRequest { id: "x".into() }))
                .await
                .unwrap_err(),
            expected
        );
        assert_eq!(
            server
                .related(Parameters(RelatedRequest {
                    chunk_id: "x".into()
                }))
                .await
                .unwrap_err(),
            expected
        );
        let request = serde_json::from_value(serde_json::json!({"query":"x"})).unwrap();
        assert_eq!(
            server.search(Parameters(request)).await.unwrap_err(),
            expected
        );
        assert_eq!(
            server
                .verify(Parameters(VerifyRequest { citations: vec![] }))
                .await
                .unwrap_err(),
            expected
        );
        assert!(server.index.lock().unwrap().is_none());
        assert_eq!(server.work_counts.snapshot(), [0; 3]);
    }

    #[tokio::test]
    async fn field_caps_precede_admission_and_missing_index() {
        let root = tempfile::tempdir().unwrap();
        let server = OntoDocServer::new(root.path());
        let _permit = server.admission.clone().try_acquire_owned().unwrap();
        let expected = to_mcp(check_input_limit(MAX_ID_BYTES + 1, MAX_ID_BYTES, "id").unwrap_err());
        assert_eq!(
            server
                .read(Parameters(ReadRequest {
                    id: "x".repeat(MAX_ID_BYTES + 1)
                }))
                .await
                .unwrap_err(),
            expected
        );
        assert_eq!(
            server
                .related(Parameters(RelatedRequest {
                    chunk_id: "x".repeat(MAX_ID_BYTES + 1)
                }))
                .await
                .unwrap_err()
                .data,
            expected.data
        );
        let request =
            serde_json::from_value(serde_json::json!({"query":"x".repeat(4097)})).unwrap();
        assert_ne!(
            server.search(Parameters(request)).await.unwrap_err(),
            to_mcp(AppError::Busy)
        );
        let citations = (0..=MAX_LIST_ITEMS)
            .map(|_| CitationInput {
                generation_id: "0".repeat(64),
                path: "a.md".into(),
                heading_path: vec![],
                byte_start: 0,
                byte_end: 0,
                line_start: 1,
                line_end: 1,
                sha256: "0".repeat(64),
            })
            .collect();
        assert_ne!(
            server
                .verify(Parameters(VerifyRequest { citations }))
                .await
                .unwrap_err(),
            to_mcp(AppError::Busy)
        );
        assert!(server.index.lock().unwrap().is_none());
        assert_eq!(server.work_counts.snapshot(), [0; 3]);
    }

    #[tokio::test]
    async fn native_argument_caps_precede_typed_conversion_and_all_work() {
        let root = tempfile::tempdir().unwrap();
        let server = OntoDocServer::new(root.path());
        let (server_io, client_io) = tokio::io::duplex(1 << 20);
        let service = tokio::spawn(server.clone().serve(server_io));
        let client = ().serve(client_io).await.unwrap();
        let running = service.await.unwrap().unwrap();
        let _permit = server.admission.clone().try_acquire_owned().unwrap();
        for (name, arguments, kind) in [
            (
                "status",
                serde_json::json!({"unknown":"x".repeat(MAX_ARGUMENT_BYTES)}),
                "cap_exceeded",
            ),
            (
                "search",
                serde_json::json!({"scope":"x".repeat(4097)}),
                "cap_exceeded",
            ),
            (
                "read",
                serde_json::json!({"id":"x".repeat(MAX_ID_BYTES+1)}),
                "cap_exceeded",
            ),
            (
                "related",
                serde_json::json!({"chunk_id":"x".repeat(MAX_ID_BYTES+1)}),
                "cap_exceeded",
            ),
            (
                "verify",
                serde_json::json!({"citations":vec![serde_json::json!({});101]}),
                "cap_exceeded",
            ),
            (
                "verify",
                serde_json::json!({"citations":[{"heading_path":vec!["";65]}]}),
                "cap_exceeded",
            ),
            (
                "verify",
                serde_json::json!({"citations":[{"path":"x".repeat(4097)}]}),
                "cap_exceeded",
            ),
            (
                "verify",
                serde_json::json!({"citations":[{"sha256":"A".repeat(64)}]}),
                "invalid_argument",
            ),
        ] {
            let request = rmcp::model::CallToolRequestParams::new(name)
                .with_arguments(arguments.as_object().unwrap().clone());
            let rmcp::ServiceError::McpError(error) = client.call_tool(request).await.unwrap_err()
            else {
                panic!("expected typed refusal");
            };
            assert_eq!(error.data.unwrap()["error"], kind, "{name}");
        }
        assert_eq!(server.work_counts.snapshot(), [0; 3]);
        assert!(server.index.lock().unwrap().is_none());
        client.cancel().await.unwrap();
        running.cancel().await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_waiter_and_server_drop_keep_actual_work_admitted() {
        let root = tempfile::tempdir().unwrap();
        App::new(root.path()).index().unwrap();
        let server = OntoDocServer::new(root.path());
        let observer = server.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let waiter = tokio::spawn(async move {
            server
                .blocking::<crate::cli::StatusResponse, _>(move |app, index| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    app.status(index)
                })
                .await
        });
        entered_rx.await.unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        // Tool work runs on a snapshot; the cache lock is free but the
        // permit still belongs to the surviving work.
        assert!(observer.index.try_lock().unwrap().is_some());
        assert_eq!(
            observer
                .status(Parameters(StatusRequest {}))
                .await
                .unwrap_err(),
            to_mcp(AppError::Busy)
        );
        let admission = observer.admission.clone();
        let index = observer.index.clone();
        drop(observer);
        assert_eq!(admission.available_permits(), 0);
        release_tx.send(()).unwrap();
        // Awaiting admission is test synchronization only; production never queues.
        let _permit = admission.acquire().await.unwrap();
        assert!(index.try_lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn cancelled_cold_reload_keeps_admission_before_current_and_cache_mutation() {
        let root = tempfile::tempdir().unwrap();
        App::new(root.path()).index().unwrap();
        let server = OntoDocServer::new(root.path());
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        {
            // Hold the actual mutation lock to stop the worker before CURRENT.
            let slot = server.index.lock().unwrap();
            let calls = calls.clone();
            let mut work = Box::pin(server.blocking::<crate::cli::StatusResponse, _>(
                move |app, index| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    app.status(index)
                },
            ));
            let mut context = Context::from_waker(std::task::Waker::noop());
            assert!(work.as_mut().poll(&mut context).is_pending());
            drop(work);
            assert_eq!(server.admission.available_permits(), 0);
            // If CURRENT had been read outside the lock/worker, the previously
            // valid generation could be loaded and the tool would run.
            std::fs::write(root.path().join(".ontodoc/CURRENT"), "0".repeat(64)).unwrap();
            assert!(slot.is_none());
        }
        let _permit = server.admission.acquire().await.unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert!(server.index.try_lock().unwrap().is_none());
    }

    #[test]
    fn runtime_shutdown_keeps_admission_until_worker_stops() {
        let root = tempfile::tempdir().unwrap();
        App::new(root.path()).index().unwrap();
        let server = OntoDocServer::new(root.path());
        let worker = server.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let waiter = tokio::spawn(async move {
                worker
                    .blocking::<crate::cli::StatusResponse, _>(move |app, index| {
                        entered_tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                        app.status(index)
                    })
                    .await
            });
            entered_rx.await.unwrap();
            waiter.abort();
            assert!(waiter.await.unwrap_err().is_cancelled());
        });
        runtime.shutdown_background();
        assert_eq!(server.admission.available_permits(), 0);
        assert!(server.index.try_lock().unwrap().is_some());
        release_tx.send(()).unwrap();
        let replacement = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        replacement.block_on(async {
            let _permit = server.admission.acquire().await.unwrap();
            assert!(server.index.try_lock().unwrap().is_some());
        });
    }

    #[tokio::test]
    async fn shared_global_permits_bound_work_across_corpora_before_any_io() {
        let share = tempfile::tempdir().unwrap();
        for name in ["a", "b", "c"] {
            std::fs::create_dir(share.path().join(name)).unwrap();
        }
        assert!(matches!(
            Registry::discover_with(share.path(), 0, None),
            Err(RegistryError::InvalidPermits)
        ));
        let global = Arc::new(Semaphore::new(4));
        let registry = Registry::discover_with(share.path(), 2, Some(global.clone())).unwrap();
        let a = registry.get("a").unwrap();
        let b = registry.get("b").unwrap();
        let c = registry.get("c").unwrap();
        // Per corpus: 2 permits; corpora do not share them.
        assert_eq!(a.admission.available_permits(), 2);
        assert!(!Arc::ptr_eq(&a.admission, &b.admission));
        // Hold all 4 global permits through corpora a and b.
        let held: Vec<_> = (0..2)
            .flat_map(|_| {
                [
                    global.clone().try_acquire_owned().unwrap(),
                    global.clone().try_acquire_owned().unwrap(),
                ]
            })
            .collect();
        assert_eq!(held.len(), 4);
        // Corpus c still has per-corpus room, but the global bound refuses it
        // before CURRENT reads, reloads or spawns, and takes no c permit.
        let err = c.status(Parameters(StatusRequest {})).await.unwrap_err();
        assert_eq!(err, to_mcp(AppError::Busy));
        assert_eq!(c.admission.available_permits(), 2);
        assert_eq!(c.work_counts.snapshot(), [0; 3]);
        drop(held);
        // With global room, c proceeds to real work (no index: typed error).
        let err = c.status(Parameters(StatusRequest {})).await.unwrap_err();
        assert_ne!(err, to_mcp(AppError::Busy));
        assert_eq!(global.available_permits(), 4);
    }

    #[tokio::test]
    async fn admitted_calls_overlap_on_one_snapshot_and_excess_is_busy() {
        let root = tempfile::tempdir().unwrap();
        App::new(root.path()).index().unwrap();
        assert!(OntoDocServer::with_permits(root.path(), 0).is_none());
        assert!(OntoDocServer::with_permits(root.path(), MAX_PERMITS + 1).is_none());
        let server = OntoDocServer::with_permits(root.path(), 2).unwrap();
        // Both workers wait at one rendezvous that only opens if they run at
        // once. The wait is bounded inside the worker, so a serialized
        // regression fails the test instead of hanging runtime teardown.
        let arrived = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let spawn = |server: OntoDocServer| {
            let arrived = arrived.clone();
            tokio::spawn(async move {
                server
                    .blocking::<crate::cli::StatusResponse, _>(move |app, index| {
                        arrived.fetch_add(1, Ordering::SeqCst);
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(10);
                        while arrived.load(Ordering::SeqCst) < 2 {
                            if std::time::Instant::now() > deadline {
                                return Err(AppError::Internal);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                        app.status(index)
                    })
                    .await
            })
        };
        let (a, b) = (spawn(server.clone()), spawn(server.clone()));
        let (a, b) = (a.await.unwrap(), b.await.unwrap());
        assert!(a.is_ok() && b.is_ok(), "admitted calls did not overlap");
        // One shared load served both calls.
        assert_eq!(server.work_counts.snapshot(), [2, 2, 1]);

        // Holding both permits, a third call is refused before any work.
        let _held = server
            .admission
            .clone()
            .acquire_many_owned(2)
            .await
            .unwrap();
        assert_eq!(
            server
                .status(Parameters(StatusRequest {}))
                .await
                .unwrap_err(),
            to_mcp(AppError::Busy)
        );
        assert_eq!(server.work_counts.snapshot(), [2, 2, 1]);
    }

    #[tokio::test]
    async fn in_flight_snapshot_finishes_while_later_calls_see_new_generation() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.md"), "# A\n\nold\n").unwrap();
        let app = App::new(root.path());
        app.index().unwrap();
        let old = app.current_id().unwrap();
        let server = OntoDocServer::with_permits(root.path(), 2).unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let worker = server.clone();
        let in_flight = tokio::spawn(async move {
            worker
                .blocking::<crate::cli::StatusResponse, _>(move |app, index| {
                    entered_tx.send(()).unwrap();
                    // Bounded so a blocked later call cannot hang teardown.
                    release_rx
                        .recv_timeout(std::time::Duration::from_secs(10))
                        .map_err(|_| AppError::Internal)?;
                    app.status(index)
                })
                .await
        });
        entered_rx.await.unwrap();
        // Publish a new generation while the first call holds its snapshot.
        std::fs::write(root.path().join("a.md"), "# A\n\nnew\n").unwrap();
        app.index().unwrap();
        let new = app.current_id().unwrap();
        assert_ne!(old, new);
        let generation = |r: CallToolResult| r.structured_content.unwrap()["generation_id"].clone();
        let later = server.status(Parameters(StatusRequest {})).await.unwrap();
        assert_eq!(generation(later), serde_json::json!(new));
        release_tx.send(()).unwrap();
        let first = in_flight.await.unwrap().unwrap();
        assert_eq!(generation(first), serde_json::json!(old));
        // A broken CURRENT is refused for every later call; the cached old
        // or new index is never served as current.
        std::fs::write(root.path().join(".ontodoc/CURRENT"), "0".repeat(64)).unwrap();
        assert!(server.status(Parameters(StatusRequest {})).await.is_err());
    }

    #[tokio::test]
    async fn cache_reuses_shared_index_and_failed_reload_never_serves_old_data() {
        let root = tempfile::tempdir().unwrap();
        App::new(root.path()).index().unwrap();
        let server = OntoDocServer::new(root.path());
        server.status(Parameters(StatusRequest {})).await.unwrap();
        let original = server.index.lock().unwrap().as_ref().unwrap().1.clone();
        server
            .clone()
            .status(Parameters(StatusRequest {}))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(
            &original,
            &server.index.lock().unwrap().as_ref().unwrap().1
        ));
        std::fs::write(root.path().join(".ontodoc/CURRENT"), "0".repeat(64)).unwrap();
        let error = server
            .status(Parameters(StatusRequest {}))
            .await
            .unwrap_err();
        assert_ne!(error, to_mcp(AppError::Busy));
        assert!(Arc::ptr_eq(
            &original,
            &server.index.lock().unwrap().as_ref().unwrap().1
        ));
        assert!(server.status(Parameters(StatusRequest {})).await.is_err());
    }

    #[tokio::test]
    async fn rejected_id_stays_closed_when_receive_is_cancelled_during_close() {
        struct PendingClose(Option<RxJsonRpcMessage<RoleServer>>);
        impl Transport<RoleServer> for PendingClose {
            type Error = std::io::Error;
            fn send(
                &mut self,
                _: TxJsonRpcMessage<RoleServer>,
            ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
                std::future::ready(Ok(()))
            }
            async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
                self.0.take()
            }
            async fn close(&mut self) -> std::io::Result<()> {
                std::future::pending().await
            }
        }
        let request = serde_json::from_value(serde_json::json!({
            "jsonrpc":"2.0", "id":"\u{1}".repeat(MAX_RESPONSE_BYTES), "method":"ping"
        }))
        .unwrap();
        let mut transport = BoundedTransport {
            inner: PendingClose(Some(request)),
            closed: false,
        };
        {
            let receive = transport.receive();
            let mut receive = std::pin::pin!(receive);
            std::future::poll_fn(|cx| {
                assert!(receive.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
        }
        assert!(transport.closed);
        assert!(transport.receive().await.is_none());
    }

    #[tokio::test]
    async fn blocking_join_failure_is_a_generic_typed_error() {
        let root = tempfile::tempdir().unwrap();
        let app = App::new(root.path());
        app.index().unwrap();
        let server = OntoDocServer::new(root.path());
        let error = server
            .blocking::<crate::cli::StatusResponse, _>(|_, _| {
                panic!("private-worker-sentinel");
            })
            .await
            .unwrap_err();
        assert_eq!(error, to_mcp(AppError::Internal));
        assert_eq!(server.admission.available_permits(), 1);
        // The tool panicked outside the cache lock, so the cache is not
        // poisoned and the next caller is served normally.
        assert!(server.status(Parameters(StatusRequest {})).await.is_ok());
        assert_eq!(server.admission.available_permits(), 1);
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("private-worker-sentinel")
        );
    }
}
