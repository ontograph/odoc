use clap::Args;
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use ontocode_app_server_daemon::BootstrapOptions as AppServerBootstrapOptions;
use ontocode_app_server_daemon::LifecycleCommand as AppServerLifecycleCommand;
use ontocode_arg0::Arg0DispatchPaths;
use ontocode_arg0::arg0_dispatch_or_else;
use ontocode_cli::read_api_key_from_stdin;
use ontocode_cli::run_login_status;
use ontocode_cli::run_login_with_api_key;
use ontocode_cli::run_logout;
use ontocode_exec::Cli as ExecCli;
use ontocode_exec::Command as ExecCommand;
use ontocode_exec::ReviewArgs;
use ontocode_execpolicy::ExecPolicyCheckCommand;
use ontocode_responses_api_proxy::Args as ResponsesApiProxyArgs;
use ontocode_rollout_trace::MAX_WORKFLOW_RECORDS;
use ontocode_rollout_trace::MAX_WORKFLOW_ROLLOUTS;
use ontocode_rollout_trace::REDUCED_STATE_FILE_NAME;
use ontocode_rollout_trace::RoutineOpportunityReport;
use ontocode_rollout_trace::ToolOutcomeReport;
use ontocode_rollout_trace::WorkflowIncidenceInput;
use ontocode_rollout_trace::WorkflowIncidenceReport;
use ontocode_rollout_trace::WorkflowIncidenceRollout;
use ontocode_rollout_trace::WorkflowIncidenceWindow;
use ontocode_rollout_trace::aggregate_routine_opportunities;
use ontocode_rollout_trace::aggregate_tool_outcomes;
use ontocode_rollout_trace::replay_bundle;
use ontocode_state::StateRuntime;
use ontocode_state::memories_db_path;
use ontocode_tui::AppExitInfo;
use ontocode_tui::Cli as TuiCli;
use ontocode_tui::ExitReason;
use ontocode_utils_absolute_path::AbsolutePathBuf;
use ontocode_utils_cli::CliConfigOverrides;
use ontocode_utils_cli::ProfileV2Name;
use ontocode_utils_cli::SharedCliOptions;
use ontocode_utils_cli::resume_hint;
use owo_colors::OwoColorize;
use std::ffi::OsString;
use std::io::BufRead;
use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use supports_color::Stream;

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod app_cmd;
mod decisions_pilot;
#[cfg(test)]
#[path = "decisions_pilot_tests.rs"]
mod decisions_pilot_tests;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod desktop_app;
mod diagnostics_cmd;
mod doctor;
mod km_cmd;
mod marketplace_cmd;
mod mcp_cmd;
mod plugin_cmd;
#[cfg(target_os = "windows")]
mod sandbox_setup;
mod state_db_recovery;

use crate::km_cmd::CoreKmLlmExtractModel;
use crate::km_cmd::KmLlmLiveModelContext;
use crate::km_cmd::KmLlmRuntimeConfig;
use crate::km_cmd::OntographCommand;
use crate::mcp_cmd::McpCli;
use crate::plugin_cmd::PluginCli;
use crate::plugin_cmd::PluginSubcommand;
use diagnostics_cmd::DiagnosticsCommand;
use doctor::DoctorCommand;
use state_db_recovery as local_state_db;

use ontocode_config::LoaderOverrides;
use ontocode_core::ModelClient;
use ontocode_core::build_models_manager;
use ontocode_core::config::ConfigBuilder;
use ontocode_core::config::ConfigOverrides;
use ontocode_core::config::edit::ConfigEditsBuilder;
use ontocode_core::config::find_ontocode_home;
use ontocode_core::config::load_config_as_toml_with_cli_and_loader_overrides;
use ontocode_core::config::resolve_profile_v2_config_path;
use ontocode_core::resolve_installation_id;
use ontocode_features::FEATURES;
use ontocode_features::Feature;
use ontocode_features::Stage;
use ontocode_features::is_known_feature_key;
use ontocode_login::AuthManager;
use ontocode_login::CodexAuth;
use ontocode_memories_write::clear_memory_roots_contents;
use ontocode_models_manager::bundled_models_response;
use ontocode_models_manager::manager::RefreshStrategy;
use ontocode_ontograph_extension::km_llm_client::KmLlmClientConfig;
use ontocode_protocol::SessionId;
use ontocode_protocol::ThreadId;
use ontocode_protocol::protocol::AskForApproval;
use ontocode_protocol::protocol::SessionSource;
use ontocode_protocol::user_input::UserInput;
use ontocode_terminal_detection::TerminalName;

#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    // If a sub‑command is given, ignore requirements of the default args.
    subcommand_negates_reqs = true
)]
struct MultitoolCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    pub feature_toggles: FeatureToggles,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Run Ontocode non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Run a code review non-interactively.
    Review(ReviewCommand),

    /// Manage login.
    Login(LoginCommand),

    /// Remove stored authentication credentials.
    Logout(LogoutCommand),

    /// Manage external MCP servers for Ontocode.
    Mcp(McpCli),

    /// Manage Ontocode plugins.
    Plugin(PluginCli),

    /// Manage local Ontograph artifacts.
    Ontograph(OntographCommand),

    /// Start Ontocode as an MCP server (stdio).
    McpServer(McpServerCommand),

    /// [experimental] Run the app server or related tooling.
    AppServer(AppServerCommand),

    /// Launch the Ontocode desktop app (opens the app installer if missing).
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    App(app_cmd::AppCommand),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),

    /// Diagnose local Ontocode installation, config, auth, and runtime health.
    Doctor(DoctorCommand),

    /// Analyze local session issues without a terminal.
    Diagnostics(DiagnosticsCommand),

    /// Run commands within an Ontocode-provided sandbox.
    Sandbox(HostSandboxArgs),

    /// Debugging tools.
    Debug(DebugCommand),

    /// Execpolicy tooling.
    #[clap(hide = true)]
    Execpolicy(ExecpolicyCommand),

    /// Resume a previous interactive session (picker by default; use --last to continue the most recent).
    Resume(ResumeCommand),

    /// Archive a saved session by id or session name.
    Archive(SessionArchiveCommand),

    /// Unarchive a saved session by id or session name.
    Unarchive(SessionArchiveCommand),

    /// Fork a previous interactive session (picker by default; use --last to fork the most recent).
    Fork(ForkCommand),

    /// Internal: run the responses API proxy.
    #[clap(hide = true)]
    ResponsesApiProxy(ResponsesApiProxyArgs),

    /// Internal: relay stdio to a Unix domain socket.
    #[clap(hide = true, name = "stdio-to-uds")]
    StdioToUds(StdioToUdsCommand),

    /// [EXPERIMENTAL] Run the standalone exec-server service.
    ExecServer(ExecServerCommand),

    /// Inspect feature flags.
    Features(FeaturesCli),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    /// Shell to generate completions for
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

#[derive(Debug, Parser)]
struct DebugCommand {
    #[command(subcommand)]
    subcommand: DebugSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugSubcommand {
    /// Send one synthetic Decisions API request through the configured provider.
    #[clap(hide = true)]
    DecisionsPilot,

    /// Render the raw model catalog as JSON.
    Models(DebugModelsCommand),

    /// Generate a read-only continuation diagnostics report as JSON.
    ContinuationDiagnostics(DebugContinuationDiagnosticsCommand),

    /// Tooling: helps debug the app server.
    AppServer(DebugAppServerCommand),

    /// Render the model-visible prompt input list as JSON.
    PromptInput(DebugPromptInputCommand),

    /// Replay a rollout trace bundle and write reduced state JSON.
    #[clap(hide = true)]
    TraceReduce(DebugTraceReduceCommand),

    /// Report aggregate tool outcomes from one or more rollout trace bundles.
    #[clap(hide = true)]
    TraceStats(DebugTraceStatsCommand),

    /// Report deterministic routine-work opportunities from rollout traces.
    #[clap(hide = true)]
    TraceOpportunities(DebugTraceStatsCommand),

    /// Internal: reset local memory state for a fresh start.
    #[clap(hide = true)]
    ClearMemories,
}

#[derive(Debug, Parser)]
struct DebugAppServerCommand {
    #[command(subcommand)]
    subcommand: DebugAppServerSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugAppServerSubcommand {
    // Send message to app server V2.
    SendMessageV2(DebugAppServerSendMessageV2Command),
}

#[derive(Debug, Parser)]
struct DebugAppServerSendMessageV2Command {
    #[arg(value_name = "USER_MESSAGE", required = true)]
    user_message: String,
}

#[derive(Debug, Parser)]
struct DebugPromptInputCommand {
    /// Optional user prompt to append after session context.
    #[arg(value_name = "PROMPT")]
    prompt: Option<String>,

    /// Optional image(s) to attach to the user prompt.
    #[arg(long = "image", short = 'i', value_name = "FILE", value_delimiter = ',', num_args = 1..)]
    images: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
struct DebugModelsCommand {
    /// Skip refresh and dump only the bundled catalog shipped with this binary.
    #[arg(long = "bundled", default_value_t = false)]
    bundled: bool,
}

#[derive(Debug, Parser)]
struct DebugContinuationDiagnosticsCommand {
    /// Session root to inspect read-only.
    #[arg(long, value_name = "DIR")]
    session_root: PathBuf,

    /// Inclusive RFC 3339 start timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    start_timestamp: String,

    /// Exclusive RFC 3339 end timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    end_timestamp: String,
}

#[derive(Debug, Parser)]
struct ReviewCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    #[clap(flatten)]
    args: ReviewArgs,
}

#[derive(Debug, Parser)]
struct McpServerCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,
}

#[derive(Debug, Parser)]
struct DebugTraceReduceCommand {
    /// Trace bundle directory containing manifest.json and trace.jsonl.
    #[arg(value_name = "TRACE_BUNDLE")]
    trace_bundle: PathBuf,

    /// Output path for reduced RolloutTrace JSON. Defaults to TRACE_BUNDLE/state.json.
    #[arg(long = "output", short = 'o', value_name = "FILE")]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct DebugTraceStatsCommand {
    /// One trace bundle directory containing manifest.json and trace.jsonl.
    #[arg(
        value_name = "TRACE_BUNDLE",
        required_unless_present = "root",
        conflicts_with = "root"
    )]
    trace_bundle: Option<PathBuf>,

    /// Trace root whose direct trace-* child directories should be aggregated.
    #[arg(
        long = "root",
        value_name = "DIR",
        required_unless_present = "trace_bundle"
    )]
    root: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ResumeCommand {
    /// Session id (UUID) or session name. UUIDs take precedence if it parses.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,

    /// Continue the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false)]
    last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    all: bool,

    /// Include non-interactive sessions in the resume picker and --last selection.
    #[arg(long = "include-non-interactive", default_value_t = false)]
    include_non_interactive: bool,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    config_overrides: TuiCli,
}

#[derive(Debug, Parser)]
struct SessionArchiveCommand {
    /// Session id (UUID) or session name. UUIDs take precedence if it parses.
    #[arg(value_name = "SESSION")]
    target: String,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    config_overrides: SessionArchiveConfigOverrides,
}

#[derive(Debug, Args, Clone, Default)]
struct SessionArchiveConfigOverrides {
    #[clap(flatten)]
    shared: SharedCliOptions,

    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    #[clap(flatten)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct ForkCommand {
    /// Conversation/session id (UUID). When provided, forks this session.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,

    /// Fork the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false, conflicts_with = "session_id")]
    last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    all: bool,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    config_overrides: TuiCli,
}

#[cfg(target_os = "macos")]
type HostSandboxArgs = ontocode_cli::SeatbeltCommand;
#[cfg(target_os = "linux")]
type HostSandboxArgs = ontocode_cli::LandlockCommand;
#[cfg(target_os = "windows")]
type HostSandboxArgs = ontocode_cli::WindowsCommand;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
type HostSandboxArgs = UnsupportedSandboxArgs;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
#[derive(Debug, Parser)]
struct UnsupportedSandboxArgs {
    /// Layer $ONTOCODE_HOME/<name>.config.toml on top of the base user config.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<ProfileV2Name>,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Full command args to run under the host sandbox.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Parser)]
struct ExecpolicyCommand {
    #[command(subcommand)]
    sub: ExecpolicySubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum ExecpolicySubcommand {
    /// Check execpolicy files against a command.
    #[clap(name = "check")]
    Check(ExecPolicyCheckCommand),
}

#[derive(Debug, Parser)]
struct LoginCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,

    #[arg(
        long = "with-api-key",
        help = "Read the API key from stdin (e.g. `printenv OPENAI_API_KEY | ontocode login --with-api-key`)"
    )]
    with_api_key: bool,

    #[arg(
        long = "api-key",
        value_name = "API_KEY",
        help = "Log in with the given API key (prefer --with-api-key so the key stays out of shell history)"
    )]
    api_key: Option<String>,

    #[command(subcommand)]
    action: Option<LoginSubcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum LoginSubcommand {
    /// Show login status.
    Status,
}

#[derive(Debug, Parser)]
struct LogoutCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct AppServerCommand {
    /// Omit to run the app server; specify a subcommand for tooling.
    #[command(subcommand)]
    subcommand: Option<AppServerSubcommand>,

    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    /// Transport endpoint URL. Supported values: `stdio://` (default),
    /// `unix://`, `unix://PATH`, `ws://IP:PORT`, `off`.
    #[arg(
        long = "listen",
        value_name = "URL",
        default_value = ontocode_app_server::AppServerTransport::DEFAULT_LISTEN_URL
    )]
    listen: ontocode_app_server::AppServerTransport,

    /// Use stdio as the transport (equivalent to `--listen stdio://`).
    #[arg(long = "stdio", conflicts_with = "listen")]
    stdio: bool,

    /// Controls whether analytics are enabled by default.
    ///
    /// Analytics are disabled by default for app-server. Users have to explicitly opt in
    /// via the `analytics` section in the config.toml file.
    ///
    /// However, for first-party use cases like the VSCode IDE extension, we default analytics
    /// to be enabled by default by setting this flag. Users can still opt out by setting this
    /// in their config.toml:
    ///
    /// ```toml
    /// [analytics]
    /// enabled = false
    /// ```
    ///
    /// See https://developers.openai.com/codex/config-advanced/#metrics for more details.
    #[arg(long = "analytics-default-enabled")]
    analytics_default_enabled: bool,

    #[command(flatten)]
    auth: ontocode_app_server::AppServerWebsocketAuthArgs,
}

#[derive(Debug, Parser)]
struct ExecServerCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Codex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    /// Transport endpoint URL. Supported values: `ws://IP:PORT` (default), `stdio`, `stdio://`.
    #[arg(long = "listen", value_name = "URL", conflicts_with = "remote")]
    listen: Option<String>,

    /// Register this exec-server as a remote environment using the given base URL.
    #[arg(long = "remote", value_name = "URL", requires = "environment_id")]
    remote: Option<String>,

    /// Environment id to attach to when registering remotely.
    #[arg(long = "environment-id", value_name = "ID")]
    environment_id: Option<String>,

    /// Human-readable environment name.
    #[arg(long = "name", value_name = "NAME")]
    name: Option<String>,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::enum_variant_names)]
enum AppServerSubcommand {
    /// Manage the local app-server daemon.
    Daemon(AppServerDaemonCommand),

    /// Proxy stdio bytes to the running app-server control socket.
    Proxy(AppServerProxyCommand),

    /// [experimental] Generate TypeScript bindings for the app server protocol.
    GenerateTs(GenerateTsCommand),

    /// [experimental] Generate JSON Schema for the app server protocol.
    GenerateJsonSchema(GenerateJsonSchemaCommand),

    /// [internal] Generate internal JSON Schema artifacts for Codex tooling.
    #[clap(hide = true)]
    GenerateInternalJsonSchema(GenerateInternalJsonSchemaCommand),
}

