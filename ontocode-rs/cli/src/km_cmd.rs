use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use ontocode_core::ModelClient;
use ontocode_core::ModelClientOneShotTextRequest;
use ontocode_core::run_model_client_one_shot_text;
use ontocode_git_utils::get_git_repo_root;
use ontocode_ontograph_extension::km_eval::KmEvalResult;
use ontocode_ontograph_extension::km_eval::run_km_eval;
use ontocode_ontograph_extension::km_import::KmImportBatchResult;
use ontocode_ontograph_extension::km_import::KmImportResult;
use ontocode_ontograph_extension::km_import::run_km_import;
use ontocode_ontograph_extension::km_import::run_km_import_batch;
use ontocode_ontograph_extension::km_import::run_km_import_batch_for_project;
use ontocode_ontograph_extension::km_import::run_km_import_for_project;
use ontocode_ontograph_extension::km_librarian::KmLibrarianCatalogRequest;
use ontocode_ontograph_extension::km_librarian::KmLibrarianCatalogResult;
use ontocode_ontograph_extension::km_librarian::KmLibrarianConsolidateRequest;
use ontocode_ontograph_extension::km_librarian::KmLibrarianConsolidateResult;
use ontocode_ontograph_extension::km_librarian::KmLibrarianHealScanRequest;
use ontocode_ontograph_extension::km_librarian::KmLibrarianHealScanResult;
use ontocode_ontograph_extension::km_librarian::KmLibrarianHealthRequest;
use ontocode_ontograph_extension::km_librarian::KmLibrarianHealthResult;
use ontocode_ontograph_extension::km_librarian::KmLibrarianQueryRequest;
use ontocode_ontograph_extension::km_librarian::KmLibrarianQueryResult;
use ontocode_ontograph_extension::km_librarian::KmLibrarianQueryScope;
use ontocode_ontograph_extension::km_librarian::KmLibrarianVersionDiffRequest;
use ontocode_ontograph_extension::km_librarian::KmLibrarianVersionDiffResult;
use ontocode_ontograph_extension::km_librarian::run_km_librarian_catalog;
use ontocode_ontograph_extension::km_librarian::run_km_librarian_consolidate;
use ontocode_ontograph_extension::km_librarian::run_km_librarian_heal_scan;
use ontocode_ontograph_extension::km_librarian::run_km_librarian_health;
use ontocode_ontograph_extension::km_librarian::run_km_librarian_query;
use ontocode_ontograph_extension::km_librarian::run_km_librarian_version_diff;
use ontocode_ontograph_extension::km_llm_client::KmLlmClientConfig;
use ontocode_ontograph_extension::km_llm_client::KmLlmClientError;
use ontocode_ontograph_extension::km_llm_client::KmLlmModelContext;
use ontocode_ontograph_extension::km_llm_client::KmLlmTextModel;
use ontocode_ontograph_extension::km_llm_consolidate::KmLlmConsolidateRequest;
use ontocode_ontograph_extension::km_llm_consolidate::KmLlmConsolidateResult;
use ontocode_ontograph_extension::km_llm_consolidate::run_km_llm_consolidate;
use ontocode_ontograph_extension::km_llm_extract::KmLlmExtractRequest;
use ontocode_ontograph_extension::km_llm_extract::KmLlmExtractResult;
use ontocode_ontograph_extension::km_llm_extract::run_km_llm_extract;
use ontocode_ontograph_extension::km_llm_judge::KmLlmJudgeRequest;
use ontocode_ontograph_extension::km_llm_judge::KmLlmJudgeResult;
use ontocode_ontograph_extension::km_llm_judge::run_km_llm_judge;
use ontocode_ontograph_extension::km_llm_lint::KmLlmLintRequest;
use ontocode_ontograph_extension::km_llm_lint::KmLlmLintResult;
use ontocode_ontograph_extension::km_llm_lint::run_km_llm_lint;
use ontocode_ontograph_extension::km_llm_recipe::KmLlmRecipeRequest;
use ontocode_ontograph_extension::km_llm_recipe::KmLlmRecipeResult;
use ontocode_ontograph_extension::km_llm_recipe::run_km_llm_recipe;
use ontocode_protocol::config_types::ReasoningSummary;
use ontocode_protocol::openai_models::ModelInfo;
use ontocode_protocol::openai_models::ReasoningEffort;
use ontocode_protocol::protocol::SessionSource;

