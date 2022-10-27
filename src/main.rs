use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

use clap::Parser;
use tree_sitter::{Language, Query, QueryPredicateArg};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_loader::{Config, Loader};

#[derive(serde::Deserialize)]
struct Ts2TexConfig {
    theme: HashMap<String, ThemeValue>,
    query_search_dirs: Vec<PathBuf>,
    parser_search_dirs: Vec<PathBuf>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ThemeValue {
    Color(String),
    Object {
        color: String,
        #[serde(default)]
        underline: bool,
        #[serde(default)]
        strikethrough: bool,
        #[serde(default)]
        italic: bool,
        #[serde(default)]
        bold: bool,
    },
}

impl ThemeValue {
    pub fn write(&self, text: &str) {
        let text = text.replace('{', "×{").replace('}', "×}");
        let lines: Vec<_> = text
            .lines()
            .map(|line| {
                let mut out = String::new();
                match self {
                    ThemeValue::Color(color)
                    | ThemeValue::Object {
                        color,
                        underline: false,
                        strikethrough: false,
                        italic: false,
                        bold: false,
                    } => {
                        out +=
                            &format!("×textcolor[HTML]{{{color}}}{{{line}}}", color = &color[1..])
                    }
                    ThemeValue::Object {
                        color,
                        underline,
                        strikethrough,
                        italic,
                        bold,
                    } => {
                        out += &format!("×textcolor[HTML]{{{color}}}{{", color = &color[1..]);
                        let mut brace_count = 1;
                        if *underline {
                            out += "×uline{";
                            brace_count += 1;
                        }
                        if *strikethrough {
                            out += "×sout{";
                            brace_count += 1;
                        }
                        if *italic {
                            out += "×textit{";
                            brace_count += 1;
                        }
                        if *bold {
                            out += "×textbf{";
                            brace_count += 1;
                        }
                        out += &format!("{line}{braces}", braces = "}".repeat(brace_count));
                    }
                }
                out
            })
            .collect();
        print!("{}", lines.join("\n"));
    }
}

#[derive(clap::Parser)]
struct Cli {
    file: PathBuf,

    #[arg(short, long)]
    raw: bool,

    #[arg(long)]
    raw_queries: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_file = File::open("ts2tex.json")?;
    let config_file_reader = BufReader::new(config_file);
    let config: Ts2TexConfig = serde_json::from_reader(config_file_reader)?;

    let code = fs::read_to_string(&cli.file)?;

    if cli.raw {
        print!("{}", code.replace('{', "×{").replace('}', "×}"));
        return Ok(());
    }

    let mut highlight_names = Vec::with_capacity(config.theme.len());
    let mut highlight_styles = Vec::with_capacity(config.theme.len());
    for (key, value) in config.theme.into_iter() {
        highlight_names.push(key);
        highlight_styles.push(value);
    }

    let mut loader = Loader::new()?;
    loader.configure_highlights(&highlight_names);
    loader.find_all_languages(&Config {
        parser_directories: config.parser_search_dirs,
    })?;

    let (lang, lang_config) = loader
        .language_configuration_for_file_name(&cli.file)?
        .unwrap();

    let parser_name = lang_config.scope.as_ref().unwrap().replace("source.", "");
    let mut highlights_query = String::new();
    let mut injection_query = String::new();
    let mut locals_query = String::new();
    for dir in &config.query_search_dirs {
        let filetype_dir = dir.join(&parser_name);
        let highlights_file = filetype_dir.join("highlights.scm");
        let injection_file = filetype_dir.join("injections.scm");
        let locals_file = filetype_dir.join("locals.scm");

        // TODO: check for `; inherits: x` comments
        if highlights_file.is_file() {
            highlights_query = fs::read_to_string(highlights_file)?;
        }
        if injection_file.is_file() {
            injection_query = fs::read_to_string(injection_file)?;
        }
        if locals_file.is_file() {
            locals_query = fs::read_to_string(locals_file)?;
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
                        q.split_once(predicate)
                            .unwrap()
                            .1
                            .split_once(')')
                            .unwrap()
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