#[derive(Debug, Args)]
struct AppServerDaemonCommand {
    #[command(subcommand)]
    subcommand: AppServerDaemonSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum AppServerDaemonSubcommand {
    /// Install durable local app-server management for SSH-driven use.
    Bootstrap,

    /// Start the local app server daemon if it is not already running.
    Start,

    /// Restart the local app server daemon.
    Restart,

    /// Stop the local app server daemon.
    Stop,

    /// Print local CLI and running app-server versions as JSON.
    Version,

    /// [internal] Run the detached pid-backed standalone updater loop.
    #[clap(hide = true)]
    PidUpdateLoop,
}

#[derive(Debug, Args)]
struct AppServerProxyCommand {
    /// Path to the app-server Unix domain socket to connect to.
    #[arg(long = "sock", value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    socket_path: Option<AbsolutePathBuf>,
}

#[derive(Debug, Args)]
struct GenerateTsCommand {
    /// Output directory where .ts files will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Optional path to the Prettier executable to format generated files
    #[arg(short = 'p', long = "prettier", value_name = "PRETTIER_BIN")]
    prettier: Option<PathBuf>,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    experimental: bool,
}

#[derive(Debug, Args)]
struct GenerateJsonSchemaCommand {
    /// Output directory where the schema bundle will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    experimental: bool,
}

#[derive(Debug, Args)]
struct GenerateInternalJsonSchemaCommand {
    /// Output directory where internal JSON Schema artifacts will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct StdioToUdsCommand {
    /// Path to the Unix domain socket to connect to.
    #[arg(value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    socket_path: AbsolutePathBuf,
}

fn parse_socket_path(raw: &str) -> Result<AbsolutePathBuf, String> {
    AbsolutePathBuf::relative_to_current_dir(raw)
        .map_err(|err| format!("failed to resolve socket path `{raw}`: {err}"))
}

fn format_exit_messages(exit_info: AppExitInfo, color_enabled: bool) -> Vec<String> {
    let AppExitInfo {
        token_usage,
        thread_id: conversation_id,
        thread_name,
        ..
    } = exit_info;

    let mut lines = Vec::new();
    if !token_usage.is_zero() {
        lines.push(token_usage.to_string());
    }

    if let Some(resume_cmd) = resume_hint(thread_name.as_deref(), conversation_id) {
        let command = if color_enabled {
            resume_cmd.cyan().to_string()
        } else {
            resume_cmd
        };
        lines.push(format!("To continue this session, run {command}"));
    }

    lines
}

/// Handle the app exit and print the results.
fn handle_app_exit(exit_info: AppExitInfo) -> anyhow::Result<()> {
    match exit_info.exit_reason {
        ExitReason::Fatal(message) => {
            eprintln!("ERROR: {message}");
            std::process::exit(1);
        }
        ExitReason::UserRequested => { /* normal exit */ }
    }

    let color_enabled = supports_color::on(Stream::Stdout).is_some();
    for line in format_exit_messages(exit_info, color_enabled) {
        println!("{line}");
    }
    Ok(())
}

fn run_execpolicycheck(cmd: ExecPolicyCheckCommand) -> anyhow::Result<()> {
    cmd.run()
}

async fn run_session_archive_cli_command(
    action: ontocode_tui::SessionArchiveAction,
    cmd: SessionArchiveCommand,
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    root_remote: Option<String>,
    root_remote_auth_token_env: Option<String>,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<String> {
    let SessionArchiveCommand {
        target,
        remote,
        config_overrides,
    } = cmd;
    interactive =
        finalize_session_archive_interactive(interactive, root_config_overrides, config_overrides);
    let explicit_remote_endpoint = resolve_remote_endpoint(
        remote.remote.or(root_remote),
        remote.remote_auth_token_env.or(root_remote_auth_token_env),
    )?;
    ontocode_tui::run_session_archive_command(
        action,
        target,
        ontocode_tui::SessionArchiveCommandOptions {
            cli: interactive,
            arg0_paths,
            explicit_remote_endpoint,
        },
    )
    .await
    .map_err(|err| anyhow::anyhow!("{err}"))
}

async fn run_debug_app_server_command(cmd: DebugAppServerCommand) -> anyhow::Result<()> {
    match cmd.subcommand {
        DebugAppServerSubcommand::SendMessageV2(cmd) => {
            let codex_bin = std::env::current_exe()?;
            ontocode_app_server_test_client::send_message_v2(
                &codex_bin,
                &[],
                cmd.user_message,
                &None,
            )
            .await
        }
    }
}

#[derive(Debug, Default, Parser, Clone)]
struct FeatureToggles {
    /// Enable a feature (repeatable). Equivalent to `-c features.<name>=true`.
    #[arg(long = "enable", value_name = "FEATURE", action = clap::ArgAction::Append, global = true)]
    enable: Vec<String>,

    /// Disable a feature (repeatable). Equivalent to `-c features.<name>=false`.
    #[arg(long = "disable", value_name = "FEATURE", action = clap::ArgAction::Append, global = true)]
    disable: Vec<String>,
}

#[derive(Debug, Default, Parser, Clone)]
struct InteractiveRemoteOptions {
    /// Connect the TUI to a remote app server endpoint.
    ///
    /// Accepted forms: `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`.
    #[arg(long = "remote", value_name = "ADDR")]
    remote: Option<String>,

    /// Name of the environment variable containing the bearer token to send to
    /// a remote app server websocket.
    #[arg(long = "remote-auth-token-env", value_name = "ENV_VAR")]
    remote_auth_token_env: Option<String>,
}

impl FeatureToggles {
    fn to_overrides(&self) -> anyhow::Result<Vec<String>> {
        let mut v = Vec::new();
        for feature in &self.enable {
            Self::validate_feature(feature)?;
            v.push(format!("features.{feature}=true"));
        }
        for feature in &self.disable {
            Self::validate_feature(feature)?;
            v.push(format!("features.{feature}=false"));
        }
        Ok(v)
    }

    fn validate_feature(feature: &str) -> anyhow::Result<()> {
        if is_known_feature_key(feature) {
            Ok(())
        } else {
            anyhow::bail!("Unknown feature flag: {feature}")
        }
    }
}

#[derive(Debug, Parser)]
struct FeaturesCli {
    #[command(subcommand)]
    sub: FeaturesSubcommand,
}

#[derive(Debug, Parser)]
enum FeaturesSubcommand {
    /// List known features with their stage and effective state.
    List,
    /// Enable a feature in config.toml.
    Enable(FeatureSetArgs),
    /// Disable a feature in config.toml.
    Disable(FeatureSetArgs),
}

#[derive(Debug, Parser)]
struct FeatureSetArgs {
    /// Feature key to update (for example: unified_exec).
    feature: String,
}

fn stage_str(stage: Stage) -> &'static str {
    match stage {
        Stage::UnderDevelopment => "under development",
        Stage::Experimental { .. } => "experimental",
        Stage::Stable => "stable",
        Stage::Deprecated => "deprecated",
        Stage::Removed => "removed",
    }
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        cli_main(arg0_paths).await?;
        Ok(())
    })
}

const COMMAND_NAME: &str = "ontocode";

fn apply_plugin_bin_names(command: &mut clap::Command) {
    for subcommand in command.get_subcommands_mut() {
        if subcommand.get_name() == "plugin" {
            subcommand.set_bin_name(format!("{COMMAND_NAME} plugin"));
            for plugin_subcommand in subcommand.get_subcommands_mut() {
                match plugin_subcommand.get_name() {
                    "add" => {
                        plugin_subcommand.set_bin_name(format!("{COMMAND_NAME} plugin add"));
                    }
                    "list" => {
                        plugin_subcommand.set_bin_name(format!("{COMMAND_NAME} plugin list"));
                    }
                    "remove" => {
                        plugin_subcommand.set_bin_name(format!("{COMMAND_NAME} plugin remove"));
                    }
                    "marketplace" => {
                        plugin_subcommand
                            .set_bin_name(format!("{COMMAND_NAME} plugin marketplace"));
                        for marketplace_subcommand in plugin_subcommand.get_subcommands_mut() {
                            match marketplace_subcommand.get_name() {
                                "add" => {
                                    marketplace_subcommand.set_bin_name(format!(
                                        "{COMMAND_NAME} plugin marketplace add"
                                    ));
                                }
                                "upgrade" => {
                                    marketplace_subcommand.set_bin_name(format!(
                                        "{COMMAND_NAME} plugin marketplace upgrade"
                                    ));
                                }
                                "remove" => {
                                    marketplace_subcommand.set_bin_name(format!(
                                        "{COMMAND_NAME} plugin marketplace remove"
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn multitool_command() -> clap::Command {
    let app_title = "Ontocode CLI";

    let mut command = MultitoolCli::command()
        .name(app_title)
        .about(format!("{app_title}\n\nIf no subcommand is specified, options will be forwarded to the interactive CLI."))
        .bin_name(COMMAND_NAME)
        .override_usage(format!(
            "{COMMAND_NAME} [OPTIONS] [PROMPT]\n       {COMMAND_NAME} [OPTIONS] <COMMAND> [ARGS]"
        ));
    apply_plugin_bin_names(&mut command);
    command
}

fn try_parse_multitool_cli_from<I, T>(args: I) -> clap::error::Result<MultitoolCli>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let mut command = multitool_command();
    let mut matches = command.try_get_matches_from_mut(args)?;
    let cli = MultitoolCli::from_arg_matches_mut(&mut matches)?;
    if cli.subcommand.is_none() && cli.interactive.prompt.as_deref() == Some("update") {
        return Err(command.error(
            clap::error::ErrorKind::InvalidSubcommand,
            "unrecognized subcommand 'update'",
        ));
    }
    Ok(cli)
}

async fn cli_main(arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    let command_name = COMMAND_NAME;
    let MultitoolCli {
        config_overrides: mut root_config_overrides,
        feature_toggles,
        remote,
        mut interactive,
        subcommand,
    } = try_parse_multitool_cli_from(std::env::args_os()).unwrap_or_else(|err| err.exit());

    // Fold --enable/--disable into config overrides so they flow to all subcommands.
    let toggle_overrides = feature_toggles.to_overrides()?;
    root_config_overrides.raw_overrides.extend(toggle_overrides);
    let root_remote = remote.remote;
    let root_remote_auth_token_env = remote.remote_auth_token_env;
    let root_strict_config = interactive.strict_config;
    reject_root_strict_config_for_subcommand(root_strict_config, &subcommand)?;
    if let Some(subcommand) = subcommand.as_ref() {
        profile_v2_for_subcommand(&interactive, subcommand)?;
    }

    match subcommand {
        None => {
            prepend_config_flags(
                &mut interactive.config_overrides,
                root_config_overrides.clone(),
            );
            let exit_info = run_interactive_tui(
                interactive,
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::Exec(mut exec_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "exec",
            )?;
            exec_cli
                .shared
                .inherit_exec_root_options(&interactive.shared);
            exec_cli.strict_config |= root_strict_config;
            prepend_config_flags(
                &mut exec_cli.config_overrides,
                root_config_overrides.clone(),
            );
            ontocode_exec::run_main(exec_cli, arg0_paths.clone()).await?;
        }
        Some(Subcommand::Review(ReviewCommand {
            strict_config,
            args: review_args,
        })) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "review",
            )?;
            let mut exec_cli = ExecCli::try_parse_from([command_name, "exec"])?;
            exec_cli
                .shared
                .inherit_exec_root_options(&interactive.shared);
            exec_cli.command = Some(ExecCommand::Review(review_args));
            exec_cli.strict_config = strict_config || root_strict_config;
            prepend_config_flags(
                &mut exec_cli.config_overrides,
                root_config_overrides.clone(),
            );
            ontocode_exec::run_main(exec_cli, arg0_paths.clone()).await?;
        }
        Some(Subcommand::McpServer(McpServerCommand { strict_config })) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "mcp-server",
            )?;
            ontocode_mcp_server::run_main(
                arg0_paths.clone(),
                root_config_overrides,
                strict_config || root_strict_config,
            )
            .await?;
        }
        Some(Subcommand::Mcp(mut mcp_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "mcp",
            )?;
            // Propagate any root-level config overrides (e.g. `-c key=value`).
            prepend_config_flags(&mut mcp_cli.config_overrides, root_config_overrides.clone());
            let loader_overrides =
                loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?;
            mcp_cli.run(loader_overrides).await?;
        }
        Some(Subcommand::Plugin(plugin_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "plugin",
            )?;
            let PluginCli {
                mut config_overrides,
                subcommand,
            } = plugin_cli;
            prepend_config_flags(&mut config_overrides, root_config_overrides.clone());
            match subcommand {
                PluginSubcommand::Add(args) => {
                    let overrides = config_overrides
                        .parse_overrides()
                        .map_err(anyhow::Error::msg)?;
                    plugin_cmd::run_plugin_add(overrides, args).await?;
                }
                PluginSubcommand::List(args) => {
                    let overrides = config_overrides
                        .parse_overrides()
                        .map_err(anyhow::Error::msg)?;
                    plugin_cmd::run_plugin_list(overrides, args).await?;
                }
                PluginSubcommand::Marketplace(mut marketplace_cli) => {
                    prepend_config_flags(&mut marketplace_cli.config_overrides, config_overrides);
                    marketplace_cli.run().await?;
                }
                PluginSubcommand::Remove(args) => {
                    let overrides = config_overrides
                        .parse_overrides()
                        .map_err(anyhow::Error::msg)?;
                    plugin_cmd::run_plugin_remove(overrides, args).await?;
                }
            }
        }
        Some(Subcommand::Ontograph(command)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "ontograph",
            )?;
            let km_llm_runtime = load_ontograph_km_llm_runtime_config(
                &root_config_overrides,
                interactive.config_profile_v2.as_ref(),
            )
            .await?;
            km_cmd::run(command, km_llm_runtime)?;
        }
        Some(Subcommand::AppServer(app_server_cli)) => {
            let AppServerCommand {
                subcommand,
                strict_config: app_server_strict_config,
                listen,
                stdio,
                analytics_default_enabled,
                auth,
            } = app_server_cli;
            let strict_config = app_server_strict_config || root_strict_config;
            reject_strict_config_for_app_server_subcommand(strict_config, subcommand.as_ref())?;
            reject_remote_mode_for_app_server_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                subcommand.as_ref(),
            )?;
            match subcommand {
                None => {
                    let transport = if stdio {
                        ontocode_app_server::AppServerTransport::Stdio
                    } else {
                        listen
                    };
                    let auth = auth.try_into_settings()?;
                    ontocode_app_server::run_main_with_transport_options(
                        arg0_paths.clone(),
                        root_config_overrides,
                        LoaderOverrides::default(),
                        strict_config,
                        analytics_default_enabled,
                        transport,
                        ontocode_protocol::protocol::SessionSource::VSCode,
                        auth,
                        ontocode_app_server::AppServerRuntimeOptions::default(),
                    )
                    .await?;
                }
                Some(AppServerSubcommand::Daemon(daemon_cli)) => match daemon_cli.subcommand {
                    AppServerDaemonSubcommand::Start => {
                        print_app_server_daemon_output(AppServerLifecycleCommand::Start).await?;
                    }
                    AppServerDaemonSubcommand::Bootstrap => {
                        let output =
                            ontocode_app_server_daemon::bootstrap(AppServerBootstrapOptions)
                                .await?;
                        println!("{}", serde_json::to_string(&output)?);
                    }
                    AppServerDaemonSubcommand::Restart => {
                        print_app_server_daemon_output(AppServerLifecycleCommand::Restart).await?;
                    }
                    AppServerDaemonSubcommand::Stop => {
                        print_app_server_daemon_output(AppServerLifecycleCommand::Stop).await?;
                    }
                    AppServerDaemonSubcommand::Version => {
                        print_app_server_daemon_output(AppServerLifecycleCommand::Version).await?;
                    }
                    AppServerDaemonSubcommand::PidUpdateLoop => {
                        ontocode_app_server_daemon::run_pid_update_loop().await?;
                    }
                },
                Some(AppServerSubcommand::Proxy(proxy_cli)) => {
                    let socket_path = match proxy_cli.socket_path {
                        Some(socket_path) => socket_path,
                        None => {
                            let ontocode_home = find_ontocode_home()?;
                            ontocode_app_server::app_server_control_socket_path(&ontocode_home)?
                        }
                    };
                    ontocode_stdio_to_uds::run(socket_path.as_path()).await?;
                }
                Some(AppServerSubcommand::GenerateTs(gen_cli)) => {
                    let options = ontocode_app_server_protocol::GenerateTsOptions {
                        experimental_api: gen_cli.experimental,
                        ..Default::default()
                    };
                    ontocode_app_server_protocol::generate_ts_with_options(
                        &gen_cli.out_dir,
                        gen_cli.prettier.as_deref(),
                        options,
                    )?;
                }
                Some(AppServerSubcommand::GenerateJsonSchema(gen_cli)) => {
                    ontocode_app_server_protocol::generate_json_with_experimental(
                        &gen_cli.out_dir,
                        gen_cli.experimental,
                    )?;
                }
                Some(AppServerSubcommand::GenerateInternalJsonSchema(gen_cli)) => {
                    ontocode_app_server_protocol::generate_internal_json_schema(&gen_cli.out_dir)?;
                }
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        Some(Subcommand::App(app_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "app",
            )?;
            app_cmd::run_app(app_cli).await?;
        }
        Some(Subcommand::Resume(ResumeCommand {
            session_id,
            last,
            all,
            include_non_interactive,
            remote,
            config_overrides,
        })) => {
            interactive = finalize_resume_interactive(
                interactive,
                root_config_overrides.clone(),
                session_id,
                last,
                all,
                include_non_interactive,
                config_overrides,
            );
            let exit_info = run_interactive_tui(
                interactive,
                remote.remote.or(root_remote.clone()),
                remote
                    .remote_auth_token_env
                    .or(root_remote_auth_token_env.clone()),
                arg0_paths.clone(),
            )
            .await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::Archive(cmd)) => {
            let output = run_session_archive_cli_command(
                ontocode_tui::SessionArchiveAction::Archive,
                cmd,
                interactive,
                root_config_overrides.clone(),
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            println!("{output}");
        }
        Some(Subcommand::Unarchive(cmd)) => {
            let output = run_session_archive_cli_command(
                ontocode_tui::SessionArchiveAction::Unarchive,
                cmd,
                interactive,
                root_config_overrides.clone(),
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            println!("{output}");
        }
        Some(Subcommand::Fork(ForkCommand {
            session_id,
            last,
            all,
            remote,
            config_overrides,
        })) => {
            interactive = finalize_fork_interactive(
                interactive,
                root_config_overrides.clone(),
                session_id,
                last,
                all,
                config_overrides,
            );
            let exit_info = run_interactive_tui(
                interactive,
                remote.remote.or(root_remote.clone()),
                remote
                    .remote_auth_token_env
                    .or(root_remote_auth_token_env.clone()),
                arg0_paths.clone(),
            )
            .await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::Login(mut login_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "login",
            )?;
            prepend_config_flags(
                &mut login_cli.config_overrides,
                root_config_overrides.clone(),
            );
            match login_cli.action {
                Some(LoginSubcommand::Status) => {
                    run_login_status(login_cli.config_overrides).await;
                }
                None => {
                    if let Some(api_key) = login_cli.api_key {
                        eprintln!(
                            "Warning: --api-key leaves the key in shell history; prefer `printenv OPENAI_API_KEY | ontocode login --with-api-key`."
                        );
                        run_login_with_api_key(login_cli.config_overrides, api_key).await;
                    } else if login_cli.with_api_key {
                        let api_key = read_api_key_from_stdin();
                        run_login_with_api_key(login_cli.config_overrides, api_key).await;
                    } else {
                        eprintln!(
                            "Pipe an API key to log in, e.g. `printenv OPENAI_API_KEY | ontocode login --with-api-key`."
                        );
                        std::process::exit(1);
                    }
                }
            }
        }
        Some(Subcommand::Logout(mut logout_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "logout",
            )?;
            prepend_config_flags(
                &mut logout_cli.config_overrides,
                root_config_overrides.clone(),
            );
            run_logout(logout_cli.config_overrides).await;
        }
        Some(Subcommand::Completion(completion_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "completion",
            )?;
            print_completion(completion_cli);
        }
        Some(Subcommand::Doctor(doctor_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "doctor",
            )?;
            doctor::run_doctor(
                doctor_cli,
                root_config_overrides.clone(),
                &interactive,
                &arg0_paths,
            )
            .await?;
        }
        Some(Subcommand::Diagnostics(diagnostics_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "diagnostics",
            )?;
            diagnostics_cmd::run_diagnostics_command(diagnostics_cli).await?;
        }
        Some(Subcommand::Sandbox(mut sandbox_cli)) => {
            #[cfg(target_os = "windows")]
            if let Some(setup_cli) = sandbox_setup::parse_setup_command(&sandbox_cli.command)? {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "sandbox setup",
                )?;
                sandbox_setup::run(setup_cli).await?;
                return Ok(());
            }
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "sandbox",
            )?;
            let config_profile = sandbox_cli
                .config_profile
                .as_ref()
                .or(interactive.config_profile_v2.as_ref());
            let loader_overrides = loader_overrides_for_profile(config_profile)?;
            prepend_config_flags(
                &mut sandbox_cli.config_overrides,
                root_config_overrides.clone(),
            );
            #[cfg(target_os = "macos")]
            ontocode_cli::run_command_under_seatbelt(
                sandbox_cli,
                arg0_paths.codex_linux_sandbox_exe.clone(),
                loader_overrides,
            )
            .await?;
            #[cfg(target_os = "linux")]
            ontocode_cli::run_command_under_landlock(
                sandbox_cli,
                arg0_paths.codex_linux_sandbox_exe.clone(),
                loader_overrides,
            )
            .await?;
            #[cfg(target_os = "windows")]
            ontocode_cli::run_command_under_windows_sandbox(
                sandbox_cli,
                arg0_paths.codex_linux_sandbox_exe.clone(),
                loader_overrides,
            )
            .await?;
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                let _ = loader_overrides;
                anyhow::bail!("`ontocode sandbox` is not supported on this operating system");
            }
        }
        Some(Subcommand::Debug(DebugCommand { subcommand })) => match subcommand {
            DebugSubcommand::DecisionsPilot => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug decisions-pilot",
                )?;
                decisions_pilot::run(root_config_overrides).await?;
            }
            DebugSubcommand::Models(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug models",
                )?;
                run_debug_models_command(cmd, root_config_overrides).await?;
            }
            DebugSubcommand::ContinuationDiagnostics(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug continuation-diagnostics",
                )?;
                run_debug_continuation_diagnostics_command(cmd).await?;
            }
            DebugSubcommand::AppServer(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug app-server",
                )?;
                run_debug_app_server_command(cmd).await?;
            }
            DebugSubcommand::PromptInput(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug prompt-input",
                )?;
                run_debug_prompt_input_command(
                    cmd,
                    root_config_overrides,
                    interactive,
                    arg0_paths.clone(),
                )
                .await?;
            }
            DebugSubcommand::TraceReduce(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug trace-reduce",
                )?;
                run_debug_trace_reduce_command(cmd).await?;
            }
            DebugSubcommand::TraceStats(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug trace-stats",
                )?;
                run_debug_trace_stats_command(cmd)?;
            }
            DebugSubcommand::TraceOpportunities(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug trace-opportunities",
                )?;
                run_debug_trace_opportunities_command(cmd)?;
            }
            DebugSubcommand::ClearMemories => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug clear-memories",
                )?;
                run_debug_clear_memories_command(&root_config_overrides).await?;
            }
        },
        Some(Subcommand::Execpolicy(ExecpolicyCommand { sub })) => match sub {
            ExecpolicySubcommand::Check(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "execpolicy check",
                )?;
                run_execpolicycheck(cmd)?
            }
        },
        Some(Subcommand::ResponsesApiProxy(args)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "responses-api-proxy",
            )?;
            tokio::task::spawn_blocking(move || ontocode_responses_api_proxy::run_main(args))
                .await??;
        }
        Some(Subcommand::StdioToUds(cmd)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "stdio-to-uds",
            )?;
            let socket_path = cmd.socket_path;
            ontocode_stdio_to_uds::run(socket_path.as_path()).await?;
        }
        Some(Subcommand::ExecServer(cmd)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "exec-server",
            )?;
            let strict_config = cmd.strict_config || root_strict_config;
            run_exec_server_command(cmd, &arg0_paths, &root_config_overrides, strict_config)
                .await?;
        }
        Some(Subcommand::Features(FeaturesCli { sub })) => match sub {
            FeaturesSubcommand::List => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "features list",
                )?;
                let mut cli_kv_overrides = root_config_overrides
                    .parse_overrides()
                    .map_err(anyhow::Error::msg)?;

                // Honor `--search` via the canonical web_search mode.
                if interactive.web_search {
                    cli_kv_overrides.push((
                        "web_search".to_string(),
                        toml::Value::String("live".to_string()),
                    ));
                }

                let config = ConfigBuilder::default()
                    .cli_overrides(cli_kv_overrides)
                    .build()
                    .await?;
                let mut rows = Vec::with_capacity(FEATURES.len());
                let mut name_width = 0;
                let mut stage_width = 0;
                for def in FEATURES {
                    let name = def.key;
                    let stage = stage_str(def.stage);
                    let enabled = config.features.enabled(def.id);
                    name_width = name_width.max(name.len());
                    stage_width = stage_width.max(stage.len());
                    rows.push((name, stage, enabled));
                }
                rows.sort_unstable_by_key(|(name, _, _)| *name);

                for (name, stage, enabled) in rows {
                    println!("{name:<name_width$}  {stage:<stage_width$}  {enabled}");
                }
            }
            FeaturesSubcommand::Enable(FeatureSetArgs { feature }) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "features enable",
                )?;
                enable_feature_in_config(&feature).await?;
            }
            FeaturesSubcommand::Disable(FeatureSetArgs { feature }) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "features disable",
                )?;
                disable_feature_in_config(&feature).await?;
            }
        },
    }

    Ok(())
}

