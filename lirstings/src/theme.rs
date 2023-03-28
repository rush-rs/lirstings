use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::renderer;

#[derive(Serialize, Deserialize, Clone, Hash, Debug)]
pub struct Theme {
    pub highlights: BTreeMap<String, ThemeValue>,
    pub ansi_colors: Vec<String>,
    pub comment_map: BTreeMap<String, CommentStyle>,
}

#[derive(Serialize, Deserialize, Clone, Hash, Debug)]
pub struct CommentStyle {
    pub line: String,
    pub block: (String, String),
}

#[derive(Deserialize, Serialize, Clone, Hash, Debug)]
#[serde(untagged)]
pub enum ThemeValue {
    Color(String),
    Object {
        color: Option<String>,
        #[serde(default)]
        underline: bool,
        #[serde(default)]
        strikethrough: bool,
        #[serde(default)]
        italic: bool,
        #[serde(default)]
        bold: bool,
        link: Option<String>,
    },
}

impl Theme {
    pub fn resolve_links(&mut self) -> Result<()> {
        let mut must_reresolve = false;
        let mut replacements = vec![];
        for (key, value) in self.highlights.iter() {
            let link_key = match value {
                ThemeValue::Color(str) if str.starts_with('$') => &str[1..],
                ThemeValue::Object {
                    link: Some(str), ..
                } => str,
                _ => continue,
            };
            let resolved = value.linked_to(
                self.highlights
                    .get(link_key)
                    .with_context(|| format!("link to unknown key '{link_key}'"))?,
            );
            if matches!(&resolved, ThemeValue::Color(str) if str.starts_with('$'))
                || matches!(&resolved, ThemeValue::Object { link: Some(_), .. })
            {
                must_reresolve = true;
            }
            replacements.push((key.clone(), resolved));
        }
        for (key, replacement) in replacements {
            *self
                .highlights
                .get_mut(&key)
                .expect("key validity checked above") = replacement;
        }
        if must_reresolve {
            self.resolve_links()?;
        }
        Ok(())
    }
}

impl ThemeValue {
    pub fn linked_to(&self, other: &Self) -> Self {
        match (self, other) {
            (ThemeValue::Color(_), _) => other.clone(),
            (
                ThemeValue::Object {
                    color: Some(color),
                    underline,
                    strikethrough,
                    italic,
                    bold,
                    link: _,
                },
                ThemeValue::Color(_),
            )
            | (
                ThemeValue::Object {
                    color: None,
                    underline,
                    strikethrough,
                    italic,
                    bold,
                    link: _,
                },
                ThemeValue::Color(color),
            ) => Self::Object {
                color: Some(color.clone()),
                underline: *underline,
                strikethrough: *strikethrough,
                italic: *italic,
                bold: *bold,
                link: None,
            },
            (
                ThemeValue::Object {
                    color: color @ Some(_),
                    underline,
                    strikethrough,
                    italic,
                    bold,
                    link: _,
                },
                ThemeValue::Object {
                    color: _,
                    underline: other_underline,
                    strikethrough: other_strikethrough,
                    italic: other_italic,
                    bold: other_bold,
                    link,
                },
            )
            | (
                ThemeValue::Object {
                    color: None,
                    underline,
                    strikethrough,
                    italic,
                    bold,
                    link: _,
                },
                ThemeValue::Object {
                    color,
                    underline: other_underline,
                    strikethrough: other_strikethrough,
                    italic: other_italic,
                    bold: other_bold,
                    link,
                },
            ) => Self::Object {
                color: color.clone(),
                underline: *underline || *other_underline,
                strikethrough: *strikethrough || *other_strikethrough,
                italic: *italic || *other_italic,
                bold: *bold || *other_bold,
                link: link.clone(),
            },
        }
    }

    pub fn write<Renderer: renderer::Renderer>(&self, text: &str) -> String {
        let text = Renderer::unstyled(text);
        let lines: Vec<_> = text
            .lines()
            .map(|line| {
                let mut out = String::new();
                match self {
                    ThemeValue::Color(color) => {
                        out.push_str(&Renderer::styled(line, color, false, false, false, false))
                    }
                    ThemeValue::Object {
                        color: Some(color),
                        underline,
                        strikethrough,
                        italic,
                        bold,
                        link: _,
                    } => out.push_str(&Renderer::styled(
                        line,
                        color,
                        *underline,
                        *strikethrough,
                        *italic,
                        *bold,
                    )),
                    ThemeValue::Object { color: None, .. } => {}
                }
                out
            })
            .collect();
        lines.join("\n")
    }
}
