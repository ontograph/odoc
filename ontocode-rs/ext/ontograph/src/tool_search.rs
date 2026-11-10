use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypedCypherQuery {
    SymbolNameContains(String),
    FilePathContains(String),
}

fn parse_typed_cypher(query: &str) -> Result<TypedCypherQuery, String> {
    const MAX_TYPED_CYPHER_BYTES: usize = 8 * 1024;
    const SYMBOL_PREFIX: &str = "MATCH (s:Symbol) WHERE s.name CONTAINS '";
    const SYMBOL_SUFFIX: &str = "' RETURN s";
    const FILE_PREFIX: &str = "MATCH (f:File) WHERE f.path CONTAINS '";
    const FILE_SUFFIX: &str = "' RETURN f";

    if query.len() > MAX_TYPED_CYPHER_BYTES {
        return Err(format!(
            "ontograph.search cypher query exceeds the {MAX_TYPED_CYPHER_BYTES}-byte typed-query limit"
        ));
    }

    let parse_literal = |prefix: &str, suffix: &str| {
        query
            .strip_prefix(prefix)
            .and_then(|value| value.strip_suffix(suffix))
            .filter(|value| {
                !value.is_empty()
                    && value.chars().all(|character| {
                        !character.is_control() && character != '\'' && character != '\\'
                    })
            })
            .map(str::to_string)
    };

    if let Some(value) = parse_literal(SYMBOL_PREFIX, SYMBOL_SUFFIX) {
        return Ok(TypedCypherQuery::SymbolNameContains(value));
    }
    if let Some(value) = parse_literal(FILE_PREFIX, FILE_SUFFIX) {
        return Ok(TypedCypherQuery::FilePathContains(value));
    }

    Err("ontograph.search cypher accepts only bounded typed Symbol.name or File.path CONTAINS queries; raw or write queries are rejected".to_string())
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for SearchTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<SearchArgs, SearchResult>(SEARCH_TOOL_NAME, SEARCH_DESCRIPTION)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<SearchArgs>(&call, "ontograph.search")?;
        let query = args.query.trim();
        if query.is_empty() {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.search query must not be empty".to_string(),
            ));
        }
        let response_action = args.action.clone();
        let (execution_action, effective_query) = match args.action {
            SearchAction::Semantic => (SearchAction::Semantic, query.to_string()),
            SearchAction::Cypher => {
                match parse_typed_cypher(query).map_err(FunctionCallError::RespondToModel)? {
                    TypedCypherQuery::SymbolNameContains(value) => (SearchAction::Semantic, value),
                    TypedCypherQuery::FilePathContains(value) => (SearchAction::Repomap, value),
                }
            }
            SearchAction::Repomap => (SearchAction::Repomap, query.to_string()),
        };
        let store = self
            .thread_state
            .load_current_source_index("ontograph.search")?;
        let budget = QueryBudget::bounded(
            MAX_QUERY_TIMEOUT_MS,
            args.max_items.unwrap_or(MAX_SYMBOLS),
            MAX_SERIALIZED_BYTES,
        );
        let response = match execution_action {
            SearchAction::Semantic => {
                let result = store
                    .search_symbols(SymbolSearchRequest {
                        query: effective_query,
                        budget,
                    })
                    .map_err(search_query_error)?;
                SearchResult {
                    action: response_action.clone(),
                    query: query.to_string(),
                    matches: result
                        .matches
                        .items
                        .into_iter()
                        .map(search_symbol_summary)
                        .collect(),
                    files: Vec::new(),
                    truncated: result.matches.truncated,
                    limits: vec![
                        format!("search matches are capped at {MAX_QUERY_ITEMS} items"),
                        if response_action == SearchAction::Cypher {
                            "search.cypher translates only to native source-index symbol matching"
                                .to_string()
                        } else {
                            "search.semantic currently uses native source-index symbol matching only"
                                .to_string()
                        },
                    ],
                }
            }
            SearchAction::Repomap => {
                let result = store
                    .search_repomap(RepoMapRequest {
                        query: effective_query,
                        budget,
                    })
                    .map_err(search_query_error)?;
                SearchResult {
                    action: response_action.clone(),
                    query: query.to_string(),
                    matches: Vec::new(),
                    files: result
                        .files
                        .items
                        .into_iter()
                        .map(|file| SearchRepomapFileSummary {
                            path: file.path,
                            language: file.language,
                            symbols: file
                                .symbols
                                .into_iter()
                                .map(search_symbol_summary)
                                .collect(),
                        })
                        .collect(),
                    truncated: result.files.truncated,
                    limits: vec![
                        format!("repomap files are capped at {MAX_QUERY_ITEMS} items"),
                        format!("repomap symbols per file are capped at {MAX_QUERY_ITEMS} items"),
                        if response_action == SearchAction::Cypher {
                            "search.cypher translates only to native source-index file and symbol facts"
                                .to_string()
                        } else {
                            "search.repomap currently uses native source-index file and symbol facts only"
                                .to_string()
                        },
                    ],
                }
            }
            SearchAction::Cypher => {
                unreachable!("typed cypher queries always select a native action")
            }
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.search result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}
