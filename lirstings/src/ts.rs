use std::{collections::HashMap, ops::RangeInclusive};

use anyhow::Result;
use tree_sitter::{Query, QueryPredicateArg};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{output::OutputWriter, renderer, theme::{Theme, ThemeValue}};

use self::language_provider::{Language, LanguageProvider};

pub mod language_provider;

pub struct Settings {
    pub lang: Language,
    pub highlight_names: Vec<String>,
    pub highlight_styles: Vec<ThemeValue>,
}

pub fn get_settings(
    provider: impl LanguageProvider,
    theme: Theme,
    file_extension: &str,
) -> Result<Settings> {
    let mut highlight_names = Vec::with_capacity(theme.highlights.len());
    let mut highlight_styles = Vec::with_capacity(theme.highlights.len());
    for (key, value) in theme.highlights.into_iter() {
        highlight_names.push(key);
        highlight_styles.push(value);
    }

    Ok(Settings {
        lang: provider.provide(file_extension)?,
        highlight_names,
        highlight_styles,
    })
}

pub fn highlight<Renderer: renderer::Renderer>(
    code: &str,
    line_numbers: Option<Vec<RangeInclusive<usize>>>,
    inline: bool,
    raw_queries: bool,
    fancyvrb_args: &str,
    mut settings: Settings,
    label: Option<String>,
) -> Result<String> {
    let mut writer = match line_numbers {
        Some(numbers) => OutputWriter::<Renderer>::new(
            numbers.into_iter().flatten(),
            inline,
            fancyvrb_args,
            label,
        ),
        None => OutputWriter::new(1.., inline, fancyvrb_args, label),
    };

    if !raw_queries {
        settings.lang.highlights_query =
            process_queries(settings.lang.inner, &settings.lang.highlights_query)?;
        settings.lang.injection_query =
            process_queries(settings.lang.inner, &settings.lang.injection_query)?;
        settings.lang.locals_query =
            process_queries(settings.lang.inner, &settings.lang.locals_query)?;
    }

    let mut highlighter = Highlighter::new();
    let mut highlight_config = HighlightConfiguration::new(
        settings.lang.inner,
        &settings.lang.highlights_query,
        &settings.lang.injection_query,
        &settings.lang.locals_query,
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
                Some(highlight) => writer.push_str(
                    &settings.highlight_styles[*highlight].write::<Renderer>(&code[start..end]),
                ),
                None => writer.push_str(&Renderer::unstyled(&code[start..end])),
            },
        }
    }

    Ok(writer.finish())
}

fn process_queries(lang: tree_sitter::Language, source: &str) -> Result<String> {
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
