use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ThemeValue {
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
    pub fn write(&self, text: &str) -> String {
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
        lines.join("\n")
        // print!("{}", lines.join("\n"));
    }
}