pub struct KmLlmRuntimeConfig {
    pub enabled: bool,
    pub model_context: Option<KmLlmLiveModelContext>,
}

pub struct KmLlmLiveModelContext {
    pub config: KmLlmClientConfig,
    pub model: CoreKmLlmExtractModel,
}

pub struct CoreKmLlmExtractModel {
    model_client: ModelClient,
    model_info: ModelInfo,
    model: String,
    reasoning_effort: Option<ReasoningEffort>,
    reasoning_summary: ReasoningSummary,
    service_tier: Option<String>,
}

impl CoreKmLlmExtractModel {
    pub fn new(
        model_client: ModelClient,
        model_info: ModelInfo,
        model: String,
        reasoning_effort: Option<ReasoningEffort>,
        service_tier: Option<String>,
    ) -> Self {
        let reasoning_summary = model_info.default_reasoning_summary;
        Self {
            model_client,
            model_info,
            model,
            reasoning_effort,
            reasoning_summary,
            service_tier,
        }
    }
}

impl KmLlmTextModel for CoreKmLlmExtractModel {
    fn complete(&self, prompt_text: &str) -> Result<String, KmLlmClientError> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(run_model_client_one_shot_text(
                ModelClientOneShotTextRequest {
                    model_client: &self.model_client,
                    prompt_text: prompt_text.to_string(),
                    model_info: &self.model_info,
                    model: &self.model,
                    reasoning_effort: self.reasoning_effort.clone(),
                    reasoning_summary: self.reasoning_summary,
                    service_tier: self.service_tier.clone(),
                    turn_metadata_header: None,
                    session_source: SessionSource::Cli,
                },
            ))
        })
        .map_err(|err| KmLlmClientError::ModelCall(err.to_string()))
    }
}

#[derive(Debug, Parser)]
pub struct OntographCommand {
    #[clap(subcommand)]
    subcommand: OntographSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum OntographSubcommand {
    /// Manage local Ontograph knowledge modules.
    Km(KmCommand),
}

#[derive(Debug, Parser)]
struct KmCommand {
    #[clap(subcommand)]
    subcommand: KmSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum KmSubcommand {
    /// Import one valid KM package into the project-local active store.
    Import(KmImportCommand),
    /// Import KM packages listed in a batch manifest.
    ImportBatch(KmImportBatchCommand),
    /// Evaluate active KM context coverage against labeled cases.
    Eval(KmEvalCommand),
    /// Run KM librarian maintenance checks.
    Librarian(KmLibrarianCommand),
    /// Emit a deterministic draft KM package from prefixed source paths.
    #[command(name = "draft-extract", visible_alias = "llm-extract")]
    LlmExtract(KmLlmExtractCommand),
    /// Emit a deterministic consolidated draft from existing KM packages.
    #[command(name = "draft-consolidate", visible_alias = "llm-consolidate")]
    LlmConsolidate(KmLlmConsolidateCommand),
    /// Write deterministic recipe.json metadata for an existing KM package.
    #[command(name = "draft-recipe", visible_alias = "llm-recipe")]
    LlmRecipe(KmLlmRecipeCommand),
    /// Write advisory heuristic judge rows separate from KM eval output.
    #[command(name = "draft-judge", visible_alias = "llm-judge")]
    LlmJudge(KmLlmJudgeCommand),
    /// Write deterministic advisory lint notes under okf/ after health checks.
    #[command(name = "draft-lint", visible_alias = "llm-lint")]
    LlmLint(KmLlmLintCommand),
}

#[derive(Debug, Parser)]
struct KmImportCommand {
    /// KM package folder to import.
    folder: PathBuf,

