use std::path::PathBuf;

use super::ChatWidget;
use crate::app_event::AppEvent;
use crate::history_cell;

const ONTOGRAPH_USAGE: &str = "Usage: /ontograph [status|tools|parity|index]";
const ONTOGRAPH_ROOT: &str = ".ontocode/ontograph";

impl ChatWidget {
    pub(crate) fn ontograph_overview_message(&self) -> String {
        [
            "Ontograph actions:",
            "/ontograph status  - show local artifact readiness",
            "/ontograph tools   - list checked-in native tool surface",
            "/ontograph parity  - show donor parity summary",
            "/ontograph index   - rebuild the local source index",
        ]
        .join("\n")
    }

    pub(crate) fn handle_ontograph_args(&mut self, args: &str) {
        match args.trim().to_ascii_lowercase().as_str() {
            "status" => self.add_info_message(self.ontograph_status_message(), /*hint*/ None),
            "tools" => self.add_info_message(self.ontograph_tools_message(), /*hint*/ None),
            "parity" => self.add_info_message(self.ontograph_parity_message(), /*hint*/ None),
            "index" => self.start_ontograph_index(),
            _ => self.add_error_message(ONTOGRAPH_USAGE.to_string()),
        }
    }

    /// Rebuilds the local source index through the extension's single writer action.
    ///
    /// The rebuild walks the whole workspace, so it runs on a blocking task and
    /// reports its outcome through the app event channel rather than stalling
    /// the draw loop.
    fn start_ontograph_index(&mut self) {
        let repo_root = self.config.cwd.to_path_buf();
        let tx = self.app_event_tx.clone();
        self.add_info_message(
            "Ontograph index: rebuilding the local source index...".to_string(),
            /*hint*/ None,
        );
        tokio::task::spawn_blocking(move || {
            let cell: Box<dyn history_cell::HistoryCell> =
                match ontocode_ontograph_extension::rebuild_source_index(&repo_root) {
                    Ok(outcome) => Box::new(history_cell::new_info_event(
                        format!(
                            "Ontograph index: published generation {generation}\ncounts: files {files}, symbols {symbols}, relationships {relationships}, nodes {nodes}, edges {edges}",
                            generation = outcome.generation_id,
                            files = outcome.files,
                            symbols = outcome.symbols,
                            relationships = outcome.relationships,
                            nodes = outcome.nodes,
                            edges = outcome.edges,
                        ),
                        /*hint*/ None,
                    )),
                    Err(message) => Box::new(history_cell::new_error_event(format!(
                        "Ontograph index failed: {message}"
                    ))),
                };
            tx.send(AppEvent::InsertHistoryCell(cell));
        });
    }

    fn ontograph_artifact_root(&self) -> PathBuf {
        self.config.cwd.join(ONTOGRAPH_ROOT).to_path_buf()
    }

    fn ontograph_status_message(&self) -> String {
        let root = self.ontograph_artifact_root();
        let current_path = root.join("current-generation");
        let Ok(current_generation) = std::fs::read_to_string(&current_path) else {
            return format!(
                "Ontograph status: not indexed\nartifact root: {}\nmissing: {}",
                root.display(),
                current_path.display()
            );
        };
        let current_generation = current_generation.trim();
        let manifest_path = root
            .join("generations")
            .join(current_generation)
            .join("manifest.json");
        let source_index_path = root
            .join("generations")
            .join(current_generation)
            .join("source-index.json");
        let manifest_status = match std::fs::read_to_string(&manifest_path) {
            Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(manifest) => {
                    let counts = manifest
                        .get("completed_generations")
                        .and_then(|generations| generations.as_array())
                        .and_then(|generations| {
                            generations.iter().find(|generation| {
                                generation
                                    .get("metadata")
                                    .and_then(|metadata| metadata.get("generation_id"))
                                    .and_then(|id| id.as_str())
                                    == Some(current_generation)
                            })
                        })
                        .and_then(|generation| generation.get("counts"));
                    match counts {
                        Some(counts) => format!("manifest: ready\ncounts: {counts}"),
                        None => "manifest: ready".to_string(),
                    }
                }
                Err(_) => "manifest: invalid".to_string(),
            },
            Err(_) => "manifest: missing".to_string(),
        };
        let source_index_status = if source_index_path.exists() {
            "source-index: present"
        } else {
            "source-index: missing"
        };

        format!(
            "Ontograph status: local-only\nartifact root: {}\nactive generation: {}\n{}\n{}",
            root.display(),
            current_generation,
            manifest_status,
            source_index_status
        )
    }

    fn ontograph_tools_message(&self) -> String {
        let tools = ontocode_ontograph_extension::implemented_tool_names();
        if tools.is_empty() {
            return "Ontograph tools: the native extension reports no installed tools.".to_string();
        }

        format!("Ontograph tools:\n- {}", tools.join("\n- "))
    }

    fn ontograph_parity_message(&self) -> String {
        let parity = ontocode_ontograph_extension::parity_summary();
        format!(
            "Ontograph parity:\n\
             - Current donor-facade parity: {implemented}/{total} donor facade actions implemented\n\
             - Donor actions tracked: {total}\n\
             - Current implemented donor actions in Ontograph: {implemented}\n\
             - Current intentional divergences accepted by ADR: {divergence}\n\
             - Current missing actions: {missing}",
            implemented = parity.donor_actions_implemented,
            total = parity.donor_actions_total,
            divergence = parity.donor_actions_intentional_divergence,
            missing = parity.donor_actions_missing,
        )
    }
}