fn profile_v2_for_subcommand<'a>(
    interactive: &'a TuiCli,
    subcommand: &Subcommand,
) -> anyhow::Result<Option<&'a ProfileV2Name>> {
    let Some(profile_v2) = interactive.config_profile_v2.as_ref() else {
        return Ok(None);
    };

    match subcommand {
        Subcommand::Exec(_)
        | Subcommand::Review(_)
        | Subcommand::Resume(_)
        | Subcommand::Archive(_)
        | Subcommand::Unarchive(_)
        | Subcommand::Fork(_)
        | Subcommand::Mcp(_)
        | Subcommand::Sandbox(_)
        | Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::PromptInput(_),
        }) => Ok(Some(profile_v2)),
        _ => anyhow::bail!(
            "--profile only applies to runtime commands and `ontocode mcp`: `ontocode`, `ontocode exec`, `ontocode review`, `ontocode resume`, `ontocode archive`, `ontocode unarchive`, `ontocode fork`, `ontocode mcp`, `ontocode sandbox`, and `ontocode debug prompt-input`."
        ),
    }
}

async fn run_exec_server_command(
    cmd: ExecServerCommand,
    arg0_paths: &Arg0DispatchPaths,
    root_config_overrides: &CliConfigOverrides,
    strict_config: bool,
) -> anyhow::Result<()> {
    let codex_self_exe = arg0_paths
        .codex_self_exe
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Codex executable path is not configured"))?;
    let runtime_paths = ontocode_exec_server::ExecServerRuntimePaths::new(
        codex_self_exe,
        arg0_paths.codex_linux_sandbox_exe.clone(),
    )?;
    if let Some(base_url) = cmd.remote {
        let environment_id = cmd
            .environment_id
            .ok_or_else(|| anyhow::anyhow!("--environment-id is required when --remote is set"))?;
        let config = load_exec_server_config(root_config_overrides, strict_config).await?;
        let auth_provider = load_exec_server_remote_auth_provider(&config, &base_url).await?;
        let mut remote_config = ontocode_exec_server::RemoteEnvironmentConfig::new(
            base_url,
            environment_id,
            auth_provider,
        )?;
        if let Some(name) = cmd.name {
            remote_config.name = name;
        }
        ontocode_exec_server::run_remote_environment(remote_config, runtime_paths).await?;
        Ok(())
    } else {
        if strict_config {
            // Local exec-server startup does not consume Config, but strict
            // mode should still reject unknown fields before opening a listener.
            let _validated_config =
                load_exec_server_config(root_config_overrides, strict_config).await?;
        }
        let listen_url = cmd
            .listen
            .as_deref()
            .unwrap_or(ontocode_exec_server::DEFAULT_LISTEN_URL);
        ontocode_exec_server::run_main(listen_url, runtime_paths)
            .await
            .map_err(anyhow::Error::from_boxed)
    }
}

async fn load_exec_server_remote_auth_provider(
    config: &ontocode_core::config::Config,
    base_url: &str,
) -> anyhow::Result<ontocode_api::SharedAuthProvider> {
    let auth = load_exec_server_remote_auth(
        config,
        "remote exec-server registration requires API key authentication; run `ontocode login --with-api-key` or set CODEX_API_KEY",
    )
    .await?;

    if !is_supported_exec_server_remote_auth(&auth) {
        anyhow::bail!("remote exec-server registration requires API key authentication");
    }

    validate_api_key_remote_host(base_url)?;

    Ok(ontocode_model_provider::auth_provider_from_auth(&auth))
}

fn is_supported_exec_server_remote_auth(auth: &CodexAuth) -> bool {
    auth.is_api_key_auth()
}

fn validate_api_key_remote_host(base_url: &str) -> anyhow::Result<()> {
    let url = url::Url::parse(base_url)
        .map_err(|err| anyhow::anyhow!("invalid remote exec-server registration URL: {err}"))?;
    let host = url.host().ok_or_else(|| {
        anyhow::anyhow!("remote exec-server registration URL must include a host")
    })?;

    let is_loopback = match &host {
        url::Host::Domain(host) => host.eq_ignore_ascii_case("localhost"),
        url::Host::Ipv4(ip) => ip.is_loopback(),
        url::Host::Ipv6(ip) => ip.is_loopback(),
    };
    let is_openai_host = match &host {
        url::Host::Domain(host) => ["openai.com", "openai.org"].into_iter().any(|domain| {
            host.eq_ignore_ascii_case(domain)
                || host.to_ascii_lowercase().ends_with(&format!(".{domain}"))
        }),
        _ => false,
    };
    let is_allowed = match url.scheme() {
        "https" => is_loopback || is_openai_host,
        "http" => is_loopback,
        _ => false,
    };

    if !is_allowed {
        anyhow::bail!(
            "remote exec-server API-key authentication is restricted to HTTPS openai.com and openai.org hosts and subdomains or loopback hosts"
        );
    }

    Ok(())
}

async fn load_exec_server_config(
    root_config_overrides: &CliConfigOverrides,
    strict_config: bool,
) -> anyhow::Result<ontocode_core::config::Config> {
    let cli_kv_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    Ok(ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .strict_config(strict_config)
        .build()
        .await?)
}

async fn load_exec_server_remote_auth(
    config: &ontocode_core::config::Config,
    missing_auth_error: &'static str,
) -> anyhow::Result<ontocode_login::CodexAuth> {
    let auth_manager =
        AuthManager::shared_from_config(config, /*enable_codex_api_key_env*/ true).await?;

    let auth = match auth_manager.auth().await {
        Some(auth) => auth,
        None => {
            auth_manager.reload().await?;
            auth_manager
                .auth()
                .await
                .ok_or_else(|| anyhow::anyhow!(missing_auth_error))?
        }
    };

    Ok(auth)
}

async fn enable_feature_in_config(feature: &str) -> anyhow::Result<()> {
    FeatureToggles::validate_feature(feature)?;
    let ontocode_home = find_ontocode_home()?;
    ConfigEditsBuilder::new(&ontocode_home)
        .set_feature_enabled(feature, /*enabled*/ true)
        .apply()
        .await?;
    println!("Enabled feature `{feature}` in config.toml.");
    maybe_print_under_development_feature_warning(&ontocode_home, feature);
    Ok(())
}

async fn disable_feature_in_config(feature: &str) -> anyhow::Result<()> {
    FeatureToggles::validate_feature(feature)?;
    let ontocode_home = find_ontocode_home()?;
    ConfigEditsBuilder::new(&ontocode_home)
        .set_feature_enabled(feature, /*enabled*/ false)
        .apply()
        .await?;
    println!("Disabled feature `{feature}` in config.toml.");
    Ok(())
}

fn loader_overrides_for_profile(
    profile_v2: Option<&ProfileV2Name>,
) -> anyhow::Result<LoaderOverrides> {
    match profile_v2 {
        Some(profile_v2) => {
            let ontocode_home = find_ontocode_home()?;
            Ok(LoaderOverrides {
                user_config_path: Some(resolve_profile_v2_config_path(&ontocode_home, profile_v2)),
                user_config_profile: Some(profile_v2.clone()),
                ..Default::default()
            })
        }
        None => Ok(LoaderOverrides::default()),
    }
}

