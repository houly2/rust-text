use std::sync::Arc;
use tree_sitter::{Language, Query};

#[allow(dead_code)]
#[derive(Debug)]
pub struct LanguageConfig {
    pub language_id: &'static str,
    file_types: Vec<&'static str>,
    pub language: Language,
    pub injection_query: Option<Query>,
    pub highlight_query: Query,
}

impl LanguageConfig {
    pub fn new(
        language_id: &'static str,
        file_types: Vec<&'static str>,
        language: Language,
        injection_query: Option<Query>,
        highlight_query: Query,
    ) -> Self {
        Self {
            language_id,
            file_types,
            language,
            injection_query,
            highlight_query,
        }
    }
}

pub struct LanguageConfigManager {
    language_configs: Vec<Arc<LanguageConfig>>,
}

impl LanguageConfigManager {
    pub fn new() -> Self {
        let language_configs = vec![
            Arc::new(LanguageConfig::new(
                "markdown",
                vec!["md"],
                tree_sitter_md::LANGUAGE.into(),
                Some(Self::load_query(
                    &tree_sitter_md::LANGUAGE.into(),
                    include_str!("../../languages/markdown/injections.scm"),
                )),
                Self::load_query(
                    &tree_sitter_md::LANGUAGE.into(),
                    include_str!("../../languages/markdown/highlights.scm"),
                ),
            )),
            Arc::new(LanguageConfig::new(
                "markdown_inline",
                vec![],
                tree_sitter_md::INLINE_LANGUAGE.into(),
                None,
                Self::load_query(
                    &tree_sitter_md::INLINE_LANGUAGE.into(),
                    include_str!("../../languages/markdown-inline/highlights.scm"),
                ),
            )),
            Arc::new(LanguageConfig::new(
                "yml",
                vec!["yaml", "yml"],
                tree_sitter_yaml::LANGUAGE.into(),
                None,
                Self::load_query(
                    &tree_sitter_yaml::LANGUAGE.into(),
                    tree_sitter_yaml::HIGHLIGHTS_QUERY,
                ),
            )),
            Arc::new(LanguageConfig::new(
                "html",
                vec!["html"],
                tree_sitter_html::LANGUAGE.into(),
                Some(Self::load_query(
                    &tree_sitter_html::LANGUAGE.into(),
                    tree_sitter_html::INJECTIONS_QUERY,
                )),
                Self::load_query(
                    &tree_sitter_html::LANGUAGE.into(),
                    tree_sitter_html::HIGHLIGHTS_QUERY,
                ),
            )),
        ];

        Self { language_configs }
    }

    fn load_query(language: &Language, source: &'static str) -> Query {
        Query::new(language, source).expect("Could not load Language")
    }

    pub fn language_config_for_language_id(&self, id: &str) -> Option<Arc<LanguageConfig>> {
        self.language_configs
            .iter()
            .find(|lang| lang.language_id == id)
            .cloned()
    }
}
