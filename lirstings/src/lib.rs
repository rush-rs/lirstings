use std::{any::Any, hash::Hash, iter};

use anyhow::{Context, Result};

use self::{
    cache::{CacheKey, CACHE_SKIP_MESSAGE, CACHE_WRITE_MESSAGE},
    log::Log,
    theme::Theme,
    ts::language_provider::LanguageProvider,
};
pub use self::{output::OutputWriter, range::Range, ts::language_provider};

mod ansi;
pub mod cache;
pub mod log;
mod output;
mod range;
pub mod renderer;
pub mod theme;
mod ts;

#[derive(Clone, Hash)]
pub enum Mode {
    TreeSitter {
        raw: bool,
        raw_queries: bool,
        ranges: Vec<Range>,
        label: Option<String>,
        file_extension: String,
    },
    TreeSitterInline {
        file_extension: String,
    },
    Ansi,
}

pub struct HighlightConfig<
    'args,
    'log,
    Logger: Log,
    Provider: LanguageProvider,
    AdditionalHashValue: Hash,
> {
    pub mode: Mode,
    pub theme: Theme,
    pub fancyvrb_args: &'args str,
    pub logger: &'log mut Logger,
    pub provider: Provider,
    pub additional_hash_value: AdditionalHashValue,
}

pub fn highlight<
    Renderer: renderer::Renderer + Any,
    Cache: cache::Cache,
    Logger: Log,
    Provider: LanguageProvider,
    AdditionalHashValue: Hash,
>(
    input: String,
    HighlightConfig {
        mode,
        theme: mut config,
        fancyvrb_args,
        logger,
        provider,
        additional_hash_value,
    }: HighlightConfig<Logger, Provider, AdditionalHashValue>,
) -> Result<String> {
    let mut cache = Cache::default();
    cache
        .instantiate()
        .with_context(|| "could not read or create cache file")?;

    config
        .resolve_links()
        .with_context(|| "invalid config file")?;

    let (mut code, line_numbers) = match &mode {
        Mode::TreeSitter { ranges, .. } if ranges.is_empty() => (input, None),
        Mode::Ansi | Mode::TreeSitterInline { .. } => (input, None),
        Mode::TreeSitter {
            ranges,
            file_extension,
            ..
        } => {
            let lines: Vec<_> = input.lines().collect();
            let comment_style = config.comment_map.get(file_extension);
            let mut code = String::new();
            let mut line_numbers = vec![];
            let mut prev_range = Range::default();
            // TODO: prevent panics during indexing
            for (index, range) in ranges.iter().enumerate() {
                let mut range_offset = 0;
                if index != 0 {
                    if range.inline {
                        // remove previous newline
                        code.truncate(code.len() - 1);

                        // add comment and following line
                        code += comment_style.map_or("/*", |style| &style.block.0);
                        code += " ... ";
                        code += comment_style.map_or("*/", |style| &style.block.1);
                        code += match range.start_col {
                            Some(col) => &lines[range.start][col..],
                            None => lines[range.start].trim_start(),
                        };
                        code.push('\n');

                        // set range offset
                        range_offset = 1;
                    } else {
                        // take the larger indent from...
                        let indent = range.indent_offset
                            + usize::max(
                                // ...the last line of the previous range and...
                                lines[prev_range.end]
                                    .chars()
                                    .take_while(|char| *char == ' ')
                                    .count(),
                                // ...the first line of the following range.
                                lines[range.start]
                                    .chars()
                                    .take_while(|char| *char == ' ')
                                    .count(),
                            );
                        code += &format!(
                            "{}{} ...\n",
                            " ".repeat(indent),
                            comment_style.map_or("//", |style| &style.line)
                        );
                        line_numbers.push(0..=0);
                    }
                }
                code += &lines
                    .get(range.start + range_offset..=range.end)
                    .with_context(|| "range out of bounds for input file")?
                    .iter()
                    .enumerate()
                    .map(
                        |(index, line)| match (index, range.start_col, range.end_col) {
                            (0, Some(col), _) if range_offset == 0 => &line[col..],
                            (idx, _, Some(col))
                                if idx == range.end - range.start + range_offset =>
                            {
                                &line[..col]
                            }
                            _ => line,
                        },
                    )
                    .fold(String::new(), |mut acc, line| {
                        acc += line;
                        acc.push('\n');
                        acc
                    });
                line_numbers.push(range.start + range_offset + 1..=range.end + 1);
                prev_range = *range;
            }
            (code, Some(line_numbers))
        }
    };
    let gobble = code
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.chars().take_while(|char| *char == ' ').count())
        .min()
        .unwrap_or(0);
    if gobble > 0 {
        code = code
            .lines()
            .flat_map(|line| line.chars().skip(gobble).chain(iter::once('\n')))
            .collect::<String>();
    }
    code.truncate(code.trim_end_matches('\n').len());

    let cache_key = CacheKey::new::<Renderer>(&mode, &code, &config, additional_hash_value);
    if let Some(cached) = cache.get_entry(cache_key) {
        logger.info(CACHE_SKIP_MESSAGE);
        return Ok(cached.to_owned());
    }

    let (output, cache_key) = match mode {
        Mode::Ansi => (
            ansi::highlight(code, fancyvrb_args, &config.ansi_colors),
            cache_key,
        ),
        Mode::TreeSitter {
            raw,
            raw_queries,
            label,
            file_extension,
            ..
        } => {
            if raw {
                let mut writer = match line_numbers {
                    Some(numbers) => OutputWriter::<Renderer>::new(
                        numbers.into_iter().flatten(),
                        false,
                        fancyvrb_args,
                        label,
                    ),
                    None => OutputWriter::new(1.., false, fancyvrb_args, label),
                };
                writer.push_str(&Renderer::unstyled(&code));
                (writer.finish(), cache_key)
            } else {
                let settings = ts::get_settings(provider, config.clone(), &file_extension)?;
                (
                    ts::highlight::<Renderer>(
                        &code,
                        line_numbers,
                        false,
                        raw_queries,
                        fancyvrb_args,
                        settings,
                        label,
                    )?,
                    cache_key,
                )
            }
        }
        Mode::TreeSitterInline { file_extension, .. } => {
            let settings = ts::get_settings(provider, config.clone(), &file_extension)?;
            (
                ts::highlight::<Renderer>(
                    &code,
                    line_numbers,
                    true,
                    false,
                    fancyvrb_args,
                    settings,
                    None,
                )?,
                cache_key,
            )
        }
    };

    cache
        .set_entry(cache_key, output.clone())
        .with_context(|| "could not update cache file")?;
    logger.info(CACHE_WRITE_MESSAGE);

    Ok(output)
}