async fn load_ontograph_km_llm_runtime_config(
    root_config_overrides: &CliConfigOverrides,
    profile_v2: Option<&ProfileV2Name>,
) -> anyhow::Result<KmLlmRuntimeConfig> {
    let ontocode_home = find_ontocode_home()?;
    let cwd = AbsolutePathBuf::current_dir()?;
    let cli_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let loader_overrides = loader_overrides_for_profile(profile_v2)?;
    let config_toml = load_config_as_toml_with_cli_and_loader_overrides(
        &ontocode_home,
        Some(&cwd),
        cli_overrides.clone(),
        loader_overrides.clone(),
    )
    .await?;

    let llm = config_toml
        .ontograph
        .and_then(|ontograph| ontograph.km)
        .and_then(|km| km.llm);
    let Some(llm) = llm else {
        return Ok(KmLlmRuntimeConfig {
            enabled: false,
            model_context: None,
        });
    };
    if !llm.enabled {
        return Ok(KmLlmRuntimeConfig {
            enabled: false,
            model_context: None,
        });
    }

    let mut effective_overrides = cli_overrides;
    if !effective_overrides
        .iter()
        .any(|(key, _)| key == "model_provider")
        && let Some(model_provider) = llm.model_provider.clone()
    {
        effective_overrides.push((
            "model_provider".to_string(),
            toml::Value::String(model_provider),
        ));
    }
    if !effective_overrides.iter().any(|(key, _)| key == "model")
        && let Some(model) = llm.model.clone()
    {
        effective_overrides.push(("model".to_string(), toml::Value::String(model)));
    }
    let config = ConfigBuilder::default()
        .cli_overrides(effective_overrides)
        .loader_overrides(loader_overrides)
        .build()
        .await?;
    let auth_manager =
        AuthManager::shared_from_config(&config, /*enable_codex_api_key_env*/ true).await?;
    let models_manager = build_models_manager(&config, auth_manager.clone());
    let model = models_manager
        .get_default_model(&config.model, RefreshStrategy::Offline)
        .await;
    let model_info = models_manager
        .get_model_info(&model, &config.to_models_manager_config())
        .await;
    let installation_id = resolve_installation_id(&config.codex_home).await?;
    let thread_id = ThreadId::new();
    let model_client = ModelClient::new(
        Some(auth_manager),
        SessionId::new(),
        thread_id,
        installation_id,
        config.model_provider.clone(),
        SessionSource::Cli,
        /*parent_thread_id*/ None,
        config.model_verbosity,
        config.features.enabled(Feature::EnableRequestCompression),
        config.features.enabled(Feature::RuntimeMetrics),
        /*beta_features_header*/ None,
        /*attestation_provider*/ None,
    );
    let provider_is_local = config
        .model_provider
        .base_url
        .as_deref()
        .is_some_and(provider_url_is_local);
    let model_context = KmLlmLiveModelContext {
        config: KmLlmClientConfig {
            enabled: llm.enabled,
            offline_only: llm.offline_only,
            provider_is_local,
            max_input_tokens: llm.max_input_tokens,
            model_slug: model.clone(),
            provider_id: config.model_provider_id.clone(),
            project_root: crate::km_cmd::optional_project_root().ok().flatten(),
        },
        model: CoreKmLlmExtractModel::new(
            model_client,
            model_info,
            model,
            llm.reasoning_effort,
            config.service_tier,
        ),
    };

    Ok(KmLlmRuntimeConfig {
        enabled: true,
        model_context: Some(model_context),
    })
}

fn provider_url_is_local(base_url: &str) -> bool {
    let Ok(url) = url::Url::parse(base_url) else {
        return false;
    };
    match url.host() {
        Some(url::Host::Domain(host)) => host == "localhost",
        Some(url::Host::Ipv4(addr)) => addr.is_loopback(),
        Some(url::Host::Ipv6(addr)) => addr.is_loopback(),
        None => false,
    }
}

fn maybe_print_under_development_feature_warning(ontocode_home: &std::path::Path, feature: &str) {
    let Some(spec) = FEATURES.iter().find(|spec| spec.key == feature) else {
        return;
    };
    if !matches!(spec.stage, Stage::UnderDevelopment) {
        return;
    }

    let config_path = ontocode_home.join(ontocode_config::CONFIG_TOML_FILE);
    eprintln!(
        "Under-development features enabled: {feature}. Under-development features are incomplete and may behave unpredictably. To suppress this warning, set `suppress_unstable_features_warning = true` in {}.",
        config_path.display()
    );
}

async fn run_debug_trace_reduce_command(cmd: DebugTraceReduceCommand) -> anyhow::Result<()> {
    let output = cmd
        .output
        .unwrap_or_else(|| cmd.trace_bundle.join(REDUCED_STATE_FILE_NAME));

    let trace = replay_bundle(&cmd.trace_bundle)?;
    let reduced_json = serde_json::to_vec_pretty(&trace)?;
    tokio::fs::write(&output, reduced_json).await?;
    println!("{}", output.display());

    Ok(())
}

async fn run_debug_continuation_diagnostics_command(
    cmd: DebugContinuationDiagnosticsCommand,
) -> anyhow::Result<()> {
    let report = ontocode_rollout::continuation_diagnostics::analyze_continuation_events(
        &cmd.session_root,
        &cmd.start_timestamp,
        &cmd.end_timestamp,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.files_truncated || report.lines_truncated || report.details_truncated {
        anyhow::bail!("continuation diagnostics incomplete: report truncated");
    }
    Ok(())
}

fn run_debug_trace_stats_command(cmd: DebugTraceStatsCommand) -> anyhow::Result<()> {
    let bundle_paths = debug_trace_bundle_paths(cmd)?;
    println!("{}", trace_stats_json(&bundle_paths)?);
    Ok(())
}

fn run_debug_trace_opportunities_command(cmd: DebugTraceStatsCommand) -> anyhow::Result<()> {
    let bundle_paths = debug_trace_bundle_paths(cmd)?;
    println!("{}", trace_opportunities_json(&bundle_paths)?);
    Ok(())
}

fn debug_trace_bundle_paths(cmd: DebugTraceStatsCommand) -> anyhow::Result<Vec<PathBuf>> {
    match (cmd.trace_bundle, cmd.root) {
        (Some(trace_bundle), None) => Ok(vec![trace_bundle]),
        (None, Some(root)) => trace_bundle_paths(&root),
        (Some(_), Some(_)) | (None, None) => unreachable!("clap requires exactly one input"),
    }
}

fn trace_bundle_paths(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = std::fs::read_dir(root)?
        .map(|entry| {
            let entry = entry?;
            let path = entry.path();
            Ok((entry.file_type()?.is_dir()
                && entry.file_name().to_string_lossy().starts_with("trace-"))
            .then_some(path))
        })
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn trace_stats_json(bundle_paths: &[PathBuf]) -> anyhow::Result<String> {
    if bundle_paths.is_empty() {
        return Ok(serde_json::to_string_pretty(&serde_json::json!({
            "tool_outcomes": ToolOutcomeReport::default(),
            "workflow_incidence": WorkflowIncidenceReport::default(),
        }))?);
    }

    let (
        workflow_rollouts,
        accepted_bundle_paths,
        upstream_omitted_rollout_count,
        upstream_omitted_record_count,
    ) = replay_workflow_rollouts(bundle_paths)?;
    let outcome_report =
        if let Some((first_path, remaining_paths)) = accepted_bundle_paths.split_first() {
            let mut combined = replay_bundle(first_path)?;
            let first_tool_calls = std::mem::take(&mut combined.tool_calls);
            for (tool_call_id, tool_call) in first_tool_calls {
                combined
                    .tool_calls
                    .insert(format!("0:{tool_call_id}"), tool_call);
            }
            for (bundle_index, path) in remaining_paths.iter().enumerate() {
                let trace = replay_bundle(path)?;
                for (tool_call_id, tool_call) in trace.tool_calls {
                    combined
                        .tool_calls
                        .insert(format!("{}:{tool_call_id}", bundle_index + 1), tool_call);
                }
            }
            aggregate_tool_outcomes(&combined)
        } else {
            ToolOutcomeReport::default()
        };

    let start_unix_ms = workflow_rollouts
        .iter()
        .map(|rollout| rollout.trace.started_at_unix_ms)
        .min()
        .unwrap_or_default();
    let end_unix_ms = workflow_rollouts
        .iter()
        .flat_map(|rollout| rollout.trace.codex_turns.values())
        .filter_map(|turn| turn.execution.ended_at_unix_ms)
        .max()
        .unwrap_or(start_unix_ms);
    let incidence_report = ontocode_rollout_trace::aggregate_workflow_incidence_across_traces(
        &WorkflowIncidenceInput {
            window: WorkflowIncidenceWindow {
                start_unix_ms,
                end_unix_ms,
            },
            rollouts: workflow_rollouts,
            upstream_omitted_rollout_count,
            upstream_omitted_record_count,
            ..WorkflowIncidenceInput::default()
        },
    )?;
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "tool_outcomes": outcome_report,
        "workflow_incidence": incidence_report,
    }))?)
}

fn trace_opportunities_json(bundle_paths: &[PathBuf]) -> anyhow::Result<String> {
    if bundle_paths.is_empty() {
        return Ok(serde_json::to_string_pretty(
            &RoutineOpportunityReport::default(),
        )?);
    }

    let (
        workflow_rollouts,
        _accepted_bundle_paths,
        upstream_omitted_rollout_count,
        upstream_omitted_record_count,
    ) = replay_workflow_rollouts(bundle_paths)?;
    let start_unix_ms = workflow_rollouts
        .iter()
        .map(|rollout| rollout.trace.started_at_unix_ms)
        .min()
        .unwrap_or_default();
    let end_unix_ms = workflow_rollouts
        .iter()
        .flat_map(|rollout| rollout.trace.codex_turns.values())
        .filter_map(|turn| turn.execution.ended_at_unix_ms)
        .max()
        .unwrap_or(start_unix_ms);
    Ok(serde_json::to_string_pretty(
        &aggregate_routine_opportunities(&WorkflowIncidenceInput {
            window: WorkflowIncidenceWindow {
                start_unix_ms,
                end_unix_ms,
            },
            rollouts: workflow_rollouts,
            upstream_omitted_rollout_count,
            upstream_omitted_record_count,
        })?,
    )?)
}

fn replay_workflow_rollouts(
    bundle_paths: &[PathBuf],
) -> anyhow::Result<(Vec<WorkflowIncidenceRollout>, Vec<PathBuf>, usize, usize)> {
    let (record_counts, omitted_rollout_count, omitted_record_count) =
        workflow_bundle_record_counts(bundle_paths)?;
    let mut rollouts = Vec::new();
    let mut accepted_bundle_paths = Vec::new();
    for (path, source_record_count) in bundle_paths.iter().zip(record_counts) {
        let Some(source_record_count) = source_record_count else {
            continue;
        };
        accepted_bundle_paths.push(path.clone());
        rollouts.push(WorkflowIncidenceRollout {
            source_record_count,
            trace: replay_bundle(path)?,
        });
    }
    Ok((
        rollouts,
        accepted_bundle_paths,
        omitted_rollout_count,
        omitted_record_count,
    ))
}

fn workflow_bundle_record_counts(
    bundle_paths: &[PathBuf],
) -> anyhow::Result<(Vec<Option<usize>>, usize, usize)> {
    let mut accepted_record_count: usize = 0;
    let mut accepted_rollout_count = 0;
    let mut omitted_rollout_count = 0;
    let mut omitted_record_count: usize = 0;
    let mut record_counts = Vec::with_capacity(bundle_paths.len());

    for path in bundle_paths {
        let source_record_count = trace_event_record_count(path)?;
        let fits_rollout_cap = accepted_rollout_count < MAX_WORKFLOW_ROLLOUTS;
        let fits_record_cap = accepted_record_count
            .checked_add(source_record_count)
            .is_some_and(|count| count <= MAX_WORKFLOW_RECORDS);
        if fits_rollout_cap && fits_record_cap {
            accepted_rollout_count += 1;
            accepted_record_count += source_record_count;
            record_counts.push(Some(source_record_count));
        } else {
            omitted_rollout_count += 1;
            omitted_record_count = omitted_record_count.saturating_add(source_record_count);
            record_counts.push(None);
        }
    }

    Ok((record_counts, omitted_rollout_count, omitted_record_count))
}

fn trace_event_record_count(bundle_path: &Path) -> anyhow::Result<usize> {
    let event_log = std::fs::File::open(bundle_path.join("trace.jsonl"))?;
    let mut count = 0;
    for line in std::io::BufReader::new(event_log).lines() {
        if !line?.trim().is_empty() {
            count += 1;
        }
    }
    Ok(count)
}

async fn run_debug_prompt_input_command(
    cmd: DebugPromptInputCommand,
    root_config_overrides: CliConfigOverrides,
    interactive: TuiCli,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<()> {
    let loader_overrides = loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?;
    let shared = interactive.shared.into_inner();
    let mut cli_kv_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    if interactive.web_search {
        cli_kv_overrides.push((
            "web_search".to_string(),
            toml::Value::String("live".to_string()),
        ));
    }

    let approval_policy = if shared.dangerously_bypass_approvals_and_sandbox {
        Some(AskForApproval::Never)
    } else {
        interactive.approval_policy.map(Into::into)
    };
    let sandbox_mode = if shared.dangerously_bypass_approvals_and_sandbox {
        Some(ontocode_protocol::config_types::SandboxMode::DangerFullAccess)
    } else {
        shared.sandbox_mode.map(Into::into)
    };
    let overrides = ConfigOverrides {
        model: shared.model,
        approval_policy,
        sandbox_mode,
        cwd: shared.cwd,
        codex_self_exe: arg0_paths.codex_self_exe,
        codex_linux_sandbox_exe: arg0_paths.codex_linux_sandbox_exe,
        main_execve_wrapper_exe: arg0_paths.main_execve_wrapper_exe,
        show_raw_agent_reasoning: shared.oss.then_some(true),
        ephemeral: Some(true),
        bypass_hook_trust: shared.bypass_hook_trust.then_some(true),
        additional_writable_roots: shared.add_dir,
        ..Default::default()
    };
    let config = ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .harness_overrides(overrides)
        .loader_overrides(loader_overrides)
        .build()
        .await?;

    let mut input = shared
        .images
        .into_iter()
        .chain(cmd.images)
        .map(|path| UserInput::LocalImage { path, detail: None })
        .collect::<Vec<_>>();
    if let Some(prompt) = cmd.prompt.or(interactive.prompt) {
        input.push(UserInput::Text {
            text: prompt.replace("\r\n", "\n").replace('\r', "\n"),
            text_elements: Vec::new(),
        });
    }

    let prompt_input = ontocode_core::build_prompt_input(config, input, /*state_db*/ None).await?;
    println!("{}", serde_json::to_string_pretty(&prompt_input)?);

    Ok(())
}

async fn run_debug_models_command(
    cmd: DebugModelsCommand,
    root_config_overrides: CliConfigOverrides,
) -> anyhow::Result<()> {
    let catalog = if cmd.bundled {
        bundled_models_response()?
    } else {
        let cli_overrides = root_config_overrides
            .parse_overrides()
            .map_err(anyhow::Error::msg)?;
        let config = ConfigBuilder::default()
            .cli_overrides(cli_overrides)
            .build()
            .await?;
        let auth_manager =
            AuthManager::shared_from_config(&config, /*enable_codex_api_key_env*/ true).await?;
        let models_manager = build_models_manager(&config, auth_manager);
        models_manager
            .raw_model_catalog(RefreshStrategy::OnlineIfUncached)
            .await
    };

    serde_json::to_writer(std::io::stdout(), &catalog)?;
    println!();
    Ok(())
}

async fn run_debug_clear_memories_command(
    root_config_overrides: &CliConfigOverrides,
) -> anyhow::Result<()> {
    let cli_kv_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .build()
        .await?;

    let memories_path = memories_db_path(config.sqlite_home.as_path());
    let cleared_memories_db =
        StateRuntime::clear_memory_data_in_sqlite_home(config.sqlite_home.as_path()).await?;

    clear_memory_roots_contents(&config.codex_home).await?;

    let mut message = if cleared_memories_db {
        format!("Cleared memory state from {}.", memories_path.display())
    } else {
        format!("No memories db found at {}.", memories_path.display())
    };
    message.push_str(&format!(
        " Cleared memory directories under {}.",
        config.codex_home.display()
    ));

    println!("{message}");

    Ok(())
}

/// Prepend root-level overrides so they have lower precedence than
/// CLI-specific ones specified after the subcommand (if any).
fn prepend_config_flags(
    subcommand_config_overrides: &mut CliConfigOverrides,
    cli_config_overrides: CliConfigOverrides,
) {
    subcommand_config_overrides.prepend_root_overrides(cli_config_overrides);
}

fn reject_remote_mode_for_subcommand(
    remote: Option<&str>,
    remote_auth_token_env: Option<&str>,
    subcommand: &str,
) -> anyhow::Result<()> {
    if let Some(remote) = remote {
        anyhow::bail!(
            "`--remote {remote}` is only supported for interactive TUI commands, not `ontocode {subcommand}`"
        );
    }
    if remote_auth_token_env.is_some() {
        anyhow::bail!(
            "`--remote-auth-token-env` is only supported for interactive TUI commands, not `ontocode {subcommand}`"
        );
    }
    Ok(())
}

fn reject_root_strict_config_for_subcommand(
    strict_config: bool,
    subcommand: &Option<Subcommand>,
) -> anyhow::Result<()> {
    if !strict_config {
        return Ok(());
    }

    match unsupported_subcommand_name_for_strict_config(subcommand) {
        Some(subcommand_name) => {
            reject_strict_config_for_unsupported_subcommand(strict_config, subcommand_name)
        }
        None => Ok(()),
    }
}

/// Return the selected subcommand name when a root-level `--strict-config`
/// flag should be rejected after parsing.
///
/// `--strict-config` is parsed on the root interactive CLI so commands like
/// `ontocode --strict-config` continue to work for the TUI and for wrappers that
/// forward root options into another command shape. Clap will still accept that
/// root flag before the dispatcher knows which subcommand the user selected, so
/// unsupported subcommands need an explicit post-parse reject path.
///
/// `Some(...)` returns the user-facing command name fragment to embed in the
/// rejection error, such as `cloud` or `app-server proxy`. `None` means the
/// selected command is allowed to inherit root `--strict-config`.
fn unsupported_subcommand_name_for_strict_config(
    subcommand: &Option<Subcommand>,
) -> Option<&'static str> {
    match subcommand {
        None
        | Some(Subcommand::Exec(_))
        | Some(Subcommand::Review(_))
        | Some(Subcommand::McpServer(_))
        | Some(Subcommand::ExecServer(_))
        | Some(Subcommand::Resume(_))
        | Some(Subcommand::Archive(_))
        | Some(Subcommand::Unarchive(_))
        | Some(Subcommand::Fork(_))
        | Some(Subcommand::Doctor(_)) => None,
        Some(Subcommand::AppServer(app_server)) if app_server.subcommand.is_none() => None,
        Some(Subcommand::AppServer(app_server)) => {
            Some(app_server_subcommand_name(app_server.subcommand.as_ref()))
        }
        Some(Subcommand::Mcp(_)) => Some("mcp"),
        Some(Subcommand::Plugin(_)) => Some("plugin"),
        Some(Subcommand::Ontograph(_)) => Some("ontograph"),
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        Some(Subcommand::App(_)) => Some("app"),
        Some(Subcommand::Login(_)) => Some("login"),
        Some(Subcommand::Logout(_)) => Some("logout"),
        Some(Subcommand::Completion(_)) => Some("completion"),
        Some(Subcommand::Diagnostics(_)) => Some("diagnostics"),
        Some(Subcommand::Sandbox(_)) => Some("sandbox"),
        Some(Subcommand::Debug(_)) => Some("debug"),
        Some(Subcommand::Execpolicy(_)) => Some("execpolicy"),
        Some(Subcommand::ResponsesApiProxy(_)) => Some("responses-api-proxy"),
        Some(Subcommand::StdioToUds(_)) => Some("stdio-to-uds"),
        Some(Subcommand::Features(_)) => Some("features"),
    }
}

