# Deploy OntoDoc behind an authenticated TLS ingress

**Untested deployment example.** The service contracts below were checked against source, but this systemd/nginx configuration has not been installed or exercised end to end. No nginx or Caddy executable was found on the preparation host. Validate the configuration and acceptance checks on the target before exposing it to users. Capacity and latency observations are recorded in the [release plan](RELEASE_0.1.0_PLAN.md); no latency acceptance target or capacity verdict was assigned.

The corrected candidate restores typed MCP overflow errors with the same request ID and byte threshold as stdio. IDs too large even for that error are refused before dispatch. Current regression tests cover large ASCII and escaped IDs. The preserved measured binary still has the original plain-HTTP-500 defect; do not confuse its hash with the corrected package. Publication decisions remain in the [release plan](RELEASE_0.1.0_PLAN.md).

OntoDoc serves native MCP clients through a loopback HTTP/1.1 backend. The operator supplies TLS and authentication at the ingress; every authenticated user can read every exposed corpus. There is no document-level authorization. Indexing remains a separate trusted CLI action.

## Files and accounts

This example uses `/opt/ontodoc/bin/ontodoc`, `/srv/ontodoc/docs_share`, a dedicated unprivileged service account named `ontodoc`, and nginx on the same host. Provision these paths and the account through the host's normal administration process. The binary must be executable by the service account. Grant that account read/traverse access to each corpus and its `.ontodoc` store; reserve write access for the trusted publisher. Do not place secrets or unrelated private documents under these roots.

The preserved 0.1.0 binary targets native Linux x86_64 GNU with glibc 2.39 or newer, Linux 5.6 or newer (`openat2`), and libgcc_s. Source builds use Rust 1.95.0. Verify the release package's `SHA256SUMS` before installing it.

Index each intended corpus explicitly as its trusted publisher before starting the service:

```sh
/opt/ontodoc/bin/ontodoc --root /srv/ontodoc/docs_share/guide index
/opt/ontodoc/bin/ontodoc --root /srv/ontodoc/docs_share/guide status
```

Each immediate real child directory is exposed at `/<percent-encoded folder>/mcp`. Real `.git` and `.ontodoc` directories are skipped; other hidden directories are exposed. Plain files are ignored, while symlinks and special entries refuse startup. Discovery is sorted, limited to 32 corpora and 4,096 examined entries, and happens once at startup. Nested folders remain in their parent corpus. Inspect the actual directory list; a shell `*` glob omits hidden folders.

Restart after adding or removing a corpus folder. A discovered folder with no index returns `no_index` until trusted indexing publishes one; it does not need a restart. Schema 4 stores are required. Source changes do not update a stored snapshot automatically; reindex through the CLI and check the result. Current-byte freshness and `verify` have separate semantics described in the [README](../README.md).

## systemd service

After provisioning paths and permissions, save this example as `/etc/systemd/system/ontodoc.service`:

```ini
[Unit]
Description=OntoDoc loopback MCP service
After=network.target

[Service]
Type=simple
User=ontodoc
Group=ontodoc
ExecStart=/opt/ontodoc/bin/ontodoc serve-http --docs-share /srv/ontodoc/docs_share --listen 127.0.0.1:8080
Restart=on-failure
KillSignal=SIGTERM
TimeoutStopSec=infinity
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
UMask=0077

[Install]
WantedBy=multi-user.target
```

The service has a read-only filesystem view; the publisher runs separately. Keep corpora outside home directories because `ProtectHome=true` hides those paths. Check read access as the service account, including ancestor directories and store generations.

SIGTERM stops acceptance and drains in-flight requests. `TimeoutStopSec=infinity` preserves that behavior but may leave a stop waiting indefinitely if work stalls. A finite operator timeout permits systemd to kill remaining work; record that tradeoff before changing it. Request disconnection does not terminate already-running blocking work or release its permit early.

Validate and start only after the ingress configuration and host access controls are ready:

```sh
systemd-analyze verify /etc/systemd/system/ontodoc.service
sudo systemctl daemon-reload
sudo systemctl enable --now ontodoc.service
sudo systemctl status ontodoc.service
```

## nginx ingress

Provision a valid certificate for the chosen hostname and a protected password file through the host's existing credential process. The nginx worker must be able to read the password file; only administrators may modify it. Avoid passwords in shell command arguments or copied logs. The example uses HTTP Basic authentication over TLS; configure the native MCP client to authenticate every request.

