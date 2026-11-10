use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::artifact_io::read_to_string;
use crate::manifest::OntographArtifactRoot;
use crate::manifest::OntographManifest;
use crate::query::DocsContextItem;
use crate::query::DocsContextRequest;
use crate::query::DocsContextResult;
use crate::query::DocsDriftItem;
use crate::query::DocsDriftRequest;
use crate::query::DocsDriftResult;
use crate::query::DocsDriftSkippedReference;
use crate::query::DocsMigrationItem;
use crate::query::DocsMigrationResult;
use crate::query::DocsMigrationSkippedReference;
use crate::query::DocsRequirementItem;
use crate::query::DocsRequirementLink;
use crate::query::DocsRequirementResult;
use crate::query::DocsTraceHeading;
use crate::query::DocsTraceItem;
use crate::query::DocsTraceRequest;
use crate::query::DocsTraceResult;
use crate::query::QueryDeadline;
use crate::query::QueryError;
use crate::query::QueryResult;
use crate::query::SourceReferenceSet;
use crate::query::bounded_serialized_items;
use crate::query::check_query_scan_deadline;
use crate::query::query_scan_step;

pub(crate) const DOCS_INDEX_FILE: &str = "docs-index.json";
pub(crate) const DOCS_SCHEMA_VERSION: u32 = 1;
pub(crate) const MAX_DOCS_CONTEXT_ITEMS: usize = 100;
pub(crate) const MAX_DOCS_EXCERPT_BYTES: usize = 1024;
const MAX_ARTIFACT_READ_BYTES: usize = 64 * 1024 * 1024;
const MAX_POINTER_READ_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocsIndexStore {
    artifact: DocsIndexArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocsIndexLoadError {
    MissingCurrentPointer,
    InvalidCurrentPointer,
    MissingSourceManifest,
    ReadSourceManifest,
    InvalidSourceManifest,
    MissingDocsIndex,
    ReadDocsIndex,
    InvalidDocsIndex,
    UnsupportedSchema,
    InProgress,
    SourceIdentityMismatch,
}

impl DocsIndexStore {
    pub(crate) fn load_active_generation(
        root: &OntographArtifactRoot,
    ) -> Result<Self, DocsIndexLoadError> {
        Self::load_active_generation_with_link_policy(root, LinkValidation::RequireSourceDocuments)
    }

    pub(crate) fn load_active_generation_for_drift(
        root: &OntographArtifactRoot,
    ) -> Result<Self, DocsIndexLoadError> {
        Self::load_active_generation_with_link_policy(root, LinkValidation::AllowStaleDocuments)
    }

    fn load_active_generation_with_link_policy(
        root: &OntographArtifactRoot,
        link_validation: LinkValidation,
    ) -> Result<Self, DocsIndexLoadError> {
        let generation_id = read_to_string(&root.current_pointer_path(), MAX_POINTER_READ_BYTES)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => DocsIndexLoadError::MissingCurrentPointer,
                _ => DocsIndexLoadError::InvalidCurrentPointer,
            })?;
        let generation_id = generation_id.trim();
        let source_manifest_path = root
            .generation_manifest_path(generation_id)
            .map_err(|_| DocsIndexLoadError::InvalidCurrentPointer)?;
        let source_manifest = read_to_string(&source_manifest_path, MAX_ARTIFACT_READ_BYTES)
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    DocsIndexLoadError::MissingSourceManifest
                } else {
                    DocsIndexLoadError::ReadSourceManifest
                }
            })?;
        let source_manifest = serde_json::from_str::<OntographManifest>(&source_manifest)
            .map_err(|_| DocsIndexLoadError::InvalidSourceManifest)?;
        let source_generation = source_manifest
            .completed_generations
            .iter()
            .find(|generation| {
                generation.metadata.generation_id == source_manifest.active_generation_id
            })
            .ok_or(DocsIndexLoadError::InvalidSourceManifest)?;

        let docs_index_path = root
            .generation_file_path(generation_id, DOCS_INDEX_FILE)
            .map_err(|_| DocsIndexLoadError::InvalidCurrentPointer)?;
        let contents =
            read_to_string(&docs_index_path, MAX_ARTIFACT_READ_BYTES).map_err(|err| {
                match err.kind() {
                    std::io::ErrorKind::NotFound => DocsIndexLoadError::MissingDocsIndex,
                    _ => DocsIndexLoadError::ReadDocsIndex,
                }
            })?;
        let artifact = serde_json::from_str::<DocsIndexArtifact>(&contents)
            .map_err(|_| DocsIndexLoadError::InvalidDocsIndex)?;
        if artifact.manifest.schema_version != DOCS_SCHEMA_VERSION {
            return Err(DocsIndexLoadError::UnsupportedSchema);
        }
        if artifact.manifest.completed_at_unix_seconds.is_none() {
            return Err(DocsIndexLoadError::InProgress);
        }
        if artifact.manifest.repository_root != source_manifest.repository.root
            || artifact.manifest.source_index_generation_id
                != source_generation.metadata.generation_id
            || artifact.manifest.source_index_content_hash
                != source_generation.metadata.manifest_hash
        {
            return Err(DocsIndexLoadError::SourceIdentityMismatch);
        }
        let document_ids = artifact
            .documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<BTreeSet<_>>();
        if artifact.facts.iter().any(|fact| {
            !document_ids.contains(fact.document_id.as_str())
                || fact.line_start == 0
                || fact.line_end < fact.line_start
        }) {
            return Err(DocsIndexLoadError::InvalidDocsIndex);
        }
        if artifact.links.iter().any(|link| {
            link_validation == LinkValidation::RequireSourceDocuments
                && !document_ids.contains(link.from_document_id.as_str())
        }) {
            return Err(DocsIndexLoadError::InvalidDocsIndex);
        }

        Ok(Self { artifact })
    }

    pub(crate) fn context(&self, request: DocsContextRequest) -> QueryResult<DocsContextResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        check_query_scan_deadline(deadline)?;
        let selector = DocsContextSelector::from_request(&request)?;
        let mut documents_by_id = BTreeMap::new();
        let mut facts_by_id = BTreeMap::new();
        let mut matched_document_ids = BTreeSet::new();
        let mut matched_fact_ids = BTreeSet::new();

        for document in &self.artifact.documents {
            query_scan_step();
            check_query_scan_deadline(deadline)?;
            documents_by_id.insert(document.id.as_str(), document);
            if selector.matches_document(document) {
                matched_document_ids.insert(document.id.as_str());
            }
        }
        for fact in &self.artifact.facts {
            query_scan_step();
            check_query_scan_deadline(deadline)?;
            // Preserve the first duplicate fact, as the former linear lookup did.
            facts_by_id.entry(fact.id.as_str()).or_insert(fact);
            if selector.matches_fact(fact)
                || matched_document_ids.contains(fact.document_id.as_str())
            {
                matched_fact_ids.insert(fact.id.as_str());
                matched_document_ids.insert(fact.document_id.as_str());
            }
        }

        let candidate_limit = request.budget.max_items.saturating_add(1);
        let mut items = Vec::new();
        let mut retain = |candidate: RankedDocsContextItem| {
            let position = items.partition_point(|item: &RankedDocsContextItem| {
                item.sort_key <= candidate.sort_key
            });
            if position < candidate_limit {
                if items.len() == candidate_limit {
                    items.pop();
                }
                items.insert(position, candidate);
            }
        };
        for document_id in matched_document_ids {
            query_scan_step();
            check_query_scan_deadline(deadline)?;
            let Some(document) = documents_by_id.get(document_id) else {
                return Err(QueryError::CorruptArtifact);
            };
            retain(RankedDocsContextItem {
                sort_key: format!("0:{}:{}", document.path, document.id),
                item: DocsContextItem::Document {
                    document_id: document.id.clone(),
                    path: document.path.clone(),
                    title: document.title.clone(),
                    document_kind: document.kind.clone(),
                    excerpt: truncate_excerpt(&document.summary),
                },
            });
        }
        for fact_id in matched_fact_ids {
            query_scan_step();
            check_query_scan_deadline(deadline)?;
            let Some(fact) = facts_by_id.get(fact_id) else {
                return Err(QueryError::CorruptArtifact);
            };
            retain(RankedDocsContextItem {
                sort_key: format!("1:{}:{:010}:{}", fact.document_id, fact.line_start, fact.id),
                item: DocsContextItem::Fact {
                    fact_id: fact.id.clone(),
                    document_id: fact.document_id.clone(),
                    heading_id: fact.heading_id.clone(),
                    fact_kind: fact.kind.clone(),
                    text: truncate_excerpt(&fact.text),
                    line_start: fact.line_start,
                    line_end: fact.line_end,
                    requirement_id: fact.requirement_id.clone(),
                },
            });
        }
        check_query_scan_deadline(deadline)?;
        let result = DocsContextResult {
            items: bounded_serialized_items(
                items.into_iter().map(|item| item.item).collect(),
                request.budget,
            )?,
        };
        check_query_scan_deadline(deadline)?;
        Ok(result)
    }

    pub(crate) fn drift(
        &self,
        request: DocsDriftRequest,
        source: &SourceReferenceSet,
    ) -> QueryResult<DocsDriftResult> {
        let document_ids = self
            .artifact
            .documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut items = Vec::new();
        let mut skipped = Vec::new();

        for fact in &self.artifact.facts {
            let normalized_paths = fact
                .paths
                .iter()
                .map(|path| normalize_docs_path(path))
                .collect::<Vec<_>>();
            for (raw_path, normalized_path) in fact.paths.iter().zip(normalized_paths.iter()) {
                if !source.paths.contains(normalized_path) {
                    items.push(DocsDriftItem::MissingSourcePath {
                        document_id: fact.document_id.clone(),
                        fact_id: fact.id.clone(),
                        path: if raw_path == normalized_path {
                            raw_path.clone()
                        } else {
                            normalized_path.clone()
                        },
                    });
                }
            }
            for symbol in &fact.symbols {
                if !docs_symbol_is_present(source, &normalized_paths, symbol) {
                    items.push(DocsDriftItem::MissingSymbol {
                        document_id: fact.document_id.clone(),
                        fact_id: fact.id.clone(),
                        symbol: symbol.clone(),
                        path: normalized_paths.first().cloned(),
                    });
                }
            }
            for route in &fact.routes {
                if !source.route_paths.contains(route) {
                    items.push(DocsDriftItem::MissingRoute {
                        document_id: fact.document_id.clone(),
                        fact_id: fact.id.clone(),
                        route: route.clone(),
                    });
                }
            }
        }

        for link in &self.artifact.links {
            if !document_ids.contains(link.from_document_id.as_str()) {
                items.push(DocsDriftItem::StaleDocumentLink {
                    link_id: link.id.clone(),
                    from_document_id: link.from_document_id.clone(),
                    target_document_id: link.target.clone(),
                    missing_document_id: link.from_document_id.clone(),
                    missing_endpoint: "from_document_id".to_string(),
                });
            }
            match link.target_kind.as_str() {
                "document_id" if !document_ids.contains(link.target.as_str()) => {
                    items.push(DocsDriftItem::StaleDocumentLink {
                        link_id: link.id.clone(),
                        from_document_id: link.from_document_id.clone(),
                        target_document_id: link.target.clone(),
                        missing_document_id: link.target.clone(),
                        missing_endpoint: "target_document_id".to_string(),
                    });
                }
                "document_id" | "requirement" | "requirement_id" | "route" | "path" | "symbol" => {}
                _ => skipped.push(DocsDriftSkippedReference {
                    link_id: link.id.clone(),
                    target_kind: link.target_kind.clone(),
                    target: link.target.clone(),
                    reason: "unsupported docs link target kind is not drift".to_string(),
                }),
            }
        }

        items.sort_by_key(|item| format!("{item:?}"));
        skipped.sort_by(|left, right| {
            left.link_id
                .cmp(&right.link_id)
                .then(left.target_kind.cmp(&right.target_kind))
                .then(left.target.cmp(&right.target))
        });
        Ok(DocsDriftResult {
            items: bounded_serialized_items(items, request.budget.clone())?,
            skipped: bounded_serialized_items(skipped, request.budget)?,
        })
    }

    pub(crate) fn readiness(&self) -> DocsReadinessSummary {
        DocsReadinessSummary {
            status: "ready".to_string(),
            paired: true,
            schema_version: self.artifact.manifest.schema_version,
            completed_at_unix_seconds: self
                .artifact
                .manifest
                .completed_at_unix_seconds
                .unwrap_or_default(),
            source_index_generation_id: self.artifact.manifest.source_index_generation_id.clone(),
            source_index_content_hash: self.artifact.manifest.source_index_content_hash.clone(),
            document_count: self.artifact.documents.len(),
            fact_count: self.artifact.facts.len(),
            link_count: self.artifact.links.len(),
        }
    }

    pub(crate) fn migration_references(
        &self,
        budget: crate::query::QueryBudget,
    ) -> QueryResult<DocsMigrationResult> {
        let mut items = Vec::new();
        let mut skipped = Vec::new();
        for fact in &self.artifact.facts {
            match fact.kind.as_str() {
                "migration" | "migration_reference" => items.push(DocsMigrationItem {
                    document_id: fact.document_id.clone(),
                    fact_id: fact.id.clone(),
                    fact_kind: fact.kind.clone(),
                    text: truncate_excerpt(&fact.text),
                    line_start: fact.line_start,
                    line_end: fact.line_end,
                    requirement_id: fact.requirement_id.clone(),
                    paths: fact
                        .paths
                        .iter()
                        .map(|path| normalize_docs_path(path))
                        .collect(),
                    symbols: fact.symbols.clone(),
                    routes: fact.routes.clone(),
                }),
                kind if kind.starts_with("migration") => {
                    skipped.push(DocsMigrationSkippedReference {
                        document_id: fact.document_id.clone(),
                        fact_id: fact.id.clone(),
                        fact_kind: fact.kind.clone(),
                        reason: "unsupported migration fact kind is not a migration reference"
                            .to_string(),
                    });
                }
                _ => {}
            }
        }
        items.sort_by(|left, right| {
            left.document_id
                .cmp(&right.document_id)
                .then(left.line_start.cmp(&right.line_start))
                .then(left.fact_id.cmp(&right.fact_id))
        });
        skipped.sort_by(|left, right| {
            left.document_id
                .cmp(&right.document_id)
                .then(left.fact_id.cmp(&right.fact_id))
        });
        Ok(DocsMigrationResult {
            items: bounded_serialized_items(items, budget.clone())?,
            skipped: bounded_serialized_items(skipped, budget)?,
        })
    }

    pub(crate) fn requirement_references(
        &self,
        budget: crate::query::QueryBudget,
    ) -> QueryResult<DocsRequirementResult> {
        let mut items = Vec::new();
        let mut links = Vec::new();
        for fact in &self.artifact.facts {
            if let Some(requirement_id) = &fact.requirement_id {
                items.push(DocsRequirementItem {
                    document_id: fact.document_id.clone(),
                    fact_id: fact.id.clone(),
                    fact_kind: fact.kind.clone(),
                    requirement_id: requirement_id.clone(),
                    text: truncate_excerpt(&fact.text),
                    line_start: fact.line_start,
                    line_end: fact.line_end,
                    paths: fact
                        .paths
                        .iter()
                        .map(|path| normalize_docs_path(path))
                        .collect(),
                    symbols: fact.symbols.clone(),
                    routes: fact.routes.clone(),
                });
            }
        }
        for link in &self.artifact.links {
            if matches!(link.target_kind.as_str(), "requirement" | "requirement_id") {
                links.push(DocsRequirementLink {
                    link_id: link.id.clone(),
                    from_document_id: link.from_document_id.clone(),
                    requirement_id: link.target.clone(),
                });
            }
        }
        items.sort_by(|left, right| {
            left.requirement_id
                .cmp(&right.requirement_id)
                .then(left.document_id.cmp(&right.document_id))
                .then(left.line_start.cmp(&right.line_start))
                .then(left.fact_id.cmp(&right.fact_id))
        });
        links.sort_by(|left, right| {
            left.requirement_id
                .cmp(&right.requirement_id)
                .then(left.from_document_id.cmp(&right.from_document_id))
                .then(left.link_id.cmp(&right.link_id))
        });
        Ok(DocsRequirementResult {
            items: bounded_serialized_items(items, budget.clone())?,
            links: bounded_serialized_items(links, budget)?,
        })
    }

    pub(crate) fn trace(&self, request: DocsTraceRequest) -> QueryResult<DocsTraceResult> {
        let selectors = DocsTraceSelectors::from_request(&request)?;
        let documents_by_id = self
            .artifact
            .documents
            .iter()
            .map(|document| (document.id.as_str(), document))
            .collect::<BTreeMap<_, _>>();
        let mut matched_document_ids = BTreeSet::new();
        let mut matched_fact_ids = BTreeSet::new();
        let mut matched_link_ids = BTreeSet::new();
        let mut items = Vec::new();

        for selector in selectors.unresolved_selectors(&self.artifact) {
            items.push(RankedDocsTraceItem {
                sort_key: format!("3:{}:{}", selector.kind.as_str(), selector.value),
                item: DocsTraceItem::UnresolvedSelector {
                    selector: selector.kind.as_str().to_string(),
                    value: selector.value.to_string(),
                    reason:
                        "selector did not match any explicit docs-index document, fact, or link"
                            .to_string(),
                },
            });
        }

        for document in &self.artifact.documents {
            if selectors.matches_document(document) {
                matched_document_ids.insert(document.id.as_str());
            }
        }
        for fact in &self.artifact.facts {
            let Some(document) = documents_by_id.get(fact.document_id.as_str()) else {
                return Err(QueryError::CorruptArtifact);
            };
            if selectors.matches_fact(fact, document) {
                matched_fact_ids.insert(fact.id.as_str());
                matched_document_ids.insert(fact.document_id.as_str());
            }
        }
        for link in &self.artifact.links {
            if selectors.matches_link(link) {
                matched_link_ids.insert(link.id.as_str());
                matched_document_ids.insert(link.from_document_id.as_str());
                if link.target_kind == "document_id"
                    && documents_by_id.contains_key(link.target.as_str())
                {
                    matched_document_ids.insert(link.target.as_str());
                }
            }
        }

        for document_id in matched_document_ids {
            let Some(document) = documents_by_id.get(document_id) else {
                return Err(QueryError::CorruptArtifact);
            };
            items.push(RankedDocsTraceItem {
                sort_key: format!("0:{}:{}", document.path, document.id),
                item: DocsTraceItem::Document {
                    document_id: document.id.clone(),
                    path: document.path.clone(),
                    title: document.title.clone(),
                    headings: document
                        .headings
                        .iter()
                        .map(|heading| DocsTraceHeading {
                            heading_id: heading.id.clone(),
                            title: heading.title.clone(),
                        })
                        .collect(),
                },
            });
        }
        for fact_id in matched_fact_ids {
            let Some(fact) = self.artifact.facts.iter().find(|fact| fact.id == fact_id) else {
                return Err(QueryError::CorruptArtifact);
            };
            items.push(RankedDocsTraceItem {
                sort_key: format!("1:{}:{:010}:{}", fact.document_id, fact.line_start, fact.id),
                item: DocsTraceItem::Fact {
                    fact_id: fact.id.clone(),
                    document_id: fact.document_id.clone(),
                    heading_id: fact.heading_id.clone(),
                    fact_kind: fact.kind.clone(),
                    text: truncate_excerpt(&fact.text),
                    line_start: fact.line_start,
                    line_end: fact.line_end,
                    requirement_id: fact.requirement_id.clone(),
                },
            });
        }
        for link_id in matched_link_ids {
            let Some(link) = self.artifact.links.iter().find(|link| link.id == link_id) else {
                return Err(QueryError::CorruptArtifact);
            };
            items.push(RankedDocsTraceItem {
                sort_key: format!(
                    "2:{}:{}:{}",
                    link.from_document_id, link.target_kind, link.id
                ),
                item: DocsTraceItem::Link {
                    link_id: link.id.clone(),
                    from_document_id: link.from_document_id.clone(),
                    to_document_id: (link.target_kind == "document_id")
                        .then(|| link.target.clone()),
                    target_kind: link.target_kind.clone(),
                    target: link.target.clone(),
                    requirement_id: (link.target_kind == "requirement_id")
                        .then(|| link.target.clone()),
                    route: (link.target_kind == "route").then(|| link.target.clone()),
                    path: (link.target_kind == "path").then(|| link.target.clone()),
                    symbol: (link.target_kind == "symbol").then(|| link.target.clone()),
                },
            });
        }

        items.sort_by(|left, right| left.sort_key.cmp(&right.sort_key));
        Ok(DocsTraceResult {
            items: bounded_serialized_items(
                items.into_iter().map(|item| item.item).collect(),
                request.budget,
            )?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkValidation {
    RequireSourceDocuments,
    AllowStaleDocuments,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsReadinessSummary {
    pub status: String,
    pub paired: bool,
    pub schema_version: u32,
    pub completed_at_unix_seconds: i64,
    pub source_index_generation_id: String,
    pub source_index_content_hash: String,
    pub document_count: usize,
    pub fact_count: usize,
    pub link_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsIndexArtifact {
    pub manifest: DocsIndexManifest,
    pub documents: Vec<DocsDocument>,
    pub facts: Vec<DocsFact>,
    pub links: Vec<DocsLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsIndexManifest {
    pub schema_version: u32,
    pub generator_version: String,
    pub repository_root: String,
    pub source_index_generation_id: String,
    pub source_index_content_hash: String,
    pub created_at_unix_seconds: i64,
    pub completed_at_unix_seconds: Option<i64>,
    pub artifact_content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsDocument {
    pub id: String,
    pub path: String,
    pub title: String,
    pub kind: String,
    pub content_hash: String,
    pub byte_length: usize,
    pub headings: Vec<DocsHeading>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsHeading {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsFact {
    pub id: String,
    pub document_id: String,
    pub heading_id: Option<String>,
    pub kind: String,
    pub text: String,
    pub line_start: usize,
    pub line_end: usize,
    pub requirement_id: Option<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsLink {
    pub id: String,
    pub from_document_id: String,
    pub target_kind: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedDocsContextItem {
    sort_key: String,
    item: DocsContextItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedDocsTraceItem {
    sort_key: String,
    item: DocsTraceItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DocsContextSelector<'a> {
    DocumentId(&'a str),
    Path(&'a str),
    HeadingId(&'a str),
    RequirementId(&'a str),
    Route(&'a str),
    Symbol(&'a str),
    Text(String),
}

impl<'a> DocsContextSelector<'a> {
    fn from_request(request: &'a DocsContextRequest) -> QueryResult<Self> {
        let selectors = [
            request.document_id.as_deref(),
            request.path.as_deref(),
            request.heading_id.as_deref(),
            request.requirement_id.as_deref(),
            request.route.as_deref(),
            request.symbol.as_deref(),
            request.text.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .count();
        if selectors != 1 {
            return Err(QueryError::AmbiguousArtifact);
        }
        if let Some(value) = request
            .document_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Self::DocumentId(value));
        }
        if let Some(value) = request
            .path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Self::Path(value));
        }
        if let Some(value) = request
            .heading_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Self::HeadingId(value));
        }
        if let Some(value) = request
            .requirement_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Self::RequirementId(value));
        }
        if let Some(value) = request
            .route
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Self::Route(value));
        }
        if let Some(value) = request
            .symbol
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Self::Symbol(value));
        }
        let Some(value) = request
            .text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Err(QueryError::AmbiguousArtifact);
        };
        Ok(Self::Text(value.to_ascii_lowercase()))
    }

    fn matches_document(&self, document: &DocsDocument) -> bool {
        match self {
            Self::DocumentId(value) => document.id == *value,
            Self::Path(value) => document.path == *value,
            Self::HeadingId(value) => document.headings.iter().any(|heading| heading.id == *value),
            Self::RequirementId(_) | Self::Route(_) | Self::Symbol(_) => false,
            Self::Text(value) => {
                document.title.to_ascii_lowercase().contains(value)
                    || document.summary.to_ascii_lowercase().contains(value)
                    || document.path.to_ascii_lowercase().contains(value)
            }
        }
    }

    fn matches_fact(&self, fact: &DocsFact) -> bool {
        match self {
            Self::DocumentId(value) => fact.document_id == *value,
            Self::Path(value) => fact.paths.iter().any(|path| path == value),
            Self::HeadingId(value) => fact.heading_id.as_deref() == Some(*value),
            Self::RequirementId(value) => fact.requirement_id.as_deref() == Some(*value),
            Self::Route(value) => fact.routes.iter().any(|route| route == value),
            Self::Symbol(value) => fact.symbols.iter().any(|symbol| symbol == value),
            Self::Text(value) => fact.text.to_ascii_lowercase().contains(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocsTraceSelectors<'a> {
    selectors: Vec<DocsTraceSelector<'a>>,
}

impl<'a> DocsTraceSelectors<'a> {
    fn from_request(request: &'a DocsTraceRequest) -> QueryResult<Self> {
        let selectors = [
            (
                DocsTraceSelectorKind::DocumentId,
                request.document_id.as_deref(),
            ),
            (
                DocsTraceSelectorKind::RequirementId,
                request.requirement_id.as_deref(),
            ),
            (DocsTraceSelectorKind::Route, request.route.as_deref()),
            (DocsTraceSelectorKind::Path, request.path.as_deref()),
            (DocsTraceSelectorKind::Symbol, request.symbol.as_deref()),
        ]
        .into_iter()
        .filter_map(|(kind, value)| {
            value
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| DocsTraceSelector { kind, value })
        })
        .collect::<Vec<_>>();
        if selectors.is_empty() {
            return Err(QueryError::AmbiguousArtifact);
        }
        Ok(Self { selectors })
    }

    fn matches_document(&self, document: &DocsDocument) -> bool {
        self.selectors
            .iter()
            .all(|selector| selector.matches_document(document))
    }

    fn matches_fact(&self, fact: &DocsFact, document: &DocsDocument) -> bool {
        self.selectors
            .iter()
            .all(|selector| selector.matches_fact(fact, document))
    }

    fn matches_link(&self, link: &DocsLink) -> bool {
        self.selectors
            .iter()
            .all(|selector| selector.matches_link(link))
    }

    fn unresolved_selectors(&self, artifact: &'a DocsIndexArtifact) -> Vec<DocsTraceSelector<'a>> {
        self.selectors
            .iter()
            .copied()
            .filter(|selector| {
                !artifact
                    .documents
                    .iter()
                    .any(|document| selector.matches_document(document))
                    && !artifact.facts.iter().any(|fact| {
                        artifact
                            .documents
                            .iter()
                            .find(|document| document.id == fact.document_id)
                            .is_some_and(|document| selector.matches_fact(fact, document))
                    })
                    && !artifact
                        .links
                        .iter()
                        .any(|link| selector.matches_link(link))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocsTraceSelector<'a> {
    kind: DocsTraceSelectorKind,
    value: &'a str,
}

impl DocsTraceSelector<'_> {
    fn matches_document(&self, document: &DocsDocument) -> bool {
        match self.kind {
            DocsTraceSelectorKind::DocumentId => document.id == self.value,
            DocsTraceSelectorKind::Path => document.path == self.value,
            DocsTraceSelectorKind::RequirementId
            | DocsTraceSelectorKind::Route
            | DocsTraceSelectorKind::Symbol => false,
        }
    }

    fn matches_fact(&self, fact: &DocsFact, document: &DocsDocument) -> bool {
        match self.kind {
            DocsTraceSelectorKind::DocumentId => fact.document_id == self.value,
            DocsTraceSelectorKind::RequirementId => {
                fact.requirement_id.as_deref() == Some(self.value)
            }
            DocsTraceSelectorKind::Route => fact.routes.iter().any(|route| route == self.value),
            DocsTraceSelectorKind::Path => {
                document.path == self.value || fact.paths.iter().any(|path| path == self.value)
            }
            DocsTraceSelectorKind::Symbol => fact.symbols.iter().any(|symbol| symbol == self.value),
        }
    }

    fn matches_link(&self, link: &DocsLink) -> bool {
        match self.kind {
            DocsTraceSelectorKind::DocumentId => {
                link.from_document_id == self.value
                    || (link.target_kind == "document_id" && link.target == self.value)
            }
            DocsTraceSelectorKind::RequirementId => {
                matches!(link.target_kind.as_str(), "requirement" | "requirement_id")
                    && link.target == self.value
            }
            DocsTraceSelectorKind::Route => {
                link.target_kind == "route" && link.target == self.value
            }
            DocsTraceSelectorKind::Path => link.target_kind == "path" && link.target == self.value,
            DocsTraceSelectorKind::Symbol => {
                link.target_kind == "symbol" && link.target == self.value
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocsTraceSelectorKind {
    DocumentId,
    RequirementId,
    Route,
    Path,
    Symbol,
}

impl DocsTraceSelectorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::DocumentId => "document_id",
            Self::RequirementId => "requirement_id",
            Self::Route => "route",
            Self::Path => "path",
            Self::Symbol => "symbol",
        }
    }
}

fn truncate_excerpt(value: &str) -> String {
    if value.len() <= MAX_DOCS_EXCERPT_BYTES {
        return value.to_string();
    }
    let mut end = 0;
    for (index, character) in value.char_indices() {
        let next = index + character.len_utf8();
        if next > MAX_DOCS_EXCERPT_BYTES {
            break;
        }
        end = next;
    }
    value[..end].to_string()
}

fn docs_symbol_is_present(source: &SourceReferenceSet, paths: &[String], symbol: &str) -> bool {
    source.symbols.contains(symbol)
        || paths.iter().any(|path| {
            source
                .symbols_by_path
                .get(path)
                .is_some_and(|symbols| symbols.contains(symbol))
        })
}

fn normalize_docs_path(path: &str) -> String {
    let replaced = path.trim().replace('\\', "/");
    let mut segments = Vec::new();
    for segment in replaced.split('/') {
        match segment {
            "" | "." => {}
            ".." => return replaced,
            _ => segments.push(segment),
        }
    }
    segments.join("/")
}