fn reject_strict_config_for_app_server_subcommand(
    strict_config: bool,
    subcommand: Option<&AppServerSubcommand>,
) -> anyhow::Result<()> {
    if subcommand.is_none() {
        return Ok(());
    }
    reject_strict_config_for_unsupported_subcommand(
        strict_config,
        app_server_subcommand_name(subcommand),
    )
}

fn reject_strict_config_for_unsupported_subcommand(
    strict_config: bool,
    subcommand: &str,
) -> anyhow::Result<()> {
    if strict_config {
        anyhow::bail!("`--strict-config` is not supported for `ontocode {subcommand}`");
    }
    Ok(())
}

fn reject_remote_mode_for_app_server_subcommand(
    remote: Option<&str>,
    remote_auth_token_env: Option<&str>,
    subcommand: Option<&AppServerSubcommand>,
) -> anyhow::Result<()> {
    let subcommand_name = app_server_subcommand_name(subcommand);
    reject_remote_mode_for_subcommand(remote, remote_auth_token_env, subcommand_name)
}

fn app_server_subcommand_name(subcommand: Option<&AppServerSubcommand>) -> &'static str {
    match subcommand {
        None => "app-server",
        Some(AppServerSubcommand::Daemon(daemon)) => match daemon.subcommand {
            AppServerDaemonSubcommand::Bootstrap => "app-server daemon bootstrap",
            AppServerDaemonSubcommand::Start => "app-server daemon start",
            AppServerDaemonSubcommand::Restart => "app-server daemon restart",
            AppServerDaemonSubcommand::Stop => "app-server daemon stop",
            AppServerDaemonSubcommand::Version => "app-server daemon version",
            AppServerDaemonSubcommand::PidUpdateLoop => "app-server daemon pid-update-loop",
        },
        Some(AppServerSubcommand::Proxy(_)) => "app-server proxy",
        Some(AppServerSubcommand::GenerateTs(_)) => "app-server generate-ts",
        Some(AppServerSubcommand::GenerateJsonSchema(_)) => "app-server generate-json-schema",
        Some(AppServerSubcommand::GenerateInternalJsonSchema(_)) => {
            "app-server generate-internal-json-schema"
        }
    }
}

async fn print_app_server_daemon_output(command: AppServerLifecycleCommand) -> anyhow::Result<()> {
    let output = ontocode_app_server_daemon::run(command).await?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

fn read_remote_auth_token_from_env_var_with<F>(
    env_var_name: &str,
    get_var: F,
) -> anyhow::Result<String>
where
    F: FnOnce(&str) -> Result<String, std::env::VarError>,
{
    let auth_token = get_var(env_var_name)
        .map_err(|_| anyhow::anyhow!("environment variable `{env_var_name}` is not set"))?;
    let auth_token = auth_token.trim().to_string();
    if auth_token.is_empty() {
        anyhow::bail!("environment variable `{env_var_name}` is empty");
    }
    Ok(auth_token)
}

fn read_remote_auth_token_from_env_var(env_var_name: &str) -> anyhow::Result<String> {
    read_remote_auth_token_from_env_var_with(env_var_name, |name| std::env::var(name))
}

async fn run_interactive_tui(
    mut interactive: TuiCli,
    remote: Option<String>,
    remote_auth_token_env: Option<String>,
    arg0_paths: Arg0DispatchPaths,
) -> std::io::Result<AppExitInfo> {
    if let Some(prompt) = interactive.prompt.take() {
        // Normalize CRLF/CR to LF so CLI-provided text can't leak `\r` into TUI state.
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }

    let terminal_info = ontocode_terminal_detection::terminal_info();
    if terminal_info.name == TerminalName::Dumb {
        if !(std::io::stdin().is_terminal() && std::io::stderr().is_terminal()) {
            return Ok(AppExitInfo::fatal(
                "TERM is set to \"dumb\". Refusing to start the interactive TUI because no terminal is available for a confirmation prompt (stdin/stderr is not a TTY). Run in a supported terminal or unset TERM.",
            ));
        }

        eprintln!(
            "WARNING: TERM is set to \"dumb\". Codex's interactive TUI may not work in this terminal."
        );
        if !confirm("Continue anyway? [y/N]: ")? {
            return Ok(AppExitInfo::fatal(
                "Refusing to start the interactive TUI because TERM is set to \"dumb\". Run in a supported terminal or unset TERM.",
            ));
        }
    }

    let remote_endpoint = match resolve_remote_endpoint(remote, remote_auth_token_env) {
        Ok(remote_endpoint) => remote_endpoint,
        Err(err) if is_remote_auth_usage_error(&err) => {
            return Ok(AppExitInfo::fatal(err.to_string()));
        }
        Err(err) => return Err(err),
    };
    let start_tui = || {
        ontocode_tui::run_main(
            interactive.clone(),
            arg0_paths.clone(),
            ontocode_config::LoaderOverrides::default(),
            remote_endpoint.clone(),
        )
    };
    let mut attempted_repair = false;
    loop {
        let err = match start_tui().await {
            Ok(exit_info) => return Ok(exit_info),
            Err(err) => err,
        };
        let Some(startup_error) = local_state_db::startup_error(&err) else {
            return Err(err);
        };
        if local_state_db::is_locked(startup_error.detail()) {
            local_state_db::print_locked_guidance(startup_error);
            return Ok(AppExitInfo::fatal(startup_error.to_string()));
        }
        if attempted_repair {
            local_state_db::print_diagnostic_guidance(startup_error);
            return Ok(AppExitInfo::fatal(startup_error.to_string()));
        }
        if !local_state_db::confirm_repair(startup_error)? {
            local_state_db::print_diagnostic_guidance(startup_error);
            return Ok(AppExitInfo::fatal(startup_error.to_string()));
        }

        match local_state_db::repair_files(startup_error).await {
            Ok(backups) => local_state_db::print_repair_backups(&backups),
            Err(repair_err) => {
                local_state_db::print_diagnostic_guidance(startup_error);
                return Ok(AppExitInfo::fatal(format!(
                    "failed to repair Codex local data automatically: {repair_err}"
                )));
            }
        }
        attempted_repair = true;
    }
}

fn resolve_remote_endpoint(
    remote: Option<String>,
    remote_auth_token_env: Option<String>,
) -> std::io::Result<Option<ontocode_tui::RemoteAppServerEndpoint>> {
    let mut remote_endpoint = remote
        .as_deref()
        .map(ontocode_tui::resolve_remote_addr)
        .transpose()
        .map_err(std::io::Error::other)?;
    if let Some(remote_auth_token_env) = remote_auth_token_env {
        let Some(endpoint) = remote_endpoint.as_mut() else {
            return Err(std::io::Error::other(
                "`--remote-auth-token-env` requires `--remote`.",
            ));
        };
        if !ontocode_tui::remote_addr_supports_auth_token(endpoint) {
            return Err(std::io::Error::other(
                "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote.",
            ));
        }
        let auth_token = read_remote_auth_token_from_env_var(&remote_auth_token_env)
            .map_err(std::io::Error::other)?;
        let ontocode_tui::RemoteAppServerEndpoint::WebSocket {
            auth_token: slot, ..
        } = endpoint
        else {
            return Err(std::io::Error::other(
                "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote.",
            ));
        };
        *slot = Some(auth_token);
    }
    Ok(remote_endpoint)
}

fn is_remote_auth_usage_error(err: &std::io::Error) -> bool {
    err.to_string()
        .starts_with("`--remote-auth-token-env` requires")
}

fn confirm(prompt: &str) -> std::io::Result<bool> {
    eprintln!("{prompt}");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let answer = input.trim();
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

/// Build the final `TuiCli` for an `ontocode resume` invocation.
fn finalize_resume_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    session_id: Option<String>,
    last: bool,
    show_all: bool,
    include_non_interactive: bool,
    resume_cli: TuiCli,
) -> TuiCli {
    // Start with the parsed interactive CLI so resume shares the same
    // configuration surface area as `ontocode` without additional flags.
    let resume_session_id = session_id;
    interactive.resume_picker = resume_session_id.is_none() && !last;
    interactive.resume_last = last;
    interactive.resume_session_id = resume_session_id;
    interactive.resume_show_all = show_all;
    interactive.resume_include_non_interactive = include_non_interactive;

    // Merge resume-scoped flags and overrides with highest precedence.
    merge_interactive_cli_flags(&mut interactive, resume_cli);

    // Propagate any root-level config overrides (e.g. `-c key=value`).
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);

    interactive
}

/// Build the final `TuiCli` for an `ontocode fork` invocation.
fn finalize_fork_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    session_id: Option<String>,
    last: bool,
    show_all: bool,
    fork_cli: TuiCli,
) -> TuiCli {
    // Start with the parsed interactive CLI so fork shares the same
    // configuration surface area as `ontocode` without additional flags.
    let fork_session_id = session_id;
    interactive.fork_picker = fork_session_id.is_none() && !last;
    interactive.fork_last = last;
    interactive.fork_session_id = fork_session_id;
    interactive.fork_show_all = show_all;

    // Merge fork-scoped flags and overrides with highest precedence.
    merge_interactive_cli_flags(&mut interactive, fork_cli);

    // Propagate any root-level config overrides (e.g. `-c key=value`).
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);

    interactive
}

fn finalize_session_archive_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    archive_cli: SessionArchiveConfigOverrides,
) -> TuiCli {
    let SessionArchiveConfigOverrides {
        shared,
        strict_config,
        config_overrides,
    } = archive_cli;
    interactive.shared.apply_subcommand_overrides(shared);
    if strict_config {
        interactive.strict_config = true;
    }
    interactive
        .config_overrides
        .raw_overrides
        .extend(config_overrides.raw_overrides);
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);
    interactive
}

/// Merge flags provided to runtime wrapper commands so they take precedence over any root-level
/// flags. Only overrides fields explicitly set on the subcommand-scoped CLI. Also appends
/// `-c key=value` overrides with highest precedence.
fn merge_interactive_cli_flags(interactive: &mut TuiCli, subcommand_cli: TuiCli) {
    let TuiCli {
        shared,
        strict_config,
        approval_policy,
        web_search,
        prompt,
        config_overrides,
        ..
    } = subcommand_cli;
    interactive
        .shared
        .apply_subcommand_overrides(shared.into_inner());
    if let Some(approval) = approval_policy {
        interactive.approval_policy = Some(approval);
    }
    if web_search {
        interactive.web_search = true;
    }
    if strict_config {
        interactive.strict_config = true;
    }
    if let Some(prompt) = prompt {
        // Normalize CRLF/CR to LF so CLI-provided text can't leak `\r` into TUI state.
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }

    interactive
        .config_overrides
        .raw_overrides
        .extend(config_overrides.raw_overrides);
}

