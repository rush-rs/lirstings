use std::{collections::HashMap, fs, ops::RangeInclusive, path::PathBuf};

use anyhow::{bail, Context, Result};
use tree_sitter::{Language, Query, QueryPredicateArg};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_loader::Loader;

use crate::{config::Config, output::Output, theme::ThemeValue, Cli, Command};

pub struct Settings {
    pub lang: Language,
    pub highlight_names: Vec<String>,
    pub highlight_styles: Vec<ThemeValue>,

    pub highlights_query: String,
    pub injection_query: String,
    pub locals_query: String,
}

pub fn get_settings(config: Config, subcommand: &Command) -> Result<Settings> {
    let mut highlight_names = Vec::with_capacity(config.theme.len());
    let mut highlight_styles = Vec::with_capacity(config.theme.len());
    for (key, value) in config.theme.into_iter() {
        highlight_names.push(key);
        highlight_styles.push(value);
    }

    let mut loader = Loader::new()?;
    loader.configure_highlights(&highlight_names);
    loader.find_all_languages(&tree_sitter_loader::Config {
        parser_directories: config.parser_search_dirs,
    })?;

    let (lang, lang_config) = match match &subcommand {
        Command::TreeSitter { file, .. } => loader.language_configuration_for_file_name(file)?,
        Command::Inline { file_ext, .. } => loader
            .language_configuration_for_file_name(&PathBuf::from(format!("file.{file_ext}")))?,
        Command::Ansi { .. } => panic!("`ts::get_settings` called with `ansi` subcommand"),
        Command::TexInclude => unreachable!("`tex-include` subcommand immediately returns"),
    } {
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
    for glob_str in &config.query_search_dirs {
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
                locals_query = fs::read_to_string(&locals_file)
                    .with_context(|| format!("Could not read {}", locals_file.to_string_lossy()))?;
            }
        }
    }

    Ok(Settings {
        lang,
        highlight_names,
        highlight_styles,
        highlights_query,
        injection_query,
        locals_query,
    })
}

pub fn highlight(
    code: &str,
    line_numbers: Option<Vec<RangeInclusive<usize>>>,
    cli: &Cli,
    mut settings: Settings,
    file_name: Option<String>,
) -> Result<String> {
    let inline = matches!(&cli.subcommand, Command::Inline { .. });
    let mut output = match line_numbers {
        Some(numbers) => Output::new(
            numbers.into_iter().flatten(),
            inline,
            &cli.fancyvrb_args,
            file_name,
            cli.size(),
        ),
        None => Output::new(1.., inline, &cli.fancyvrb_args, file_name, cli.size()),
    };

    if !matches!(
        &cli.subcommand,
        Command::TreeSitter {
            raw_queries: true,
            ..
        }
    ) {
        settings.highlights_query = process_queries(settings.lang, &settings.highlights_query)?;
        settings.injection_query = process_queries(settings.lang, &settings.injection_query)?;
        settings.locals_query = process_queries(settings.lang, &settings.locals_query)?;
    }

    let mut highlighter = Highlighter::new();
    let mut highlight_config = HighlightConfiguration::new(
        settings.lang,
        &settings.highlights_query,
        &settings.injection_query,
        &settings.locals_query,
    )?;
    highlight_config.configure(&settings.highlight_names);

    let highlights = highlighter.highlight(&highlight_config, code.as_bytes(), None, |_| None)?;
    let mut style_stack = vec![];
    for event in highlights {
        match event? {
            HighlightEvent::HighlightStart(Highlight(highlight)) => style_stack.push(highlight),
            HighlightEvent::HighlightEnd => {
                style_stack.pop();
            }
            HighlightEvent::Source { start, end } => match style_stack.last() {
                Some(highlight) => {
                    output.push_str(&settings.highlight_styles[*highlight].write(&code[start..end]))
                }
                None => output.push_str(&code[start..end].replace('{', "×{").replace('}', "×}")),
            },
        }
    }

    Ok(output.finish())
}

fn process_queries(lang: Language, source: &str) -> Result<String> {
    let query = Query::new(lang, source)?;
    let start_bytes: Vec<_> = (0..query.pattern_count())
        .map(|index| {
            (
                query.start_byte_for_pattern(index),
                query
                    .general_predicates(index)
                    .iter()
                    .filter_map(|predicate| match predicate.operator.as_ref() {
                        "lua-match?" => Some((
                            "#lua-match?",
                            (
                                "#match?",
                                vec![
                                    clone_predicate_arg(&predicate.args[0]),
                                    QueryPredicateArg::String(match &predicate.args[1] {
                                        QueryPredicateArg::String(str) => {
                                            str.replace("%d", "\\\\d").into_boxed_str()
                                        }
                                        _ => panic!("second arg to #lua-match? must be string"),
                                    }),
                                ],
                            ),
                        )),
                        "any-of?" => Some((
                            "#any-of?",
                            (
                                "#match?",
                                vec![
                                    clone_predicate_arg(&predicate.args[0]),
                                    QueryPredicateArg::String(
                                        format!(
                                            "^({})$",
                                            predicate.args[1..]
                                                .iter()
                                                .map(|arg| match arg {
                                                    QueryPredicateArg::String(str) => str.as_ref(),
                                                    _ => panic!("args to #any-of? must be strings"),
                                                })
                                                .collect::<Vec<_>>()
                                                .join("|")
                                        )
                                        .into_boxed_str(),
                                    ),
                                ],
                            ),
                        )),
                        "contains?" => Some((
                            "#contains?",
                            (
                                "#match?",
                                vec![
                                    clone_predicate_arg(&predicate.args[0]),
                                    clone_predicate_arg(&predicate.args[1]),
                                ],
                            ),
                        )),
                        _ => None,
                    })
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect();
    let queries: String = start_bytes
        .iter()
        .enumerate()
        .map(|(index, (start, predicate_replacements))| {
            let mut q = match start_bytes.get(index + 1) {
                Some((end, _)) => &source[*start..*end],
                None => &source[*start..],
            }
            .to_string();
            for (predicate, replacement) in predicate_replacements {
                q = q.replace(
                    &format!(
                        "{}{}",
                        predicate,
                        q.split_once(predicate)
                            .expect("replacements are correctly added above")
                            .1
                            .split_once(')')
                            .expect("replacements are correctly added above")
                            .0
                    ),
                    &format!(
                        "{} {}",
                        replacement.0,
                        display_predicate_args(&query, &replacement.1)
                    ),
                );
            }
            q
        })
        .rev()
        .collect();
    Ok(queries)
}

fn clone_predicate_arg(arg: &QueryPredicateArg) -> QueryPredicateArg {
    match arg {
        QueryPredicateArg::Capture(num) => QueryPredicateArg::Capture(*num),
        QueryPredicateArg::String(str) => QueryPredicateArg::String(str.clone()),
    }
}

fn display_predicate_args(query: &Query, args: &[QueryPredicateArg]) -> String {
    args.iter()
        .map(|arg| match arg {
            QueryPredicateArg::Capture(num) => {
                format!("@{}", query.capture_names()[*num as usize])
            }
            QueryPredicateArg::String(str) => format!("\"{str}\""),
        } + " ")
        .collect()
}