    /// Print the imported record as JSON.
    #[arg(long)]
    json: bool,

    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct KmImportBatchCommand {
    /// Batch directory containing manifest.jsonl.
    batch_dir: PathBuf,

    /// Resume from progress.jsonl.
    #[arg(long)]
    resume: bool,

    /// Print the batch summary as JSON.
    #[arg(long)]
    json: bool,

    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct KmEvalCommand {
    /// Active KM id to evaluate.
    name: String,

    /// Override the cases directory.
    #[arg(long, value_name = "DIR")]
    cases: Option<PathBuf>,

    /// Print the eval summary as JSON.
    #[arg(long)]
    json: bool,

    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct KmLibrarianCommand {
    #[clap(subcommand)]
    subcommand: KmLibrarianSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum KmLibrarianSubcommand {
    /// Check active and obsolete KM cache health.
    Health(KmLibrarianHealthCommand),
    /// List active KM modules in the catalog.
    Catalog(KmLibrarianCatalogCommand),
    /// Query active KM modules with deterministic local/global candidate ranking.
    Query(KmLibrarianQueryCommand),
    /// Scan cached packages for invalid packages and conflict records.
    #[command(name = "heal-scan")]
    HealScan(KmLibrarianHealScanCommand),
    /// Consolidate KM packages and run report-only librarian checks.
    Consolidate(KmLibrarianConsolidateCommand),
    /// Compare two retained versions of one KM module (read-only snapshot diff).
    #[command(name = "version-diff")]
    VersionDiff(KmLibrarianVersionDiffCommand),
}

#[derive(Debug, Parser)]
struct KmLibrarianVersionDiffCommand {
    /// KM module id to compare.
    module_id: String,

    /// Earlier retained version.
    left_version: String,

    /// Later retained version.
    right_version: String,

    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,

    /// Print the version diff as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLibrarianCatalogCommand {
    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,

    /// Print the catalog list as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLibrarianHealthCommand {
    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,

    /// Print the health report as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLibrarianQueryCommand {
    /// Query terms to match against active KM metadata.
    query: String,

    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,

    /// Maximum candidates to print. The librarian enforces a hard cap.
    #[arg(long, default_value_t = 10)]
    limit: usize,

    /// Print the ranked candidates as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLibrarianHealScanCommand {
    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,

    /// Print the heal scan report as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLibrarianConsolidateCommand {
    /// Input KM package directory. Repeat for multiple packages.
    #[arg(long = "input", value_name = "DIR")]
    inputs: Vec<PathBuf>,

    /// Reserved underscore output directory for the consolidated package.
    #[arg(long, value_name = "DIR")]
    output: PathBuf,

    /// Override the active store root.
    #[arg(long, value_name = "DIR")]
    store_root: Option<PathBuf>,

    /// Print the consolidate summary as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLlmExtractCommand {
    /// Reserved underscore output directory for the draft package.
    #[arg(long, value_name = "DIR")]
    output: PathBuf,

    /// Source path with bronze: or silver: prefix. Repeat for multiple sources.
    #[arg(long = "source", value_name = "PREFIX:PATH")]
    sources: Vec<String>,

    /// Project root used to resolve source paths.
    #[arg(long, value_name = "DIR")]
    project_root: Option<PathBuf>,

    /// Print the extraction summary as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLlmConsolidateCommand {
    /// Reserved underscore output directory for the consolidated package.
    #[arg(long, value_name = "DIR")]
    output: PathBuf,

    /// Input KM package directory. Repeat for multiple packages.
    #[arg(long = "input", value_name = "DIR")]
    inputs: Vec<PathBuf>,

    /// Print the consolidation summary as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLlmRecipeCommand {
    /// KM package directory to receive recipe.json.
    package_dir: PathBuf,

    /// Generated package output path. Repeat for multiple outputs.
    #[arg(long = "generated-output", value_name = "PATH")]
    generated_outputs: Vec<String>,

    /// Print the recipe summary as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLlmJudgeCommand {
    /// KM root containing _eval/ and _store/.
    #[arg(long, value_name = "DIR")]
    km_root: PathBuf,

    /// Optional KM package directory used to generate _eval/suite cases.
    #[arg(long, value_name = "DIR")]
    package: Option<PathBuf>,

    /// Print the judge summary as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct KmLlmLintCommand {
    /// KM package directory to lint.
    package_dir: PathBuf,

    /// Print the lint summary as JSON.
    #[arg(long)]
    json: bool,
}

const KM_AUTHORING_DISABLED_MESSAGE: &str = "Ontograph KM authoring is disabled; set [ontograph.km.llm].enabled = true to run draft authoring commands";

pub fn run(cmd: OntographCommand, km_llm_runtime: KmLlmRuntimeConfig) -> anyhow::Result<()> {
    match cmd.subcommand {
        OntographSubcommand::Km(command) => run_km(command, km_llm_runtime),
    }
}

fn run_km(cmd: KmCommand, km_llm_runtime: KmLlmRuntimeConfig) -> anyhow::Result<()> {
    match cmd.subcommand {
        KmSubcommand::Import(command) => run_import(command),
        KmSubcommand::ImportBatch(command) => run_import_batch(command),
        KmSubcommand::Eval(command) => run_eval(command),
        KmSubcommand::Librarian(command) => run_librarian(command),
        KmSubcommand::LlmExtract(command) => {
            ensure_km_llm_enabled(km_llm_runtime.enabled)?;
            run_llm_extract(command, km_llm_runtime.model_context)
        }
        KmSubcommand::LlmConsolidate(command) => {
            ensure_km_llm_enabled(km_llm_runtime.enabled)?;
            run_llm_consolidate(command, km_llm_runtime.model_context)
        }
        KmSubcommand::LlmRecipe(command) => {
            ensure_km_llm_enabled(km_llm_runtime.enabled)?;
            run_llm_recipe(command, km_llm_runtime.model_context)
        }
        KmSubcommand::LlmJudge(command) => {
            ensure_km_llm_enabled(km_llm_runtime.enabled)?;
            run_llm_judge(command, km_llm_runtime.model_context)
        }
        KmSubcommand::LlmLint(command) => {
            ensure_km_llm_enabled(km_llm_runtime.enabled)?;
            run_llm_lint(command, km_llm_runtime.model_context)
        }
    }
}

fn ensure_km_llm_enabled(enabled: bool) -> anyhow::Result<()> {
    anyhow::ensure!(enabled, KM_AUTHORING_DISABLED_MESSAGE);
    Ok(())
}

fn canonical_project_root() -> anyhow::Result<PathBuf> {
    optional_project_root()?.context("KM commands require a Git project root")
}

pub(crate) fn optional_project_root() -> anyhow::Result<Option<PathBuf>> {
    let cwd = std::env::current_dir()?;
    get_git_repo_root(&cwd)
        .map(|root| {
            root.canonicalize()
                .context("failed to resolve canonical project root")
        })
        .transpose()
}

fn run_import(cmd: KmImportCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = match optional_project_root()? {
        Some(project_scope) => run_km_import_for_project(&cmd.folder, &store_root, &project_scope)?,
        None => run_km_import(&cmd.folder, &store_root)?,
    };
    print_import_result(&result, cmd.json)
}

fn run_import_batch(cmd: KmImportBatchCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = match optional_project_root()? {
        Some(project_scope) => run_km_import_batch_for_project(
            &cmd.batch_dir,
            &store_root,
            cmd.resume,
            &project_scope,
        )?,
        None => run_km_import_batch(&cmd.batch_dir, &store_root, cmd.resume)?,
    };
    print_import_batch_result(&result, cmd.json)
}

fn run_eval(cmd: KmEvalCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = run_km_eval(&cmd.name, &store_root, cmd.cases.as_deref())?;
    print_eval_result(&result, cmd.json)
}

fn run_librarian(cmd: KmLibrarianCommand) -> anyhow::Result<()> {
    match cmd.subcommand {
        KmLibrarianSubcommand::Health(command) => run_librarian_health(command),
        KmLibrarianSubcommand::Catalog(command) => run_librarian_catalog(command),
        KmLibrarianSubcommand::VersionDiff(command) => run_librarian_version_diff(command),
        KmLibrarianSubcommand::Query(command) => run_librarian_query(command),
        KmLibrarianSubcommand::HealScan(command) => run_librarian_heal_scan(command),
        KmLibrarianSubcommand::Consolidate(command) => run_librarian_consolidate(command),
    }
}

fn run_librarian_catalog(cmd: KmLibrarianCatalogCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = run_km_librarian_catalog(&KmLibrarianCatalogRequest {
        km_store_root: store_root,
    })?;
    print_librarian_catalog_result(&result, cmd.json)
}

fn run_librarian_health(cmd: KmLibrarianHealthCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })?;
    print_librarian_health_result(&result, cmd.json)
}

fn run_librarian_version_diff(cmd: KmLibrarianVersionDiffCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root,
        module_id: cmd.module_id,
        left_version: cmd.left_version,
        right_version: cmd.right_version,
    })?;
    print_librarian_version_diff_result(&result, cmd.json)
}

fn run_librarian_query(cmd: KmLibrarianQueryCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: cmd.query,
        max_candidates: cmd.limit,
    })?;
    print_librarian_query_result(&result, cmd.json)
}

