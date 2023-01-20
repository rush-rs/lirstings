// TODO: write README.md
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::PathBuf,
    process,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use tree_sitter::{Language, Query, QueryPredicateArg};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_loader::{Config, Loader};

use cache::CACHE_FILE_PATH;
use config::CONFIG_FILE_PATH;

use crate::output::Output;

mod cache;
mod config;
mod output;
mod theme;

#[derive(clap::Parser, Hash)]
#[clap(author, version, about)]
pub(crate) enum Cli {
    FromFile {
        file: PathBuf,

        #[arg(short, long)]
        raw: bool,

        #[arg(long)]
        raw_queries: bool,

        #[arg(short = 'R', long, value_delimiter = ',')]
        ranges: Vec<Range>,
    },
    Inline {
        file_ext: String,
        code: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, Hash)]
struct Range {
    start: usize,
    end: usize,
}

impl FromStr for Range {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (start, end) = s
            .split_once('-')
            .with_context(|| "no `-` found in range literal")?;
        let start = start
            .parse::<usize>()
            .with_context(|| "failed to parse range start literal")?
            .checked_sub(1)
            .with_context(|| "line number 0 does not exist")?;
        let end = end
            .parse::<usize>()
            .with_context(|| "failed to parse range end literal")?
            .checked_sub(1)
            .with_context(|| "line number 0 does not exist")?;
        // TODO: validate end >= start
        Ok(Self { start, end })
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let inline = matches!(&cli, Cli::Inline { .. });

    let conf = config::read()
        .with_context(|| format!("could not read or create config file at `{CONFIG_FILE_PATH}`"))?
        .unwrap_or_else(|| {
            eprintln!("New configuration file was created at `{CONFIG_FILE_PATH}`");
            process::exit(200)
        });

    let mut cache = cache::read()
        .with_context(|| format!("could not read or create cache file at `{CACHE_FILE_PATH}`"))?;

    let (code, line_numbers) = match &cli {
        Cli::FromFile { file, ranges, .. } if ranges.is_empty() => (
            fs::read_to_string(file).with_context(|| {
                format!("Could not read input file at `{}`", file.to_string_lossy())
            })?,
            None,
        ),
        Cli::FromFile { file, ranges, .. } => {
            let raw = fs::read_to_string(file).with_context(|| {
                format!("Could not read input file at `{}`", file.to_string_lossy())
            })?;
            let lines: Vec<_> = raw.lines().collect();
            let mut code = String::new();
            let mut line_numbers = vec![];
            for (index, range) in ranges.iter().enumerate() {
                if index != 0 {
                    let indent = lines[range.start]
                        .chars()
                        .take_while(|char| *char == ' ')
                        .count();
                    code += &format!("{}// ...\n", " ".repeat(indent));
                    line_numbers.push(0..=0);
                }
                code += &lines
                    .get(range.start..=range.end)
                    .with_context(|| "range out of bounds for input file")?
                    .join("\n");
                code += "\n";
                line_numbers.push(range.start..=range.end);
            }
            (code, Some(line_numbers))
        }
        Cli::Inline { code, .. } => (code.join(" "), None),
    };

    if matches!(&cli, Cli::FromFile { raw: true, .. }) {
        print(&code.replace('{', "×{").replace('}', "×}"));
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

    let (lang, lang_config) = match match &cli {
        Cli::FromFile { file, .. } => loader.language_configuration_for_file_name(file)?,
        Cli::Inline { file_ext, .. } => loader
            .language_configuration_for_file_name(&PathBuf::from(format!("file.{}", file_ext)))?,
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
    for glob_str in &conf.query_search_dirs {
        for dir in glob::glob(glob_str)?.filter_map(Result::ok) {
            #[allow(clippy::needless_borrow)] // wrongly detected
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

    let hash_query = highlights_query.to_string() + &injection_query + &locals_query;

    if let Some(cached) = cache.get_cached(&cli, &code, &hash_query) {
        eprintln!("ts2tex: skipping generation of cached input");
        print(&cached);
        return Ok(());
    }

    let mut output = match line_numbers {
        Some(numbers) => Output::new(numbers.into_iter().flatten(), inline),
        None => Output::new(1.., inline),
    };

    if !matches!(
        &cli,
        Cli::FromFile {
            raw_queries: true,
            ..
        }
    ) {
        highlights_query = process_queries(lang, &highlights_query)?;
        injection_query = process_queries(lang, &injection_query)?;
        locals_query = process_queries(lang, &locals_query)?;
    }

    let mut highlighter = Highlighter::new();
    let mut highlight_config =
        HighlightConfiguration::new(lang, &highlights_query, &injection_query, &locals_query)?;
    highlight_config.configure(&highlight_names);

    if inline {
        output.push_str("\\Verb[commandchars=×\\{\\}]{");
    }

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
                    output.push_str(&highlight_styles[*highlight].write(&code[start..end]))
                }
                None => output.push_str(&code[start..end].replace('{', "×{").replace('}', "×}")),
            },
        }
    }

    if inline {
        output.push('}');
    }

    let output = output.finish();
    print(&output);
    eprintln!("ts2tex: written to cache");
    cache
        .set_entry(&cli, &code, &hash_query, output)
        .with_context(|| "could not update cache file")?;

    Ok(())
}

#[inline]
fn print(input: &str) {
    let mut stdout = io::stdout().lock();
    _ = stdout.write_all(input.as_bytes());
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
