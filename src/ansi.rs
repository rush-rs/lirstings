enum Style {
    Bold,
    Italic,
    Underline,
    FgColor(Color),
    BgColor(Color),
}

enum Color {
    Simple(u8),
    Rgb(u8, u8, u8),
}

pub fn highlight(input: String, extra_args: &str, colors: &[String]) -> String {
    let mut out =
        format!("\\begin{{Verbatim}}[commandchars=×\\{{\\}},numbers=none,{extra_args}]\n");
    let mut split = input.split('\x1b');
    out += split.next().expect("first part is always just text");

    let mut styles = vec![];

    for part in split {
        if !part.starts_with('[') {
            out += &("\x1b".to_owned() + part)
        }
        let Some((args, text)) = part.split_once('m') else {
            out += &("\x1b".to_owned() + part);
            continue;
        };

        let mut args = args[1..].split(';');
        while let Some(arg) = args.next() {
            if arg.is_empty() {
                styles.clear();
                continue;
            }
            match arg.parse::<u8>() {
                Ok(0) => styles.clear(),
                Ok(1) => styles.push(Style::Bold),
                Ok(3) => styles.push(Style::Italic),
                Ok(4) => styles.push(Style::Underline),
                Ok(22) => styles.retain(|s| !matches!(s, Style::Bold)),
                Ok(23) => styles.retain(|s| !matches!(s, Style::Italic)),
                Ok(24) => styles.retain(|s| !matches!(s, Style::Underline)),
                Ok(col @ 30..=37) => styles.push(Style::FgColor(Color::Simple(col - 30))),
                Ok(39) => styles.retain(|s| !matches!(s, Style::FgColor(_))),
                Ok(col @ 40..=47) => styles.push(Style::BgColor(Color::Simple(col - 40))),
                Ok(49) => styles.retain(|s| !matches!(s, Style::BgColor(_))),
                Ok(col @ 90..=97) => styles.push(Style::FgColor(Color::Simple(col - 82))),
                Ok(col @ 100..=107) => styles.push(Style::BgColor(Color::Simple(col - 92))),
                Ok(38) | Ok(48) => {
                    if parse_color(arg, &mut args, &mut styles).is_none() {
                        continue;
                    }
                }
                _ => continue,
            }
        }

        for line in text.split('\n') {
            let mut command = String::new();
            for style in &styles {
                match style {
                    Style::Bold => command += "×textbf{",
                    Style::Italic => command += "×textit{",
                    Style::Underline => command += "×uline{",
                    Style::FgColor(Color::Simple(code)) => {
                        command += &format!("×textcolor[HTML]{{{}}}{{", colors[*code as usize])
                    }
                    Style::FgColor(Color::Rgb(r, g, b)) => {
                        command += &format!("×textcolor[HTML]{{{r:02x}{g:02x}{b:02x}}}{{")
                    }
                    Style::BgColor(Color::Simple(code)) => {
                        command += &format!("×colorbox[HTML]{{{}}}{{", colors[*code as usize])
                    }
                    Style::BgColor(Color::Rgb(r, g, b)) => {
                        command += &format!("×colorbox[HTML]{{{r:02x}{g:02x}{b:02x}}}{{")
                    }
                }
            }
            out += &format!(
                "{command}{text}{braces}\n",
                text = line.replace('{', "×{").replace('}', "×}"),
                braces = "}".repeat(styles.len()),
            )
        }
        // remove last newline
        out.truncate(out.len() - 1);
    }

    out + "\n\\end{Verbatim}"
}

fn parse_color<'a>(
    arg: &str,
    args: &mut impl Iterator<Item = &'a str>,
    styles: &mut Vec<Style>,
) -> Option<()> {
    let arg2 = args.next()?.parse::<u8>().ok()?;
    if arg2 == 2 {
        let r = args.next()?.parse::<u8>().ok()?;
        let g = args.next()?.parse::<u8>().ok()?;
        let b = args.next()?.parse::<u8>().ok()?;
        styles.push(if arg == "38" {
            Style::FgColor(Color::Rgb(r, g, b))
        } else {
            Style::BgColor(Color::Rgb(r, g, b))
        });
    } else if arg2 == 5 {
        let arg3 = args.next()?.parse::<u8>().ok()?;
        styles.push(if arg == "38" {
            Style::FgColor(Color::Simple(arg3))
        } else {
            Style::BgColor(Color::Simple(arg3))
        });
    }

    Some(())
}
