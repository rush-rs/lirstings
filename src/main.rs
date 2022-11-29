// TODO: write README.md
use std::{collections::HashMap, fs, path::PathBuf, process};

use anyhow::{bail, Context, Result};
use clap::Parser;
use tree_sitter::{Language, Query, QueryPredicateArg};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_loader::{Config, Loader};

mod config;
mod theme;

#[derive(clap::Parser)]
#[clap(author, version, about)]
struct Cli {
    file: PathBuf,

    #[arg(short, long)]
    raw: bool,

    #[arg(long)]
    raw_queries: bool,
}

const CONFIG_FILE_PATH: &str = "ts2tex.json";

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let conf = config::read("ts2tex.json")
        .with_context(|| "Could not read configuration file")?
        .unwrap_or_else(|| {
            eprintln!(
                "No configuration file was found. A new one was created at `{CONFIG_FILE_PATH}"
            );
            process::exit(0)
        });

    let code = fs::read_to_string(&cli.file).with_context(|| {
        format!(
            "Could not read input file at `{}`",
            cli.file.to_string_lossy()
        )
    })?;

    if cli.raw {
        print!("{}", code.replace('{', "×{").replace('}', "×}"));
        return Ok(());
    }

    let mut highlight_names = Vec::with_capacity(conf.theme.len());
    let mut highlight_styles = Vec::with_capacity(conf.theme.len());
    for (key, value) in conf.theme.into_iter() {
        highlight_names.push(key);
        highlight_styles.push(value);
    }

    let mut loader = Loader::new()?;
    loader.configure_highlights(&highlight_names);
    loader.find_all_languages(&Config {
        parser_directories: conf.parser_search_dirs,
    })?;

    let (lang, lang_config) = match loader.language_configuration_for_file_name(&cli.file)? {
        Some(conf) => conf,
        None => {
            bail!(
                "No matching tree-sitter configuration found for language in `{}`",
                cli.file.to_string_lossy()
            );
        }
    };

    let parser_name = match lang_config.scope.as_ref() {
        Some(scope) => scope.replace("source.", ""),
        None => bail!("Parser has no scope specified"),
    };

    let mut highlights_query = String::new();
    let mut injection_query = String::new();
    let mut locals_query = String::new();
    for glob_str in &conf.query_search_dirs {
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

    if !cli.raw_queries {
        highlights_query = process_queries(lang, &highlights_query)?;
        injection_query = process_queries(lang, &injection_query)?;
        locals_query = process_queries(lang, &locals_query)?;
    }

    let mut highlighter = Highlighter::new();
    let mut highlight_config =
        HighlightConfiguration::new(lang, &highlights_query, &injection_query, &locals_query)?;
    highlight_config.configure(&highlight_names);

    let highlights = highlighter.highlight(&highlight_config, code.as_bytes(), None, |_| None)?;
    let mut style_stack = vec![];
    for event in highlights {
        match event? {
            HighlightEvent::HighlightStart(Highlight(highlight)) => style_stack.push(highlight),
            HighlightEvent::HighlightEnd => {
                style_stack.pop();
            }
            HighlightEvent::Source { start, end } => match style_stack.last() {
                Some(highlight) => highlight_styles[*highlight].write(&code[start..end]),
                None => print!(
                    "{}",
                    &code[start..end].replace('{', "×{").replace('}', "×}"),
                ),
            },
        }
    }

    Ok(())
}

fn process_queries(lang: Language, source: &str) -> anyhow::Result<String> {
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
                        match q
                            .split_once(predicate)
                            .map(|(_, v)| v.split_once(')').map(|v| v.0))
                        {
                            None | Some(None) =>
                                bail!("Invalid query: At least one query file is invalid."),
                            Some(Some(q)) => q,
                        }
                    ),
                    &format!(
                        "{} {}",
                        replacement.0,
                        display_predicate_args(&query, &replacement.1)
                    ),
                );
            }
            Ok(q)
        })
        .rev()
        .collect::<Result<String>>()?;
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