fn print_completion(cmd: CompletionCommand) {
    let mut app = multitool_command();
    generate(cmd.shell, &mut app, COMMAND_NAME, &mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use ontocode_protocol::ThreadId;
    use ontocode_rollout_trace::ExecutionStatus;
    use ontocode_rollout_trace::RawToolCallRequester;
    use ontocode_rollout_trace::RawTraceEventContext;
    use ontocode_rollout_trace::RawTraceEventPayload;
    use ontocode_rollout_trace::ToolCallKind;
    use ontocode_rollout_trace::ToolCallSummary;
    use ontocode_rollout_trace::ToolFailureCategory;
    use ontocode_rollout_trace::TraceWriter;
    use ontocode_tui::TokenUsage;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn exec_server_remote_auth_accepts_api_key_auth() {
        let auth = CodexAuth::from_api_key("sk-test");

        assert!(is_supported_exec_server_remote_auth(&auth));
    }

    #[test]
    fn exec_server_rejects_removed_agent_identity_auth_flag() {
        let parse_result = MultitoolCli::try_parse_from([
            "codex",
            "exec-server",
            "--remote",
            "https://openai.com/api",
            "--environment-id",
            "environment-id",
            "--use-agent-identity-auth",
        ]);

        assert!(parse_result.is_err());
    }

    #[test]
    fn exec_server_remote_api_key_auth_accepts_https_openai_domains() {
        for base_url in [
            "https://openai.com/api",
            "https://service.openai.com/api",
            "https://openai.org/api",
            "https://service.openai.org/api",
        ] {
            assert!(validate_api_key_remote_host(base_url).is_ok());
        }
    }

    #[test]
    fn exec_server_remote_api_key_auth_accepts_http_loopback() {
        for base_url in [
            "http://localhost:8098/api",
            "http://127.0.0.1:8098/api",
            "http://[::1]:8098/api",
        ] {
            assert!(validate_api_key_remote_host(base_url).is_ok());
        }
    }

    #[test]
    fn exec_server_remote_api_key_auth_rejects_http_openai_domain() {
        for base_url in [
            "http://service.openai.com/api",
            "http://service.openai.org/api",
        ] {
            let error = validate_api_key_remote_host(base_url)
                .expect_err("reject plaintext OpenAI destination");

            assert_eq!(
                error.to_string(),
                "remote exec-server API-key authentication is restricted to HTTPS openai.com and openai.org hosts and subdomains or loopback hosts"
            );
        }
    }

    #[test]
    fn exec_server_remote_api_key_auth_rejects_suffix_spoof() {
        let error = validate_api_key_remote_host("https://service.openai.org.evil.example/api")
            .expect_err("reject suffix spoof");

        assert_eq!(
            error.to_string(),
            "remote exec-server API-key authentication is restricted to HTTPS openai.com and openai.org hosts and subdomains or loopback hosts"
        );
    }

    fn finalize_resume_from_args(args: &[&str]) -> TuiCli {
        let cli = try_parse_multitool_cli_from(args).expect("parse");
        let MultitoolCli {
            interactive,
            config_overrides: root_overrides,
            subcommand,
            feature_toggles: _,
            remote: _,
        } = cli;

        let Subcommand::Resume(ResumeCommand {
            session_id,
            last,
            all,
            include_non_interactive,
            remote: _,
            config_overrides: resume_cli,
        }) = subcommand.expect("resume present")
        else {
            unreachable!()
        };

        finalize_resume_interactive(
            interactive,
            root_overrides,
            session_id,
            last,
            all,
            include_non_interactive,
            resume_cli,
        )
    }

    fn finalize_fork_from_args(args: &[&str]) -> TuiCli {
        let cli = try_parse_multitool_cli_from(args).expect("parse");
        let MultitoolCli {
            interactive,
            config_overrides: root_overrides,
            subcommand,
            feature_toggles: _,
            remote: _,
        } = cli;

        let Subcommand::Fork(ForkCommand {
            session_id,
            last,
            all,
            remote: _,
            config_overrides: fork_cli,
        }) = subcommand.expect("fork present")
        else {
            unreachable!()
        };

        finalize_fork_interactive(interactive, root_overrides, session_id, last, all, fork_cli)
    }

    fn finalize_archive_from_args(args: &[&str]) -> (String, TuiCli, InteractiveRemoteOptions) {
        let cli = try_parse_multitool_cli_from(args).expect("parse");
        let MultitoolCli {
            interactive,
            config_overrides: root_overrides,
            subcommand,
            feature_toggles: _,
            remote: _,
        } = cli;

        let Subcommand::Archive(SessionArchiveCommand {
            target,
            remote,
            config_overrides: archive_cli,
        }) = subcommand.expect("archive present")
        else {
            unreachable!()
        };

        (
            target,
            finalize_session_archive_interactive(interactive, root_overrides, archive_cli),
            remote,
        )
    }

    fn profile_v2_for_args(args: &[&str]) -> anyhow::Result<Option<String>> {
        let cli = try_parse_multitool_cli_from(args).expect("parse");
        let Some(subcommand) = cli.subcommand.as_ref() else {
            return Ok(cli
                .interactive
                .config_profile_v2
                .as_ref()
                .map(std::string::ToString::to_string));
        };
        Ok(profile_v2_for_subcommand(&cli.interactive, subcommand)?.map(ToString::to_string))
    }

    #[test]
    fn profile_v2_is_rejected_for_config_management_subcommands() {
        assert!(profile_v2_for_args(&["codex", "--profile", "work", "features", "list"]).is_err());
    }

    #[test]
    fn profile_v2_is_allowed_for_runtime_subcommands() {
        assert_eq!(
            profile_v2_for_args(&["codex", "--profile", "work", "resume"])
                .expect("resume supports profile-v2")
                .as_deref(),
            Some("work")
        );
        assert_eq!(
            profile_v2_for_args(&["codex", "--profile", "work", "debug", "prompt-input"])
                .expect("debug prompt-input supports profile-v2")
                .as_deref(),
            Some("work")
        );
        assert_eq!(
            profile_v2_for_args(&["codex", "--profile", "work", "mcp", "list"])
                .expect("mcp supports profile-v2")
                .as_deref(),
            Some("work")
        );
        assert_eq!(
            profile_v2_for_args(&["codex", "--profile", "work", "sandbox"])
                .expect("sandbox supports config profile")
                .as_deref(),
            Some("work")
        );
    }

    #[test]
    fn profile_v2_rejects_non_plain_names_at_parse_time() {
        assert!(
            MultitoolCli::try_parse_from(["codex", "--profile", "nested/work", "resume"]).is_err()
        );
    }

    #[test]
    fn exec_resume_last_accepts_prompt_positional() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "exec", "--json", "resume", "--last", "2+2"])
                .expect("parse should succeed");

        let Some(Subcommand::Exec(exec)) = cli.subcommand else {
            panic!("expected exec subcommand");
        };
        let Some(ontocode_exec::Command::Resume(args)) = exec.command else {
            panic!("expected exec resume");
        };

        assert!(args.last);
        assert_eq!(args.session_id, None);
        assert_eq!(args.prompt.as_deref(), Some("2+2"));
    }

    #[test]
    fn exec_resume_accepts_output_flags_after_subcommand() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "exec",
            "resume",
            "session-123",
            "-o",
            "/tmp/resume-output.md",
            "--output-schema",
            "/tmp/schema.json",
            "re-review",
        ])
        .expect("parse should succeed");

        let Some(Subcommand::Exec(exec)) = cli.subcommand else {
            panic!("expected exec subcommand");
        };
        let Some(ontocode_exec::Command::Resume(args)) = exec.command else {
            panic!("expected exec resume");
        };

        assert_eq!(
            exec.last_message_file,
            Some(std::path::PathBuf::from("/tmp/resume-output.md"))
        );
        assert_eq!(
            exec.output_schema,
            Some(std::path::PathBuf::from("/tmp/schema.json"))
        );
        assert_eq!(args.session_id.as_deref(), Some("session-123"));
        assert_eq!(args.prompt.as_deref(), Some("re-review"));
    }

    #[test]
    fn dangerous_bypass_conflicts_with_approval_policy() {
        let err = MultitoolCli::try_parse_from([
            "codex",
            "--dangerously-bypass-approvals-and-sandbox",
            "--ask-for-approval",
            "on-request",
        ])
        .expect_err("conflicting permission flags should be rejected");

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    fn app_server_from_args(args: &[&str]) -> AppServerCommand {
        let cli = MultitoolCli::try_parse_from(args).expect("parse");
        let Subcommand::AppServer(app_server) = cli.subcommand.expect("app-server present") else {
            unreachable!()
        };
        app_server
    }

    fn default_app_server_socket_path() -> AbsolutePathBuf {
        let ontocode_home = find_ontocode_home().expect("ontocode home");
        ontocode_app_server::app_server_control_socket_path(&ontocode_home)
            .expect("default app-server socket path")
    }

    #[test]
    fn debug_prompt_input_parses_prompt_and_images() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "debug",
            "prompt-input",
            "hello",
            "--image",
            "/tmp/a.png,/tmp/b.png",
        ])
        .expect("parse");

        let Some(Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::PromptInput(cmd),
        })) = cli.subcommand
        else {
            panic!("expected debug prompt-input subcommand");
        };

        assert_eq!(cmd.prompt.as_deref(), Some("hello"));
        assert_eq!(
            cmd.images,
            vec![PathBuf::from("/tmp/a.png"), PathBuf::from("/tmp/b.png")]
        );
    }

    #[test]
    fn debug_models_parses_bundled_flag() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "debug", "models", "--bundled"]).expect("parse");

        let Some(Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::Models(cmd),
        })) = cli.subcommand
        else {
            panic!("expected debug models subcommand");
        };

        assert!(cmd.bundled);
    }

    #[test]
    fn debug_trace_stats_parses_bundle_or_root() {
        let bundle =
            MultitoolCli::try_parse_from(["codex", "debug", "trace-stats", "/tmp/trace-one"])
                .expect("parse bundle");
        let root = MultitoolCli::try_parse_from([
            "codex",
            "debug",
            "trace-stats",
            "--root",
            "/tmp/traces",
        ])
        .expect("parse root");

        let Some(Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::TraceStats(bundle),
        })) = bundle.subcommand
        else {
            panic!("expected bundle trace-stats command");
        };
        let Some(Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::TraceStats(root),
        })) = root.subcommand
        else {
            panic!("expected root trace-stats command");
        };
        assert_eq!(bundle.trace_bundle, Some(PathBuf::from("/tmp/trace-one")));
        assert_eq!(bundle.root, None);
        assert_eq!(root.trace_bundle, None);
        assert_eq!(root.root, Some(PathBuf::from("/tmp/traces")));
        assert!(
            MultitoolCli::try_parse_from([
                "codex",
                "debug",
                "trace-stats",
                "/tmp/trace-one",
                "--root",
                "/tmp/traces",
            ])
            .is_err()
        );
    }

    #[test]
    fn debug_trace_opportunities_parses_bundle() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "debug",
            "trace-opportunities",
            "/tmp/trace-one",
        ])
        .expect("parse bundle");

        let Some(Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::TraceOpportunities(cmd),
        })) = cli.subcommand
        else {
            panic!("expected trace-opportunities command");
        };
        assert_eq!(cmd.trace_bundle, Some(PathBuf::from("/tmp/trace-one")));
        assert_eq!(cmd.root, None);
    }

    #[test]
    fn trace_stats_reports_one_bundle_deterministically() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        write_trace_bundle(
            temp.path(),
            ToolCallKind::ExecCommand,
            ExecutionStatus::Failed,
            Some(ToolFailureCategory::Unknown),
        )?;

        let paths = vec![temp.path().to_path_buf()];
        let report = trace_stats_json(&paths)?;
        assert_eq!(report, trace_stats_json(&paths)?);
        let report: serde_json::Value = serde_json::from_str(&report)?;
        let tool_outcomes = &report["tool_outcomes"];
        assert_eq!(tool_outcomes["totals"]["total"], 1);
        assert_eq!(tool_outcomes["totals"]["failed"], 1);
        assert_eq!(tool_outcomes["totals"]["unknown"], 1);
        assert_eq!(tool_outcomes["by_kind"][0]["kind"]["type"], "exec_command");
        assert_eq!(tool_outcomes["unknown_failure_rate"], 1.0);
        let workflow_incidence = &report["workflow_incidence"];
        assert_eq!(workflow_incidence["schema_version"], 1);
        assert!(workflow_incidence["partial"].as_bool().unwrap_or(false));
        assert!(workflow_incidence["rows"].is_array());
        assert_eq!(workflow_incidence["inspected_record_count"], 4);
        assert!(!report.to_string().contains("INPUT_PREVIEW"));
        Ok(())
    }

    #[test]
    fn trace_stats_rolls_up_sorted_direct_trace_children() -> anyhow::Result<()> {
        let root = TempDir::new()?;
        write_trace_bundle(
            &root.path().join("trace-z"),
            ToolCallKind::Web,
            ExecutionStatus::Cancelled,
            None,
        )?;
        write_trace_bundle(
            &root.path().join("trace-a"),
            ToolCallKind::ApplyPatch,
            ExecutionStatus::Completed,
            None,
        )?;
        std::fs::create_dir(root.path().join("ignored"))?;

        let paths = trace_bundle_paths(root.path())?;
        assert_eq!(
            paths,
            vec![root.path().join("trace-a"), root.path().join("trace-z")]
        );
        let report: serde_json::Value = serde_json::from_str(&trace_stats_json(&paths)?)?;
        let tool_outcomes = &report["tool_outcomes"];
        assert_eq!(tool_outcomes["totals"]["total"], 2);
        assert_eq!(tool_outcomes["totals"]["completed"], 1);
        assert_eq!(tool_outcomes["totals"]["cancelled"], 1);
        assert_eq!(tool_outcomes["cancellation_rate"], 0.5);
        assert_eq!(tool_outcomes["by_kind"][0]["kind"]["type"], "apply_patch");
        assert_eq!(tool_outcomes["by_kind"][1]["kind"]["type"], "web");
        Ok(())
    }

    #[test]
    fn trace_stats_reports_empty_root_with_default_reports() -> anyhow::Result<()> {
        let root = TempDir::new()?;
        let paths = trace_bundle_paths(root.path())?;
        assert!(paths.is_empty());

        let report_str = trace_stats_json(&paths)?;
        let report: serde_json::Value = serde_json::from_str(&report_str)?;
        assert!(report.get("tool_outcomes").is_some());
        assert!(report.get("workflow_incidence").is_some());

        assert_eq!(report["tool_outcomes"]["totals"]["total"], 0);
        assert_eq!(report["workflow_incidence"]["schema_version"], 1);
        assert_eq!(report["workflow_incidence"]["rows"], serde_json::json!([]));
        assert_eq!(report["workflow_incidence"]["partial"], true);
        Ok(())
    }

    #[test]
    fn trace_opportunities_reports_empty_root() -> anyhow::Result<()> {
        let report: serde_json::Value = serde_json::from_str(&trace_opportunities_json(&[])?)?;

        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["status"], "partial");
        assert_eq!(report["partial"], true);
        assert_eq!(report["rows"], serde_json::json!([]));
        Ok(())
    }

    #[test]
    fn trace_opportunities_replays_non_empty_bundle() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        write_complete_trace_bundle(temp.path(), ToolCallKind::WaitAgent)?;

        let report: serde_json::Value =
            serde_json::from_str(&trace_opportunities_json(&[temp.path().to_path_buf()])?)?;

        assert_eq!(report["status"], "complete");
        assert_eq!(report["partial"], false);
        assert_eq!(report["rows"][0]["family"], "agent_waiting");
        assert_eq!(report["rows"][0]["count"], 1);
        assert_eq!(report["rows"][0]["denominator_count"], 1);
        assert_eq!(report["rows"][0]["confidence"], "exact");
        Ok(())
    }

    #[test]
    fn workflow_preflight_omits_oversized_invalid_bundle_without_replay() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let valid_bundle = temp.path().join("valid");
        write_trace_bundle(
            &valid_bundle,
            ToolCallKind::ExecCommand,
            ExecutionStatus::Completed,
            None,
        )?;
        let invalid_bundle = temp.path().join("invalid-oversized");
        std::fs::create_dir(&invalid_bundle)?;
        let records = (0..=MAX_WORKFLOW_RECORDS)
            .map(|_| "{}\n")
            .collect::<String>();
        std::fs::write(invalid_bundle.join("trace.jsonl"), records)?;

        let (rollouts, accepted_bundle_paths, omitted_rollouts, omitted_records) =
            replay_workflow_rollouts(&[valid_bundle.clone(), invalid_bundle])?;

        assert_eq!(rollouts.len(), 1);
        assert_eq!(accepted_bundle_paths, vec![valid_bundle.clone()]);
        assert_eq!(omitted_rollouts, 1);
        assert_eq!(omitted_records, MAX_WORKFLOW_RECORDS + 1);
        let report = ontocode_rollout_trace::aggregate_workflow_incidence_across_traces(
            &WorkflowIncidenceInput {
                window: WorkflowIncidenceWindow {
                    start_unix_ms: 0,
                    end_unix_ms: 100,
                },
                rollouts,
                upstream_omitted_rollout_count: omitted_rollouts,
                upstream_omitted_record_count: omitted_records,
                ..WorkflowIncidenceInput::default()
            },
        )?;
        assert_eq!(report.inspected_rollout_count, Some(1));
        assert_eq!(report.omitted_rollout_count, Some(1));
        let report: serde_json::Value = serde_json::from_str(&trace_stats_json(&[
            valid_bundle,
            temp.path().join("invalid-oversized"),
        ])?)?;
        assert_eq!(report["tool_outcomes"]["totals"]["total"], 1);
        assert_eq!(report["workflow_incidence"]["partial"], true);
        assert_eq!(report["workflow_incidence"]["truncated"], true);
        assert_eq!(report["workflow_incidence"]["inspected_rollout_count"], 1);
        assert_eq!(report["workflow_incidence"]["omitted_rollout_count"], 1);
        Ok(())
    }

    fn write_trace_bundle(
        path: &Path,
        kind: ToolCallKind,
        status: ExecutionStatus,
        failure_category: Option<ToolFailureCategory>,
    ) -> anyhow::Result<()> {
        let writer = TraceWriter::create(
            path,
            "trace-1".to_string(),
            "rollout-1".to_string(),
            "thread-root".to_string(),
        )?;
        writer.append(RawTraceEventPayload::ThreadStarted {
            thread_id: "thread-root".to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: None,
        })?;
        writer.append(RawTraceEventPayload::CodexTurnStarted {
            codex_turn_id: "turn-1".to_string(),
            thread_id: "thread-root".to_string(),
        })?;
        let context = RawTraceEventContext {
            thread_id: Some("thread-root".to_string()),
            codex_turn_id: Some("turn-1".to_string()),
        };
        writer.append_with_context(
            context.clone(),
            RawTraceEventPayload::ToolCallStarted {
                tool_call_id: "tool-1".to_string(),
                model_visible_call_id: None,
                code_mode_runtime_tool_id: None,
                requester: RawToolCallRequester::Model,
                kind,
                summary: ToolCallSummary::Generic {
                    label: "test".to_string(),
                    input_preview: None,
                    output_preview: None,
                },
                invocation_payload: None,
            },
        )?;
        writer.append_with_context(
            context,
            RawTraceEventPayload::ToolCallEnded {
                tool_call_id: "tool-1".to_string(),
                status,
                result_payload: None,
                failure_category,
            },
        )?;
        Ok(())
    }

    fn write_complete_trace_bundle(path: &Path, kind: ToolCallKind) -> anyhow::Result<()> {
        let writer = TraceWriter::create(
            path,
            "trace-1".to_string(),
            "rollout-1".to_string(),
            "thread-root".to_string(),
        )?;
        writer.append(RawTraceEventPayload::ThreadStarted {
            thread_id: "thread-root".to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: None,
        })?;
        writer.append(RawTraceEventPayload::CodexTurnStarted {
            codex_turn_id: "turn-1".to_string(),
            thread_id: "thread-root".to_string(),
        })?;
        let context = RawTraceEventContext {
            thread_id: Some("thread-root".to_string()),
            codex_turn_id: Some("turn-1".to_string()),
        };
        writer.append_with_context(
            context.clone(),
            RawTraceEventPayload::ToolCallStarted {
                tool_call_id: "tool-1".to_string(),
                model_visible_call_id: None,
                code_mode_runtime_tool_id: None,
                requester: RawToolCallRequester::Model,
                kind,
                summary: ToolCallSummary::Generic {
                    label: "test".to_string(),
                    input_preview: None,
                    output_preview: None,
                },
                invocation_payload: None,
            },
        )?;
        writer.append_with_context(
            context,
            RawTraceEventPayload::ToolCallEnded {
                tool_call_id: "tool-1".to_string(),
                status: ExecutionStatus::Completed,
                result_payload: None,
                failure_category: None,
            },
        )?;
        writer.append(RawTraceEventPayload::CodexTurnEnded {
            codex_turn_id: "turn-1".to_string(),
            status: ExecutionStatus::Completed,
        })?;
        Ok(())
    }

    #[test]
    fn responses_subcommand_is_not_registered() {
        let command = multitool_command();
        assert!(
            command
                .get_subcommands()
                .all(|subcommand| subcommand.get_name() != "responses")
        );
    }

    fn help_from_args(args: &[&str]) -> String {
        let err = try_parse_multitool_cli_from(args).expect_err("help should short-circuit");
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        err.to_string()
    }

    #[test]
    fn plugin_marketplace_help_uses_ontocode_namespace_for_legacy_arg0() {
        let help = help_from_args(&["codex", "plugin", "marketplace", "--help"]);
        assert!(
            help.contains("Usage: ontocode plugin marketplace [OPTIONS] <COMMAND>"),
            "{help}"
        );

        for (subcommand, usage) in [
            ("add", "Usage: ontocode plugin marketplace add"),
            ("list", "Usage: ontocode plugin marketplace list"),
            ("upgrade", "Usage: ontocode plugin marketplace upgrade"),
            ("remove", "Usage: ontocode plugin marketplace remove"),
        ] {
            let help = help_from_args(&["codex", "plugin", "marketplace", subcommand, "--help"]);
            assert!(help.contains(usage), "{help}");
        }
    }

    #[test]
    fn plugin_marketplace_help_uses_ontocode_namespace() {
        let help = help_from_args(&["ontocode", "plugin", "marketplace", "--help"]);
        assert!(
            help.contains("Usage: ontocode plugin marketplace [OPTIONS] <COMMAND>"),
            "{help}"
        );
    }

    #[test]
    fn plugin_marketplace_add_parses_under_plugin() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "plugin", "marketplace", "add", "owner/repo"])
                .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
    }

    #[test]
    fn plugin_marketplace_upgrade_parses_under_plugin() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "plugin", "marketplace", "upgrade", "debug"])
                .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
    }

    #[test]
    fn plugin_add_parses_under_plugin() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "plugin",
            "add",
            "sample",
            "--marketplace",
            "debug",
        ])
        .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
    }

    #[test]
    fn plugin_list_parses_under_plugin() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "plugin", "list", "--marketplace", "debug"])
                .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
    }

    #[test]
    fn plugin_remove_parses_under_plugin() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "plugin",
            "remove",
            "sample",
            "--marketplace",
            "debug",
        ])
        .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
    }

    #[test]
    fn update_is_not_a_subcommand() {
        for args in [
            vec!["ontocode", "update"],
            vec!["ontocode", "--model", "test-model", "update"],
            vec!["ontocode", "--", "update"],
        ] {
            let err = try_parse_multitool_cli_from(args)
                .expect_err("update should not parse as an interactive prompt");

            assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
            assert!(err.to_string().contains("unrecognized subcommand 'update'"));
        }
    }

    #[test]
    fn ontograph_km_import_subcommand_is_registered() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "ontograph",
            "km",
            "import",
            ".ontocode/ontograph/km/example",
            "--json",
        ])
        .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Ontograph(_))));
    }

    #[test]
    fn archive_merges_scoped_tui_flags() {
        let (target, interactive, remote) = finalize_archive_from_args(
            [
                "codex",
                "-C",
                "/root",
                "archive",
                "--remote",
                "unix://archive.sock",
                "--strict-config",
                "--dangerously-bypass-hook-trust",
                "-m",
                "gpt-5.1-test",
                "-p",
                "work",
                "-C",
                "/archive",
                "my-thread",
            ]
            .as_ref(),
        );

        assert_eq!(target, "my-thread");
        assert_eq!(remote.remote.as_deref(), Some("unix://archive.sock"));
        assert_eq!(interactive.model.as_deref(), Some("gpt-5.1-test"));
        assert_eq!(interactive.config_profile_v2.as_deref(), Some("work"));
        assert_eq!(
            interactive.cwd.as_deref(),
            Some(std::path::Path::new("/archive"))
        );
        assert!(interactive.strict_config);
        assert!(interactive.bypass_hook_trust);
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn sandbox_parses_permissions_profile() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "sandbox",
            "--permissions-profile",
            ":workspace",
            "--",
            "echo",
        ])
        .expect("parse");

        let Some(Subcommand::Sandbox(command)) = cli.subcommand else {
            panic!("expected sandbox command");
        };

        assert_eq!(command.permissions_profile.as_deref(), Some(":workspace"));
        assert_eq!(command.command, vec!["echo"]);
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn sandbox_parses_config_profile() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "sandbox", "--profile", "work", "--", "echo"])
                .expect("parse");

        let Some(Subcommand::Sandbox(command)) = cli.subcommand else {
            panic!("expected sandbox command");
        };

        assert_eq!(command.config_profile.as_deref(), Some("work"));
        assert_eq!(command.command, vec!["echo"]);
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn sandbox_rejects_explicit_profile_controls_without_profile() {
        let err = MultitoolCli::try_parse_from(["codex", "sandbox", "-C", "/tmp"])
            .expect_err("parse should fail");

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn plugin_marketplace_remove_parses_under_plugin() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "plugin", "marketplace", "remove", "debug"])
                .expect("parse");

        assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
    }

    #[test]
    fn marketplace_no_longer_parses_at_top_level() {
        assert!(
            MultitoolCli::command()
                .find_subcommand("marketplace")
                .is_none()
        );
        let add_result =
            MultitoolCli::try_parse_from(["codex", "marketplace", "add", "owner/repo"]);
        assert!(add_result.is_err());

        let upgrade_result =
            MultitoolCli::try_parse_from(["codex", "marketplace", "upgrade", "debug"]);
        assert!(upgrade_result.is_err());

        let remove_result =
            MultitoolCli::try_parse_from(["codex", "marketplace", "remove", "debug"]);
        assert!(remove_result.is_err());
    }

    #[test]
    fn full_auto_no_longer_parses_at_top_level() {
        let result = MultitoolCli::try_parse_from(["codex", "--full-auto"]);

        assert!(result.is_err());
    }

    #[test]
    fn exec_full_auto_reports_migration_path() {
        let cli = MultitoolCli::try_parse_from(["codex", "exec", "--full-auto", "summarize"])
            .expect("exec should accept removed flag long enough to report a migration path");
        let Some(Subcommand::Exec(exec)) = cli.subcommand else {
            panic!("expected exec subcommand");
        };

        assert_eq!(
            exec.removed_full_auto_warning(),
            Some("warning: `--full-auto` is deprecated; use `--sandbox workspace-write` instead.")
        );
    }

    #[test]
    fn sandbox_full_auto_no_longer_parses() {
        let result = MultitoolCli::try_parse_from(["codex", "sandbox", "--full-auto", "--"]);

        assert!(result.is_err());
    }

    fn sample_exit_info(conversation_id: Option<&str>, thread_name: Option<&str>) -> AppExitInfo {
        let token_usage = TokenUsage {
            output_tokens: 2,
            total_tokens: 2,
            ..Default::default()
        };
        AppExitInfo {
            token_usage,
            thread_id: conversation_id
                .map(ThreadId::from_string)
                .map(Result::unwrap),
            thread_name: thread_name.map(str::to_string),
            exit_reason: ExitReason::UserRequested,
        }
    }

    #[test]
    fn format_exit_messages_skips_zero_usage() {
        let exit_info = AppExitInfo {
            token_usage: TokenUsage::default(),
            thread_id: None,
            thread_name: None,
            exit_reason: ExitReason::UserRequested,
        };
        let lines = format_exit_messages(exit_info, /*color_enabled*/ false);
        assert!(lines.is_empty());
    }

    #[test]
    fn format_exit_messages_includes_resume_hint_without_color() {
        let exit_info = sample_exit_info(
            Some("123e4567-e89b-12d3-a456-426614174000"),
            /*thread_name*/ None,
        );
        let lines = format_exit_messages(exit_info, /*color_enabled*/ false);
        assert_eq!(
            lines,
            vec![
                "Token usage: total=2 input=0 output=2".to_string(),
                "To continue this session, run ontocode resume 123e4567-e89b-12d3-a456-426614174000"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn format_exit_messages_applies_color_when_enabled() {
        let exit_info = sample_exit_info(
            Some("123e4567-e89b-12d3-a456-426614174000"),
            /*thread_name*/ None,
        );
        let lines = format_exit_messages(exit_info, /*color_enabled*/ true);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("\u{1b}[36m"));
    }

    #[test]
    fn format_exit_messages_names_picker_item_when_thread_has_name() {
        let exit_info = sample_exit_info(
            Some("123e4567-e89b-12d3-a456-426614174000"),
            Some("my-thread"),
        );
        let lines = format_exit_messages(exit_info, /*color_enabled*/ false);
        assert_eq!(
            lines,
            vec![
                "Token usage: total=2 input=0 output=2".to_string(),
                "To continue this session, run ontocode resume, then select my-thread (123e4567-e89b-12d3-a456-426614174000)".to_string(),
            ]
        );
    }

    #[test]
    fn resume_model_flag_applies_when_no_root_flags() {
        let interactive =
            finalize_resume_from_args(["codex", "resume", "-m", "gpt-5.1-test"].as_ref());

        assert_eq!(interactive.model.as_deref(), Some("gpt-5.1-test"));
        assert!(interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
    }

    #[test]
    fn resume_picker_logic_none_and_not_last() {
        let interactive = finalize_resume_from_args(["codex", "resume"].as_ref());
        assert!(interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
        assert!(!interactive.resume_show_all);
    }

    #[test]
    fn resume_picker_logic_last() {
        let interactive = finalize_resume_from_args(["codex", "resume", "--last"].as_ref());
        assert!(!interactive.resume_picker);
        assert!(interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
        assert!(!interactive.resume_show_all);
    }

    #[test]
    fn resume_picker_logic_with_session_id() {
        let interactive = finalize_resume_from_args(["codex", "resume", "1234"].as_ref());
        assert!(!interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id.as_deref(), Some("1234"));
        assert!(!interactive.resume_show_all);
    }

    #[test]
    fn resume_all_flag_sets_show_all() {
        let interactive = finalize_resume_from_args(["codex", "resume", "--all"].as_ref());
        assert!(interactive.resume_picker);
        assert!(interactive.resume_show_all);
    }

    #[test]
    fn resume_include_non_interactive_flag_sets_source_filter_override() {
        let interactive =
            finalize_resume_from_args(["codex", "resume", "--include-non-interactive"].as_ref());

        assert!(interactive.resume_picker);
        assert!(interactive.resume_include_non_interactive);
    }

    #[test]
    fn resume_merges_option_flags() {
        let interactive = finalize_resume_from_args(
            [
                "codex",
                "resume",
                "sid",
                "--oss",
                "--search",
                "--sandbox",
                "workspace-write",
                "--ask-for-approval",
                "on-request",
                "-m",
                "gpt-5.1-test",
                "-p",
                "my-config",
                "-C",
                "/tmp",
                "--strict-config",
                "-i",
                "/tmp/a.png,/tmp/b.png",
            ]
            .as_ref(),
        );

        assert_eq!(interactive.model.as_deref(), Some("gpt-5.1-test"));
        assert!(interactive.oss);
        assert_eq!(interactive.config_profile_v2.as_deref(), Some("my-config"));
        assert_matches!(
            interactive.sandbox_mode,
            Some(ontocode_utils_cli::SandboxModeCliArg::WorkspaceWrite)
        );
        assert_matches!(
            interactive.approval_policy,
            Some(ontocode_utils_cli::ApprovalModeCliArg::OnRequest)
        );
        assert_eq!(
            interactive.cwd.as_deref(),
            Some(std::path::Path::new("/tmp"))
        );
        assert!(interactive.web_search);
        assert!(interactive.strict_config);
        let has_a = interactive
            .images
            .iter()
            .any(|p| p == std::path::Path::new("/tmp/a.png"));
        let has_b = interactive
            .images
            .iter()
            .any(|p| p == std::path::Path::new("/tmp/b.png"));
        assert!(has_a && has_b);
        assert!(!interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id.as_deref(), Some("sid"));
    }

    #[test]
    fn resume_merges_dangerously_bypass_flag() {
        let interactive = finalize_resume_from_args(
            [
                "codex",
                "resume",
                "--dangerously-bypass-approvals-and-sandbox",
            ]
            .as_ref(),
        );
        assert!(interactive.dangerously_bypass_approvals_and_sandbox);
        assert!(interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
    }

    #[test]
    fn resume_merges_bypass_hook_trust_flag() {
        let interactive = finalize_resume_from_args(
            ["codex", "resume", "--dangerously-bypass-hook-trust"].as_ref(),
        );

        assert!(interactive.bypass_hook_trust);
        assert!(interactive.resume_picker);
        assert!(!interactive.resume_last);
        assert_eq!(interactive.resume_session_id, None);
    }

    #[test]
    fn fork_picker_logic_none_and_not_last() {
        let interactive = finalize_fork_from_args(["codex", "fork"].as_ref());
        assert!(interactive.fork_picker);
        assert!(!interactive.fork_last);
        assert_eq!(interactive.fork_session_id, None);
        assert!(!interactive.fork_show_all);
    }

    #[test]
    fn fork_picker_logic_last() {
        let interactive = finalize_fork_from_args(["codex", "fork", "--last"].as_ref());
        assert!(!interactive.fork_picker);
        assert!(interactive.fork_last);
        assert_eq!(interactive.fork_session_id, None);
        assert!(!interactive.fork_show_all);
    }

    #[test]
    fn fork_picker_logic_with_session_id() {
        let interactive = finalize_fork_from_args(["codex", "fork", "1234"].as_ref());
        assert!(!interactive.fork_picker);
        assert!(!interactive.fork_last);
        assert_eq!(interactive.fork_session_id.as_deref(), Some("1234"));
        assert!(!interactive.fork_show_all);
    }

    #[test]
    fn fork_all_flag_sets_show_all() {
        let interactive = finalize_fork_from_args(["codex", "fork", "--all"].as_ref());
        assert!(interactive.fork_picker);
        assert!(interactive.fork_show_all);
    }

    #[test]
    fn app_server_analytics_default_disabled_without_flag() {
        let app_server = app_server_from_args(["codex", "app-server"].as_ref());
        assert!(!app_server.analytics_default_enabled);
        assert_eq!(
            app_server.listen,
            ontocode_app_server::AppServerTransport::Stdio
        );
    }

    #[test]
    fn app_server_analytics_default_enabled_with_flag() {
        let app_server =
            app_server_from_args(["codex", "app-server", "--analytics-default-enabled"].as_ref());
        assert!(app_server.analytics_default_enabled);
    }

    #[test]
    fn strict_config_parses_for_supported_commands() {
        let cli = MultitoolCli::try_parse_from(["codex", "--strict-config"]).expect("parse");
        assert!(cli.interactive.strict_config);

        let cli = MultitoolCli::try_parse_from(["codex", "mcp-server", "--strict-config"])
            .expect("parse");
        assert_matches!(
            cli.subcommand,
            Some(Subcommand::McpServer(McpServerCommand {
                strict_config: true,
            }))
        );

        let cli =
            MultitoolCli::try_parse_from(["codex", "review", "--strict-config", "--uncommitted"])
                .expect("parse");
        assert_matches!(
            cli.subcommand,
            Some(Subcommand::Review(ReviewCommand {
                strict_config: true,
                ..
            }))
        );

        let cli = MultitoolCli::try_parse_from(["codex", "exec-server", "--strict-config"])
            .expect("parse");
        assert_matches!(
            cli.subcommand,
            Some(Subcommand::ExecServer(ExecServerCommand {
                strict_config: true,
                ..
            }))
        );
    }

    #[test]
    fn root_strict_config_is_supported_for_exec_server() {
        let cli = MultitoolCli::try_parse_from(["codex", "--strict-config", "exec-server"])
            .expect("parse");

        reject_root_strict_config_for_subcommand(cli.interactive.strict_config, &cli.subcommand)
            .expect("exec-server should support root --strict-config");
    }

    #[test]
    fn root_strict_config_is_rejected_for_unsupported_subcommands() {
        let cli = MultitoolCli::try_parse_from(["codex", "--strict-config", "mcp", "list"])
            .expect("parse");
        let err = reject_root_strict_config_for_subcommand(
            cli.interactive.strict_config,
            &cli.subcommand,
        )
        .expect_err("mcp should not support root --strict-config");

        assert_eq!(
            err.to_string(),
            "`--strict-config` is not supported for `ontocode mcp`"
        );
    }

    #[test]
    fn app_server_subcommands_reject_strict_config() {
        let app_server =
            app_server_from_args(["codex", "app-server", "--strict-config", "proxy"].as_ref());
        let err = reject_strict_config_for_app_server_subcommand(
            app_server.strict_config,
            app_server.subcommand.as_ref(),
        )
        .expect_err("app-server proxy should not support --strict-config");

        assert_eq!(
            err.to_string(),
            "`--strict-config` is not supported for `ontocode app-server proxy`"
        );
    }

    #[test]
    fn removed_remote_control_cli_surface_is_rejected() {
        for args in [
            vec!["codex", "app-server", "--remote-control"],
            vec![
                "codex",
                "app-server",
                "daemon",
                "bootstrap",
                "--remote-control",
            ],
            vec!["codex", "app-server", "daemon", "enable-remote-control"],
            vec!["codex", "app-server", "daemon", "disable-remote-control"],
        ] {
            assert!(MultitoolCli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn remote_flag_parses_for_interactive_root() {
        let cli = MultitoolCli::try_parse_from(["codex", "--remote", "unix://codex.sock"])
            .expect("parse");
        assert_eq!(cli.remote.remote.as_deref(), Some("unix://codex.sock"));
    }

    #[test]
    fn remote_auth_token_env_flag_parses_for_interactive_root() {
        let cli = MultitoolCli::try_parse_from([
            "codex",
            "--remote-auth-token-env",
            "CODEX_REMOTE_AUTH_TOKEN",
            "--remote",
            "ws://127.0.0.1:4500",
        ])
        .expect("parse");
        assert_eq!(
            cli.remote.remote_auth_token_env.as_deref(),
            Some("CODEX_REMOTE_AUTH_TOKEN")
        );
    }

    #[test]
    fn remote_flag_parses_for_resume_subcommand() {
        let cli =
            MultitoolCli::try_parse_from(["codex", "resume", "--remote", "unix://codex.sock"])
                .expect("parse");
        let Subcommand::Resume(ResumeCommand { remote, .. }) =
            cli.subcommand.expect("resume present")
        else {
            panic!("expected resume subcommand");
        };
        assert_eq!(remote.remote.as_deref(), Some("unix://codex.sock"));
    }

    #[test]
    fn reject_remote_mode_for_non_interactive_subcommands() {
        let err = reject_remote_mode_for_subcommand(
            Some("127.0.0.1:4500"),
            /*remote_auth_token_env*/ None,
            "exec",
        )
        .expect_err("non-interactive subcommands should reject --remote");
        assert!(
            err.to_string()
                .contains("only supported for interactive TUI commands")
        );
    }

    #[test]
    fn reject_remote_auth_token_env_for_non_interactive_subcommands() {
        let err = reject_remote_mode_for_subcommand(
            /*remote*/ None,
            Some("CODEX_REMOTE_AUTH_TOKEN"),
            "exec",
        )
        .expect_err("non-interactive subcommands should reject --remote-auth-token-env");
        assert!(
            err.to_string()
                .contains("only supported for interactive TUI commands")
        );
    }

    #[test]
    fn reject_remote_auth_token_env_for_app_server_generate_internal_json_schema() {
        let subcommand =
            AppServerSubcommand::GenerateInternalJsonSchema(GenerateInternalJsonSchemaCommand {
                out_dir: PathBuf::from("/tmp/out"),
            });
        let err = reject_remote_mode_for_app_server_subcommand(
            /*remote*/ None,
            Some("CODEX_REMOTE_AUTH_TOKEN"),
            Some(&subcommand),
        )
        .expect_err("non-interactive app-server subcommands should reject --remote-auth-token-env");
        assert!(err.to_string().contains("generate-internal-json-schema"));
    }

    #[test]
    fn read_remote_auth_token_from_env_var_reports_missing_values() {
        let err = read_remote_auth_token_from_env_var_with("CODEX_REMOTE_AUTH_TOKEN", |_| {
            Err(std::env::VarError::NotPresent)
        })
        .expect_err("missing env vars should be rejected");
        assert!(err.to_string().contains("is not set"));
    }

    #[test]
    fn read_remote_auth_token_from_env_var_trims_values() {
        let auth_token =
            read_remote_auth_token_from_env_var_with("CODEX_REMOTE_AUTH_TOKEN", |_| {
                Ok("  bearer-token  ".to_string())
            })
            .expect("env var should parse");
        assert_eq!(auth_token, "bearer-token");
    }

    #[test]
    fn read_remote_auth_token_from_env_var_rejects_empty_values() {
        let err = read_remote_auth_token_from_env_var_with("CODEX_REMOTE_AUTH_TOKEN", |_| {
            Ok(" \n\t ".to_string())
        })
        .expect_err("empty env vars should be rejected");
        assert!(err.to_string().contains("is empty"));
    }

    #[test]
    fn app_server_listen_websocket_url_parses() {
        let app_server = app_server_from_args(
            ["codex", "app-server", "--listen", "ws://127.0.0.1:4500"].as_ref(),
        );
        assert_eq!(
            app_server.listen,
            ontocode_app_server::AppServerTransport::WebSocket {
                bind_address: "127.0.0.1:4500".parse().expect("valid socket address"),
            }
        );
    }

    #[test]
    fn app_server_listen_stdio_url_parses() {
        let app_server =
            app_server_from_args(["codex", "app-server", "--listen", "stdio://"].as_ref());
        assert_eq!(
            app_server.listen,
            ontocode_app_server::AppServerTransport::Stdio
        );
    }

    #[test]
    fn app_server_stdio_flag_parses() {
        let app_server = app_server_from_args(["codex", "app-server", "--stdio"].as_ref());
        assert!(app_server.stdio);
    }

    #[test]
    fn app_server_stdio_flag_conflicts_with_listen() {
        let err = MultitoolCli::try_parse_from([
            "codex",
            "app-server",
            "--stdio",
            "--listen",
            "stdio://",
        ])
        .expect_err("--stdio and --listen should be rejected together");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn app_server_listen_unix_socket_url_parses() {
        let app_server =
            app_server_from_args(["codex", "app-server", "--listen", "unix://"].as_ref());
        assert_eq!(
            app_server.listen,
            ontocode_app_server::AppServerTransport::UnixSocket {
                socket_path: default_app_server_socket_path()
            }
        );
    }

    #[test]
    fn app_server_listen_unix_socket_path_parses() {
        let app_server = app_server_from_args(
            ["codex", "app-server", "--listen", "unix:///tmp/codex.sock"].as_ref(),
        );
        assert_eq!(
            app_server.listen,
            ontocode_app_server::AppServerTransport::UnixSocket {
                socket_path: AbsolutePathBuf::from_absolute_path("/tmp/codex.sock")
                    .expect("absolute path should parse")
            }
        );
    }

    #[test]
    fn app_server_listen_off_parses() {
        let app_server = app_server_from_args(["codex", "app-server", "--listen", "off"].as_ref());
        assert_eq!(
            app_server.listen,
            ontocode_app_server::AppServerTransport::Off
        );
    }

    #[test]
    fn app_server_listen_invalid_url_fails_to_parse() {
        let parse_result =
            MultitoolCli::try_parse_from(["codex", "app-server", "--listen", "http://foo"]);
        assert!(parse_result.is_err());
    }

    #[test]
    fn app_server_proxy_subcommand_parses() {
        let app_server = app_server_from_args(["codex", "app-server", "proxy"].as_ref());
        assert!(matches!(
            app_server.subcommand,
            Some(AppServerSubcommand::Proxy(AppServerProxyCommand {
                socket_path: None
            }))
        ));
    }

    #[test]
    fn app_server_daemon_subcommands_parse() {
        assert!(matches!(
            app_server_from_args(["codex", "app-server", "daemon", "bootstrap"].as_ref())
                .subcommand,
            Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
                subcommand: AppServerDaemonSubcommand::Bootstrap
            }))
        ));
        assert!(matches!(
            app_server_from_args(["codex", "app-server", "daemon", "start"].as_ref()).subcommand,
            Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
                subcommand: AppServerDaemonSubcommand::Start
            }))
        ));
        assert!(matches!(
            app_server_from_args(["codex", "app-server", "daemon", "restart"].as_ref()).subcommand,
            Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
                subcommand: AppServerDaemonSubcommand::Restart
            }))
        ));
        assert!(matches!(
            app_server_from_args(["codex", "app-server", "daemon", "stop"].as_ref()).subcommand,
            Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
                subcommand: AppServerDaemonSubcommand::Stop
            }))
        ));
        assert!(matches!(
            app_server_from_args(["codex", "app-server", "daemon", "version"].as_ref()).subcommand,
            Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
                subcommand: AppServerDaemonSubcommand::Version
            }))
        ));
    }

    #[test]
    fn app_server_proxy_sock_path_parses() {
        let app_server =
            app_server_from_args(["codex", "app-server", "proxy", "--sock", "codex.sock"].as_ref());
        let Some(AppServerSubcommand::Proxy(proxy)) = app_server.subcommand else {
            panic!("expected proxy subcommand");
        };
        assert_eq!(
            proxy.socket_path,
            Some(
                AbsolutePathBuf::relative_to_current_dir("codex.sock")
                    .expect("relative path should resolve")
            )
        );
    }

    #[test]
    fn reject_remote_auth_token_env_for_app_server_proxy() {
        let subcommand = AppServerSubcommand::Proxy(AppServerProxyCommand { socket_path: None });
        let err = reject_remote_mode_for_app_server_subcommand(
            /*remote*/ None,
            Some("CODEX_REMOTE_AUTH_TOKEN"),
            Some(&subcommand),
        )
        .expect_err("app-server proxy should reject --remote-auth-token-env");
        assert!(err.to_string().contains("app-server proxy"));
    }

    #[test]
    fn reject_remote_auth_token_env_for_app_server_version() {
        let subcommand = AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::Version,
        });
        let err = reject_remote_mode_for_app_server_subcommand(
            /*remote*/ None,
            Some("CODEX_REMOTE_AUTH_TOKEN"),
            Some(&subcommand),
        )
        .expect_err("app-server daemon version should reject --remote-auth-token-env");
        assert!(err.to_string().contains("app-server daemon version"));
    }

    #[test]
    fn app_server_capability_token_flags_parse() {
        let app_server = app_server_from_args(
            [
                "codex",
                "app-server",
                "--ws-auth",
                "capability-token",
                "--ws-token-file",
                "/tmp/codex-token",
            ]
            .as_ref(),
        );
        assert_eq!(
            app_server.auth.ws_auth,
            Some(ontocode_app_server::WebsocketAuthCliMode::CapabilityToken)
        );
        assert_eq!(
            app_server.auth.ws_token_file,
            Some(PathBuf::from("/tmp/codex-token"))
        );
    }

    #[test]
    fn app_server_signed_bearer_flags_parse() {
        let app_server = app_server_from_args(
            [
                "codex",
                "app-server",
                "--ws-auth",
                "signed-bearer-token",
                "--ws-shared-secret-file",
                "/tmp/codex-secret",
                "--ws-issuer",
                "issuer",
                "--ws-audience",
                "audience",
                "--ws-max-clock-skew-seconds",
                "9",
            ]
            .as_ref(),
        );
        assert_eq!(
            app_server.auth.ws_auth,
            Some(ontocode_app_server::WebsocketAuthCliMode::SignedBearerToken)
        );
        assert_eq!(
            app_server.auth.ws_shared_secret_file,
            Some(PathBuf::from("/tmp/codex-secret"))
        );
        assert_eq!(app_server.auth.ws_issuer.as_deref(), Some("issuer"));
        assert_eq!(app_server.auth.ws_audience.as_deref(), Some("audience"));
        assert_eq!(app_server.auth.ws_max_clock_skew_seconds, Some(9));
    }

    #[test]
    fn app_server_rejects_removed_insecure_non_loopback_flag() {
        let parse_result = MultitoolCli::try_parse_from([
            "codex",
            "app-server",
            "--allow-unauthenticated-non-loopback-ws",
        ]);
        assert!(parse_result.is_err());
    }

    #[test]
    fn hosted_task_commands_no_longer_parse() {
        for args in [
            ["codex", "cloud", "list"].as_slice(),
            ["codex", "cloud-tasks", "list"].as_slice(),
            ["codex", "apply", "task-id"].as_slice(),
            ["codex", "a", "task-id"].as_slice(),
        ] {
            assert!(
                MultitoolCli::command().find_subcommand(args[1]).is_none(),
                "removed command is still registered: {}",
                args[1]
            );
            assert!(MultitoolCli::try_parse_from(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn features_enable_parses_feature_name() {
        let cli = MultitoolCli::try_parse_from(["codex", "features", "enable", "unified_exec"])
            .expect("parse should succeed");
        let Some(Subcommand::Features(FeaturesCli { sub })) = cli.subcommand else {
            panic!("expected features subcommand");
        };
        let FeaturesSubcommand::Enable(FeatureSetArgs { feature }) = sub else {
            panic!("expected features enable");
        };
        assert_eq!(feature, "unified_exec");
    }

    #[test]
    fn features_disable_parses_feature_name() {
        let cli = MultitoolCli::try_parse_from(["codex", "features", "disable", "shell_tool"])
            .expect("parse should succeed");
        let Some(Subcommand::Features(FeaturesCli { sub })) = cli.subcommand else {
            panic!("expected features subcommand");
        };
        let FeaturesSubcommand::Disable(FeatureSetArgs { feature }) = sub else {
            panic!("expected features disable");
        };
        assert_eq!(feature, "shell_tool");
    }

    #[test]
    fn feature_toggles_known_features_generate_overrides() {
        let toggles = FeatureToggles {
            enable: vec!["web_search_request".to_string()],
            disable: vec!["unified_exec".to_string()],
        };
        let overrides = toggles.to_overrides().expect("valid features");
        assert_eq!(
            overrides,
            vec![
                "features.web_search_request=true".to_string(),
                "features.unified_exec=false".to_string(),
            ]
        );
    }

    #[test]
    fn feature_toggles_accept_legacy_linux_sandbox_flag() {
        let toggles = FeatureToggles {
            enable: vec!["use_linux_sandbox_bwrap".to_string()],
            disable: Vec::new(),
        };
        let overrides = toggles.to_overrides().expect("valid features");
        assert_eq!(
            overrides,
            vec!["features.use_linux_sandbox_bwrap=true".to_string(),]
        );
    }

    #[test]
    fn feature_toggles_accept_removed_image_detail_original_flag() {
        let toggles = FeatureToggles {
            enable: vec!["image_detail_original".to_string()],
            disable: Vec::new(),
        };
        let overrides = toggles.to_overrides().expect("valid features");
        assert_eq!(
            overrides,
            vec!["features.image_detail_original=true".to_string(),]
        );
    }

    #[test]
    fn feature_toggles_unknown_feature_errors() {
        for feature in ["does_not_exist", "remote_control"] {
            for toggles in [
                FeatureToggles {
                    enable: vec![feature.to_string()],
                    disable: Vec::new(),
                },
                FeatureToggles {
                    enable: Vec::new(),
                    disable: vec![feature.to_string()],
                },
            ] {
                let err = toggles
                    .to_overrides()
                    .expect_err("feature should be rejected");
                assert_eq!(err.to_string(), format!("Unknown feature flag: {feature}"));
            }
        }
    }

    #[test]
    fn strict_config_with_unknown_enable_errors() {
        let err = strict_config_feature_toggle_error(["--enable", "does_not_exist"].as_ref());
        assert_eq!(err.to_string(), "Unknown feature flag: does_not_exist");
    }

    #[test]
    fn strict_config_with_unknown_disable_errors() {
        let err = strict_config_feature_toggle_error(["--disable", "does_not_exist"].as_ref());
        assert_eq!(err.to_string(), "Unknown feature flag: does_not_exist");
    }

    #[test]
    fn strict_config_with_compound_enable_errors() {
        let err = strict_config_feature_toggle_error(
            ["--enable", "multi_agent_v2.subagent_usage_hint_text"].as_ref(),
        );
        assert_eq!(
            err.to_string(),
            "Unknown feature flag: multi_agent_v2.subagent_usage_hint_text"
        );
    }

    fn strict_config_feature_toggle_error(args: &[&str]) -> anyhow::Error {
        let cli_args = std::iter::once("codex")
            .chain(std::iter::once("--strict-config"))
            .chain(args.iter().copied());
        let cli = MultitoolCli::try_parse_from(cli_args).expect("parse should succeed");
        assert!(cli.interactive.strict_config);
        cli.feature_toggles
            .to_overrides()
            .expect_err("feature should be rejected")
    }
}
