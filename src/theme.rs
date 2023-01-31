use serde::{Deserialize, Serialize};

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

    pub fn write(&self, text: &str) -> String {
        let text = text.replace('{', "×{").replace('}', "×}");
        let lines: Vec<_> = text
            .lines()
            .map(|line| {
                let mut out = String::new();
                match self {
                    ThemeValue::Color(color)
                    | ThemeValue::Object {
                        color: Some(color),
                        underline: false,
                        strikethrough: false,
                        italic: false,
                        bold: false,
                        link: _,
                    } => {
                        out +=
                            &format!("×textcolor[HTML]{{{color}}}{{{line}}}", color = &color[1..])
                    }
                    ThemeValue::Object {
                        color: Some(color),
                        underline,
                        strikethrough,
                        italic,
                        bold,
                        link: _,
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
                    ThemeValue::Object { color: None, .. } => {}
                }
                out
            })
            .collect();
        lines.join("\n")
    }
}
