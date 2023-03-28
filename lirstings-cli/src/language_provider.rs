use std::{fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use lirstings::language_provider::{LanguageProvider, Language};
use tree_sitter_loader::Loader;

pub struct DynamicLanguageProvider {
    pub query_search_dirs: Vec<String>,
    pub parser_search_dirs: Vec<PathBuf>,
}

impl LanguageProvider for DynamicLanguageProvider {
    fn provide(
        self,
        file_extension: &str, /* , highlight_names: &Vec<String> */
    ) -> Result<Language> {
        let mut loader = Loader::new()?;
        // TODO: is this needed?
        // loader.configure_highlights(highlight_names);
        loader.find_all_languages(&tree_sitter_loader::Config {
            parser_directories: self.parser_search_dirs,
        })?;

        let (lang, lang_config) = match loader.language_configuration_for_file_name(
            &PathBuf::from(format!("file.{file_extension}")),
        )? {
            Some(conf) => conf,
            None => {
                bail!("No matching tree-sitter configuration found");
            }
        };

        let parser_name = match lang_config.scope.as_ref() {
            Some(scope) => scope.replace("source.", ""),
            None => bail!("Parser has no scope specified"),
        };

        let mut highlights_query = String::new();
        let mut injection_query = String::new();
        let mut locals_query = String::new();
        for glob_str in &self.query_search_dirs {
            for dir in glob::glob(glob_str)?.filter_map(Result::ok) {
                let filetype_dir = dir.join(&parser_name);
                let highlights_file = filetype_dir.join("highlights.scm");
                let injection_file = filetype_dir.join("injections.scm");
                let locals_file = filetype_dir.join("locals.scm");

                // TODO: check for `; inherits: x` comments
                if highlights_file.is_file() {
                    highlights_query = fs::read_to_string(&highlights_file).with_context(|| {
                        format!("Could not read {}", highlights_file.to_string_lossy())
                    })?;
                }
                if injection_file.is_file() {
                    injection_query = fs::read_to_string(&injection_file).with_context(|| {
                        format!("Could not read {}", injection_file.to_string_lossy())
                    })?;
                }
                if locals_file.is_file() {
                    locals_query = fs::read_to_string(&locals_file).with_context(|| {
                        format!("Could not read {}", locals_file.to_string_lossy())
                    })?;
                }
            }
        }

        Ok(Language {
            inner: lang,
            highlights_query,
            injection_query,
            locals_query,
        })
    }
}
