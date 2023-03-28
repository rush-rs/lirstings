use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use lirstings::{
    cache::{Cache, CacheKey},
    log::{DefaultLogger, Log},
    renderer::{HtmlRenderer, LatexRenderer},
    HighlightConfig, Mode, Range,
};
use lirstings_cli::{
    config::{Config, CONFIG_FILE_PATH},
    DynamicLanguageProvider,
};
use serde::{Deserialize, Serialize};

mod from_tex;

#[derive(Parser, Hash)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(short = 'x', long, global = true, default_value = "")]
    fancyvrb_args: String,

    #[arg(short = 'F', long, global = true, default_value = "latex", value_enum)]
    output_format: OutputFormat,

    #[command(subcommand)]
    subcommand: Command,
}

#[derive(ValueEnum, Clone, Copy, Hash)]
enum OutputFormat {
    #[clap(alias = "tex")]
    Latex,
    Html,
}

#[derive(Subcommand, Hash)]
pub enum Command {
    #[command(visible_alias = "ts")]
    TreeSitter {
        file: PathBuf,

        #[arg(short, long)]
        raw: bool,

        #[arg(long)]
        raw_queries: bool,

        #[arg(short = 'R', long, value_delimiter = ',')]
        ranges: Vec<Range>,

        #[arg(short, long)]
        filename_strip_prefix: Option<PathBuf>,
    },
    Inline {
        file_ext: String,
        code: Vec<String>,
    },
    Ansi {
        file: PathBuf,
    },
    #[command(visible_aliases = ["tex", "include", "include-tex"])]
    TexInclude,
    FromTex {
        file: PathBuf,
        args: String,
    },
}

fn main() {
    if let Err(err) = run(Cli::parse()) {
        DefaultLogger.error(&err.to_string());
    }
}

fn run(cli: Cli) -> Result<()> {
    let mut config = Config::read()
        .with_context(|| {
            format!("could not read or create config file at at '{CONFIG_FILE_PATH}'")
        })?
        .unwrap_or_else(|| {
            DefaultLogger.info(&format!(
                "new configuration file was created at '{CONFIG_FILE_PATH}'"
            ));
            process::exit(200);
        });
    config
        .theme
        .resolve_links()
        .with_context(|| "invalid links in theme configuration")?;

    let mut hasher = DefaultHasher::new();
    (&cli, &config.query_search_dirs, &config.parser_search_dirs).hash(&mut hasher);
    let additional_hash_value = hasher.finish();

    let (mode, code) = match cli.subcommand {
        Command::TexInclude => {
            print(
                &include_str!("./lirstings.tex").replace(
                    "EXECUTABLE",
                    &env::current_exe()
                        .as_ref()
                        .map(|path| path.to_string_lossy())
                        .unwrap_or("lirstings".into())
                        .replace('\'', "'\"'\"'"),
                ),
            );
            return Ok(());
        }
        Command::FromTex { file, args } => return from_tex::run(&file, &args),

        Command::Ansi { file } => (Mode::Ansi, read_file(&file)?),
        Command::Inline { file_ext, code } => (
            Mode::TreeSitterInline {
                file_extension: file_ext,
            },
            code.join(" "),
        ),
        Command::TreeSitter {
            file,
            raw,
            raw_queries,
            ranges,
            filename_strip_prefix,
        } => (
            Mode::TreeSitter {
                raw,
                raw_queries,
                ranges,
                label: filename_strip_prefix.and_then(|prefix| {
                    file.strip_prefix(prefix)
                        .ok()
                        .map(|path| path.to_string_lossy().into_owned())
                }),
                file_extension: file
                    .extension()
                    .with_context(|| "file has no file extension")?
                    .to_string_lossy()
                    .into_owned(),
            },
            read_file(&file)?,
        ),
    };

    let conf = HighlightConfig {
        mode,
        theme: config.theme,
        fancyvrb_args: &cli.fancyvrb_args,
        logger: &mut DefaultLogger,
        provider: DynamicLanguageProvider {
            query_search_dirs: config.query_search_dirs,
            parser_search_dirs: config.parser_search_dirs,
        },
        additional_hash_value,
    };
    use lirstings::highlight;
    print(&match &cli.output_format {
        OutputFormat::Latex => highlight::<LatexRenderer, FileCache, _, _, _>(code, conf)?,
        OutputFormat::Html => highlight::<HtmlRenderer, FileCache, _, _, _>(code, conf)?,
    });

    Ok(())
}

#[inline]
fn print(input: &str) {
    let mut stdout = io::stdout().lock();
    _ = stdout.write_all(input.as_bytes());
}

fn read_file(path: &Path) -> Result<String> {
    let raw_code = fs::read_to_string(path)
        .with_context(|| format!("Could not read input file at '{}'", path.display()))?;
    Ok(raw_code)
}

pub const CACHE_FILE_PATH: &str = "lirstings.cache.json";

#[derive(Serialize, Deserialize, Default)]
pub struct FileCache(HashMap<CacheKey, String>);

impl Cache for FileCache {
    /// Creates a new [`Cache`] instance by trying to read
    /// or create a cache file at [`CACHE_FILE_PATH`].
    fn instantiate(&mut self) -> Result<()> {
        // either read or create a configuration file based on it's current existence
        let path = Path::new(CACHE_FILE_PATH);
        if path.exists() {
            // the file exists, it can be read
            let file = File::open(CACHE_FILE_PATH)?;
            let file_reader = BufReader::new(file);
            self.0 = serde_json::from_reader(file_reader)?;
        } else {
            // The file does not exist, therefore create a new one
            fs::create_dir_all(path.parent().unwrap())?;
            let mut file = File::create(path)?;
            let repr = serde_json::to_vec(&FileCache::default())
                .with_context(|| "could not serialize default cache")?;
            file.write_all(&repr)?;
        }
        Ok(())
    }

    fn set_entry(&mut self, key: CacheKey, value: String) -> Result<()> {
        self.0.insert(key, value);

        let repr = serde_json::to_vec(self).with_context(|| "could not serialize cache struct")?;
        fs::write(CACHE_FILE_PATH, repr).with_context(|| "could not write cache file")?;

        Ok(())
    }

    fn get_entry(&self, key: CacheKey) -> Option<&str> {
        self.0.get(&key).map(|string| string.as_str())
    }
}