Place this server block in the site's nginx configuration. Replace `docs.example` and certificate paths. Do not add an unauthenticated alternate location or a public proxy to port 8080.

```nginx
server {
    listen 443 ssl;
    server_name docs.example;

    ssl_certificate     /etc/nginx/tls/docs.fullchain.pem;
    ssl_certificate_key /etc/nginx/tls/docs.key;
    ssl_protocols TLSv1.2 TLSv1.3;

    auth_basic "OntoDoc";
    auth_basic_user_file /etc/nginx/ontodoc.htpasswd;
    access_log off;
    client_max_body_size 128k;
    client_header_timeout 30s;
    client_body_timeout 30s;

    location / {
        # Keep the original escaped URI: no URI suffix or rewrite here.
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host 127.0.0.1:8080;
        proxy_set_header Connection "";
        proxy_set_header Authorization "";
        proxy_request_buffering off;
        proxy_buffering off;
        proxy_read_timeout 300s;
    }
}
```

The fixed upstream `Host` must match the actual listen authority. Authentication occurs at nginx; its credential header is then removed before forwarding. Keep the original `Origin` header: OntoDoc refuses its presence with 403, so this configuration supports native clients, not browsers. Forward `Accept`, `Content-Type` and `MCP-Protocol-Version` unchanged. The backend emits JSON replies, but locked rmcp 1.7 still requires `Accept: application/json, text/event-stream` for POST, together with `Content-Type: application/json`. Use the protocol version negotiated during initialization.

Endpoint matching is exact and case-sensitive. Use the canonical URL printed to service stderr, including uppercase percent escapes for spaces and UTF-8. A `proxy_pass` URI suffix or rewrite can change encoded paths. Unknown endpoints never fall back to another corpus.

Run `sudo nginx -t` before reloading the ingress with the host's normal service command. Keep backend port 8080 inaccessible from remote hosts; the process itself binds only loopback. Local host users remain inside this trust boundary and can reach the backend. Do not enable request-body/header tracing or credentials in access logs; review inherited nginx error-log settings and retention as part of deployment.

## Link diagnostics

`ontodoc --root <folder> status` lists link diagnostics (`MissingTarget`, `MissingAnchor`, `OutsideRoot`). They are expected when a corpus links to files outside its own folder or uses absolute filesystem paths in links, and they are not index failures: publication succeeds and the links simply produce no graph edges. See the README section on link diagnostics for each kind.

## Acceptance checks on the target

These checks are required deployment work and have **not** been run for this guide:

1. Validate the systemd unit and nginx configuration, certificate chain/hostname, account permissions, and remote inability to connect directly to port 8080.
2. Confirm an unauthenticated ingress request returns 401, and valid authentication reaches MCP. Verify all configured locations require authentication. Do not record credentials in the evidence.
3. Initialize a native MCP client using the required Accept/Content-Type headers. List the five tools and call `status`, `search`, `read`, `related`, and `verify` on a small indexed corpus, using returned IDs and citation objects.
4. Test a folder containing a space and a UTF-8 name through the ingress. Use the canonical endpoint printed by OntoDoc; verify two corpora stay separate and an unknown endpoint returns 404.
5. With valid authentication, confirm Origin →403, known-endpoint GET/DELETE →405, and oversized body →413. From the trusted host, confirm a wrong backend Host →403. Confirm JSON-only Accept is refused; clients must advertise both required media types.
6. Observe corpus `busy` at exhausted work permits. Backend limits are 4 global and 2 per corpus, with 256 connections/in-flight requests and 128 KiB request/complete-response bodies. These limits are policies, not measured capacity. Timeouts at the proxy do not cancel admitted work.
7. Send SIGTERM during a bounded request and verify acceptance stops while work drains. Record actual stop behavior, resource measurements, service logs and any differences from this example without queries, document text or credentials.

Treat successful configuration parsing as syntax evidence only. TLS, authentication, encoded routing, tool parity, overload and drain require end-to-end validation before describing this deployment as tested.

Source contracts: `src/mcp.rs` registry and work permits, `src/http.rs` routing/transport/limits, `src/main.rs` signal handling, and [ADR revision 10](../.memory-bank/ADR_ONTODOC_SERVICE.md#http-0-exposure-and-numeric-contracts-accepted-revision-10). See the [release plan](RELEASE_0.1.0_PLAN.md) for candidate identity, capacity evidence and publication gates.
