use std::borrow::Cow;

pub trait Renderer {
    fn head(
        inline: bool,
        label: Option<String>,
        fancyvrb_args: &str,
        first_number: usize,
    ) -> Cow<'static, str>;
    fn newline(line_number: usize) -> Cow<'static, str>;
    fn tail(inline: bool) -> Cow<'static, str>;

    fn unstyled(text: &str) -> Cow<'static, str>;
    fn styled(
        text: &str,
        color_hex: &str,
        underline: bool,
        strikethrough: bool,
        italic: bool,
        bold: bool,
    ) -> Cow<'static, str>;
}

const LATEX_SET_COUNTER_COMMAND: &str = "×setcounter{LirstingsLineNo}";
pub struct LatexRenderer;
impl Renderer for LatexRenderer {
    fn head(
        inline: bool,
        label: Option<String>,
        fancyvrb_args: &str,
        first_number: usize,
    ) -> Cow<'static, str> {
        let label = label
            .map(|label| format!("label={{\\footnotesize {label}}},"))
            .unwrap_or_default();
        match inline {
            true => "\\Verb[commandchars=×\\{\\}]{".into(),
            false => format!("\\begin{{Verbatim}}[commandchars=×\\{{\\}},{label}{fancyvrb_args}]\n{LATEX_SET_COUNTER_COMMAND}{{{first_number}}}").into(),
        }
    }

    fn newline(line_number: usize) -> Cow<'static, str> {
        format!("\n{LATEX_SET_COUNTER_COMMAND}{{{line_number}}}").into()
    }

    fn tail(inline: bool) -> Cow<'static, str> {
        match inline {
            true => "}".into(),
            false => "\n\\end{Verbatim}".into(),
        }
    }

    fn unstyled(text: &str) -> Cow<'static, str> {
        text.replace('{', "×{").replace('}', "×}").into()
    }

    fn styled(
        text: &str,
        color_hex: &str,
        underline: bool,
        strikethrough: bool,
        italic: bool,
        bold: bool,
    ) -> Cow<'static, str> {
        let mut out = format!("×textcolor[HTML]{{{color}}}{{", color = &color_hex[1..]);

        let mut brace_count = 1;
        if underline {
            out += "×uline{";
            brace_count += 1;
        }
        if strikethrough {
            out += "×sout{";
            brace_count += 1;
        }
        if italic {
            out += "×textit{";
            brace_count += 1;
        }
        if bold {
            out += "×textbf{";
            brace_count += 1;
        }
        out += &format!("{text}{braces}", braces = "}".repeat(brace_count));

        out.into()
    }
}

pub struct HtmlRenderer;
impl Renderer for HtmlRenderer {
    fn head(
        _inline: bool,
        _label: Option<String>,
        _fancyvrb_args: &str,
        _first_number: usize,
    ) -> Cow<'static, str> {
        "".into()
    }

    fn newline(_line_number: usize) -> Cow<'static, str> {
        "\n".into()
    }

    fn tail(_inline: bool) -> Cow<'static, str> {
        "".into()
    }

    fn unstyled(text: &str) -> Cow<'static, str> {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace(' ', "&nbsp;<wbr>")
            .replace('\n', "<br>")
            .replace('\t', "&nbsp;&nbsp;&nbsp;&nbsp;<wbr>")
            .into()
    }

    fn styled(
        text: &str,
        color_hex: &str,
        underline: bool,
        strikethrough: bool,
        italic: bool,
        bold: bool,
    ) -> Cow<'static, str> {
        let mut style = format!("color: {color_hex};");
        if underline && strikethrough {
            style += "text-decoration: underline line-through;"
        } else if underline {
            style += "text-decoration: underline;"
        } else if strikethrough {
            style += "text-decoration: line-through;"
        }
        if bold {
            style += "font-weight: bold;"
        }
        if italic {
            style += "font-style: italic;"
        }
        format!("<span style=\"{style}\">{text}</span>").into()
    }
}