fn run_librarian_heal_scan(cmd: KmLibrarianHealScanCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let result = run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: store_root,
    })?;
    print_librarian_heal_scan_result(&result, cmd.json)
}

fn run_librarian_consolidate(cmd: KmLibrarianConsolidateCommand) -> anyhow::Result<()> {
    let store_root = store_root_or_default(cmd.store_root)?;
    let project_root = canonical_project_root()?;
    let result = run_km_librarian_consolidate(&KmLibrarianConsolidateRequest {
        input_dirs: cmd.inputs,
        output_dir: cmd.output,
        km_store_root: store_root,
        project_root,
    })?;
    print_librarian_consolidate_result(&result, cmd.json)
}

fn run_llm_extract(
    cmd: KmLlmExtractCommand,
    model_context: Option<KmLlmLiveModelContext>,
) -> anyhow::Result<()> {
    let project_root = match cmd.project_root {
        Some(path) => path,
        None => std::env::current_dir().context("failed to read current directory")?,
    };
    let model_context = model_context.as_ref().map(|ctx| KmLlmModelContext {
        config: &ctx.config,
        model: &ctx.model,
    });
    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: cmd.sources,
            output_dir: cmd.output,
            project_root,
        },
        model_context,
    )?;
    print_llm_extract_result(&result, cmd.json)
}

