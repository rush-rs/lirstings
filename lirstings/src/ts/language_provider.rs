use anyhow::Result;
pub use anyhow;

pub struct Language {
    pub inner: tree_sitter::Language,
    pub highlights_query: String,
    pub injection_query: String,
    pub locals_query: String,
}

pub trait LanguageProvider {
    // TODO: this should not depend on anyhow
    fn provide(self, file_extension: &str) -> Result<Language>;
}
