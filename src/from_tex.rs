use std::{
    collections::HashMap,
    mem,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Result;

use crate::{print, range::Range, Cli, Command};

pub fn run(file: &Path, raw_args: &str) -> Result<()> {
    // parse arguments to HashMap
    let mut args = HashMap::new();
    let mut temp_key = String::new();
    let mut temp = String::new();
    let mut brace_count = 0;
    let mut escaped = false;
    for char in raw_args.chars() {
        match char {
            '=' if brace_count == 0 => {
                temp_key = mem::take(&mut temp);
                continue;
            }
            ',' if brace_count == 0 => {
                args.insert(
                    mem::take(&mut temp_key).trim().to_owned(),
                    mem::take(&mut temp).trim().to_owned(),
                );
                continue;
            }
            '{' if !escaped => {
                brace_count += 1;
                if brace_count == 1 {
                    continue;
                }
            }
            '}' if !escaped => {
                brace_count -= 1;
                if brace_count == 0 {
                    continue;
                }
            }
            '\\' => {
                escaped = true;
                temp.push(char);
                continue;
            }
            _ => {}
        }
        temp.push(char);
        escaped = false;
    }
    if !temp_key.trim().is_empty() && !temp.trim().is_empty() {
        args.insert(
            mem::take(&mut temp_key).trim().to_owned(),
            mem::take(&mut temp).trim().to_owned(),
        );
    }

    // construct Cli struct
    let cli = Cli {
        fancyvrb_args: args.remove("fancyvrb").unwrap_or_default(),
        subcommand: if args.get("ansi").map_or(false, |val| val == "true") {
            Command::Ansi {
                file: file.to_path_buf(),
            }
        } else {
            Command::TreeSitter {
                file: file.to_path_buf(),
                raw: args.get("raw").map_or(false, |val| val == "true"),
                raw_queries: args.get("raw queries").map_or(false, |val| val == "true"),
                ranges: args.get("ranges").map_or(Ok(vec![]), |val| {
                    val.split(',').map(Range::from_str).collect()
                })?,
                filename_strip_prefix: args.remove("path prefix").map(PathBuf::from),
            }
        },
    };

    let continued = args.get("continued").map_or(false, |val| val == "true");

    // begin float or wrapfloat if set
    if let Some(float) = args.get("float") {
        print(&format!(
            "\\begin{{listing}}[{float}]{}\n",
            if continued { "\\ContinuedFloat" } else { "" },
        ));
    }
    if let Some(wrap) = args.get("wrap") {
        print(&format!(
            "\\begin{{wrapfloat}}{{listing}}{{{wrap}}}{{{}}}\n\\vspace{{-1\\baselineskip}}\n",
            args.get("wrap width")
                .map(|val| val.as_str())
                .unwrap_or("0.5\\textwidth")
        ));
    }

    // call main function
    crate::run(cli)?;

    // end (wrap)float and set caption and label
    if args.contains_key("float") || args.contains_key("wrap") {
        if let Some(caption) = args.get("caption") {
            print(&format!(
                "\n\\vspace{{-1\\baselineskip}}\\caption{{{caption}{}}}",
                if continued { " (cont.)" } else { "" },
            ));
        }
        if let Some(label) = args.get("label") {
            print(&format!("\\label{{{label}}}"));
        }
        if args.contains_key("wrap") {
            print("\n\\end{wrapfloat}");
        }
        if args.contains_key("float") {
            print("\n\\end{listing}");
        }
    }

    Ok(())
}