fn run_llm_consolidate(
    cmd: KmLlmConsolidateCommand,
    model_context: Option<KmLlmLiveModelContext>,
) -> anyhow::Result<()> {
    let project_root = canonical_project_root()?;
    let model_context = model_context.as_ref().map(|ctx| KmLlmModelContext {
        config: &ctx.config,
        model: &ctx.model,
    });
    let result = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: cmd.inputs,
            output_dir: cmd.output,
            project_root,
        },
        model_context,
    )?;
    print_llm_consolidate_result(&result, cmd.json)
}

fn run_llm_recipe(
    cmd: KmLlmRecipeCommand,
    model_context: Option<KmLlmLiveModelContext>,
) -> anyhow::Result<()> {
    let project_root = canonical_project_root()?;
    let model_context = model_context.as_ref().map(|ctx| KmLlmModelContext {
        config: &ctx.config,
        model: &ctx.model,
    });
    let result = run_km_llm_recipe(
        &KmLlmRecipeRequest {
            package_dir: cmd.package_dir,
            generated_outputs: cmd.generated_outputs,
            project_root,
        },
        model_context,
    )?;
    print_llm_recipe_result(&result, cmd.json)
}

fn run_llm_judge(
    cmd: KmLlmJudgeCommand,
    model_context: Option<KmLlmLiveModelContext>,
) -> anyhow::Result<()> {
    let model_context = model_context.as_ref().map(|ctx| KmLlmModelContext {
        config: &ctx.config,
        model: &ctx.model,
    });
    let result = run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: cmd.km_root,
            package_dir: cmd.package,
        },
        model_context,
    )?;
    print_llm_judge_result(&result, cmd.json)
}

fn run_llm_lint(
    cmd: KmLlmLintCommand,
    model_context: Option<KmLlmLiveModelContext>,
) -> anyhow::Result<()> {
    let model_context = model_context.as_ref().map(|ctx| KmLlmModelContext {
        config: &ctx.config,
        model: &ctx.model,
    });
    let result = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: cmd.package_dir,
        },
        model_context,
    )?;
    print_llm_lint_result(&result, cmd.json)
}

fn store_root_or_default(store_root: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match store_root {
        Some(path) => Ok(path),
        None => Ok(canonical_project_root()?.join(".ontocode/ontograph/km/_store")),
    }
}

fn print_import_result(result: &KmImportResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("imported {}@{}", result.id, result.version);
    println!("status: {}", result.status);
    println!("store: {}", result.store_root);
    println!("cache: {}", result.cache_path);
    Ok(())
}

fn print_import_batch_result(result: &KmImportBatchResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("batch: {}", result.batch_dir);
    println!("imported: {}", result.imported_count);
    println!("skipped: {}", result.skipped_count);
    println!("failed: {}", result.failed_count);
    println!("store: {}", result.store_root);
    Ok(())
}

fn print_eval_result(result: &KmEvalResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("id: {}", result.id);
    println!("cases: {}", result.total_cases);
    println!("passed: {}", result.passed_cases);
    println!("failed: {}", result.failed_cases);
    println!("pass rate: {:.2}", result.pass_rate);
    println!("report: {}", result.report_path);
    Ok(())
}

