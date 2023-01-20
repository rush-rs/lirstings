// TODO: configurable in config file
const COLORS: [&str; 256] = [
    "000000", "e45649", "50a14f", "cca300", "4078f2", "a626a4", "0184bc", "a0a1a7", "383a42",
    "e45649", "50a14f", "cca300", "4078f2", "a626a4", "0184bc", "ffffff", "000000", "00005f",
    "000087", "0000af", "0000d7", "0000ff", "005f00", "005f5f", "005f87", "005faf", "005fd7",
    "005fff", "008700", "00875f", "008787", "0087af", "0087d7", "0087ff", "00af00", "00af5f",
    "00af87", "00afaf", "00afd7", "00afff", "00d700", "00d75f", "00d787", "00d7af", "00d7d7",
    "00d7ff", "00ff00", "00ff5f", "00ff87", "00ffaf", "00ffd7", "00ffff", "5f0000", "5f005f",
    "5f0087", "5f00af", "5f00d7", "5f00ff", "5f5f00", "5f5f5f", "5f5f87", "5f5faf", "5f5fd7",
    "5f5fff", "5f8700", "5f875f", "5f8787", "5f87af", "5f87d7", "5f87ff", "5faf00", "5faf5f",
    "5faf87", "5fafaf", "5fafd7", "5fafff", "5fd700", "5fd75f", "5fd787", "5fd7af", "5fd7d7",
    "5fd7ff", "5fff00", "5fff5f", "5fff87", "5fffaf", "5fffd7", "5fffff", "870000", "87005f",
    "870087", "8700af", "8700d7", "8700ff", "875f00", "875f5f", "875f87", "875faf", "875fd7",
    "875fff", "878700", "87875f", "878787", "8787af", "8787d7", "8787ff", "87af00", "87af5f",
    "87af87", "87afaf", "87afd7", "87afff", "87d700", "87d75f", "87d787", "87d7af", "87d7d7",
    "87d7ff", "87ff00", "87ff5f", "87ff87", "87ffaf", "87ffd7", "87ffff", "af0000", "af005f",
    "af0087", "af00af", "af00d7", "af00ff", "af5f00", "af5f5f", "af5f87", "af5faf", "af5fd7",
    "af5fff", "af8700", "af875f", "af8787", "af87af", "af87d7", "af87ff", "afaf00", "afaf5f",
    "afaf87", "afafaf", "afafd7", "afafff", "afd700", "afd75f", "afd787", "afd7af", "afd7d7",
    "afd7ff", "afff00", "afff5f", "afff87", "afffaf", "afffd7", "afffff", "d70000", "d7005f",
    "d70087", "d700af", "d700d7", "d700ff", "d75f00", "d75f5f", "d75f87", "d75faf", "d75fd7",
    "d75fff", "d78700", "d7875f", "d78787", "d787af", "d787d7", "d787ff", "d7af00", "d7af5f",
    "d7af87", "d7afaf", "d7afd7", "d7afff", "d7d700", "d7d75f", "d7d787", "d7d7af", "d7d7d7",
    "d7d7ff", "d7ff00", "d7ff5f", "d7ff87", "d7ffaf", "d7ffd7", "d7ffff", "ff0000", "ff005f",
    "ff0087", "ff00af", "ff00d7", "ff00ff", "ff5f00", "ff5f5f", "ff5f87", "ff5faf", "ff5fd7",
    "ff5fff", "ff8700", "ff875f", "ff8787", "ff87af", "ff87d7", "ff87ff", "ffaf00", "ffaf5f",
    "ffaf87", "ffafaf", "ffafd7", "ffafff", "ffd700", "ffd75f", "ffd787", "ffd7af", "ffd7d7",
    "ffd7ff", "ffff00", "ffff5f", "ffff87", "ffffaf", "ffffd7", "ffffff", "080808", "121212",
    "1c1c1c", "262626", "303030", "3a3a3a", "444444", "4e4e4e", "585858", "626262", "6c6c6c",
    "767676", "808080", "8a8a8a", "949494", "9e9e9e", "a8a8a8", "b2b2b2", "bcbcbc", "c6c6c6",
    "d0d0d0", "dadada", "e4e4e4", "eeeeee",
];

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

pub fn parse(input: String, extra_args: &str) -> String {
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
                        command += &format!("×textcolor[HTML]{{{}}}{{", COLORS[*code as usize])
                    }
                    Style::FgColor(Color::Rgb(r, g, b)) => {
                        command += &format!("×textcolor[HTML]{{{r:02x}{g:02x}{b:02x}}}{{")
                    }
                    Style::BgColor(Color::Simple(code)) => {
                        command += &format!("×colorbox[HTML]{{{}}}{{", COLORS[*code as usize])
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