fn print_librarian_health_result(
    result: &KmLibrarianHealthResult,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("active_modules: {}", result.active_modules);
    println!("obsolete_modules: {}", result.obsolete_modules);
    println!("issues: {}", result.issues.len());
    for issue in &result.issues {
        println!(
            "issue: {}@{} [{}] {} - {}",
            issue.id, issue.version, issue.code, issue.path, issue.message
        );
    }
    Ok(())
}

fn print_librarian_catalog_result(
    result: &KmLibrarianCatalogResult,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("modules: {}", result.modules.len());
    for module in &result.modules {
        println!("module: {}@{} - {}", module.id, module.version, module.path);
    }
    Ok(())
}

fn print_librarian_version_diff_result(
    result: &KmLibrarianVersionDiffResult,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!(
        "module: {} {} -> {}",
        result.module_id, result.left.version, result.right.version
    );
    println!("truncated: {}", result.truncated);
    println!(
        "facts: +{} -{} ~{}",
        result.added_facts.len(),
        result.removed_facts.len(),
        result.changed_facts.len()
    );
    println!(
        "mappings: +{} -{} ~{}",
        result.added_mappings.len(),
        result.removed_mappings.len(),
        result.changed_mappings.len()
    );
    Ok(())
}

fn print_librarian_query_result(result: &KmLibrarianQueryResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("query: {}", result.query);
    println!("candidates: {}", result.candidates.len());
    println!("truncated: {}", result.truncated);
    for candidate in &result.candidates {
        let scope = match candidate.scope {
            KmLibrarianQueryScope::Local => "local",
            KmLibrarianQueryScope::Global => "global",
        };
        let terms = candidate.matched_terms.join(",");
        let referenced_by = candidate
            .referenced_by
            .as_deref()
            .map(|referenced_by| format!(" referenced_by={referenced_by}"))
            .unwrap_or_default();
        println!(
            "candidate: {scope} {}@{} terms={} reason={}{} path={}",
            candidate.id, candidate.version, terms, candidate.reason, referenced_by, candidate.path
        );
    }
    Ok(())
}

fn print_librarian_heal_scan_result(
    result: &KmLibrarianHealScanResult,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("reports: {}", result.reports.len());
    for report in &result.reports {
        println!(
            "report: {}@{} [{}] {} - {}",
            report.id, report.version, report.code, report.path, report.message
        );
    }
    println!("conflicts: {}", result.conflicts.len());
    for conflict in &result.conflicts {
        println!(
            "conflict: {}@{} {}",
            conflict.package_id, conflict.package_version, conflict.package_path
        );
    }
    Ok(())
}

fn print_librarian_consolidate_result(
    result: &KmLibrarianConsolidateResult,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("draft: {}", result.consolidation.output_dir);
    println!("facts: {}", result.consolidation.facts);
    println!("mappings: {}", result.consolidation.mappings);
    println!("conflicts: {}", result.consolidation.conflicts);
    println!("health_active_modules: {}", result.health.active_modules);
    println!("health_issues: {}", result.health.issues.len());
    println!("heal_reports: {}", result.heal_scan.reports.len());
    println!("heal_conflicts: {}", result.heal_scan.conflicts.len());
    println!("promotion: {}", result.promotion_skipped_reason);
    Ok(())
}

fn print_llm_extract_result(result: &KmLlmExtractResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("draft: {}", result.output_dir);
    println!("facts: {}", result.facts);
    println!("mappings: {}", result.mappings);
    Ok(())
}

fn print_llm_consolidate_result(result: &KmLlmConsolidateResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("draft: {}", result.output_dir);
    println!("facts: {}", result.facts);
    println!("mappings: {}", result.mappings);
    println!("conflicts: {}", result.conflicts);
    Ok(())
}

fn print_llm_recipe_result(result: &KmLlmRecipeResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("recipe: {}", result.recipe_path);
    println!("generated outputs: {}", result.generated_outputs);
    println!("conflicts: {}", result.conflicts);
    Ok(())
}

fn print_llm_judge_result(result: &KmLlmJudgeResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("judgments: {}", result.judgments_path);
    println!("cases generated: {}", result.cases_generated);
    println!("judgments written: {}", result.judgments_written);
    Ok(())
}

fn print_llm_lint_result(result: &KmLlmLintResult, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
        return Ok(());
    }

    println!("lint report: {}", result.report_path);
    println!("findings: {}", result.findings);
    println!("health checked: {}", result.health_checked);
    Ok(())
}
