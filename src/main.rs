use clap::Parser;
use color_eyre::{eyre::Result, eyre::WrapErr, Help};
use crossbeam::{channel::unbounded, thread};
use std::io;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    sync::Arc,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};
use syntect::util::as_24_bit_terminal_escaped;
mod bitfield;
mod glob;
mod substring;

#[derive(Parser)]
#[command(author, version, about = None)]
#[command(
    long_about = "Generate obfuscated Windows PowerShell payloads that resolve to paths by globbing environment variables."
)]
pub struct Args {
    /// Custom environment map file in YAML format
    ///
    /// For details, check out:
    /// https://github.com/lavafroth/envy-rs/wiki/Custom-Environment-Map
    #[arg(short = 'e', long, value_name = "FILE")]
    custom_environment_map: Option<String>,

    /// Output to a file instead of standard output
    #[arg(short, long, value_name = "FILE")]
    output: Option<String>,

    /// The Windows path to obfuscate
    #[arg()]
    path: String,

    /// Number of worker threads to spawn
    #[arg(short, long, default_value_t = 4)]
    threads: usize,

    /// Generate payloads of length less than or equal to the given length
    #[arg(short = 'n', long, value_name = "LENGTH")]
    target_length: Option<usize>,

    /// Syntax highlight the PowerShell output
    #[arg(short = 'H', long)]
    syntax_highlight: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let path = args.path.to_lowercase();

    let s = if let Some(filepath) = args.custom_environment_map {
        fs::read_to_string(filepath)
            .wrap_err("Failed to read custom environment map from YAML file")
            .suggestion("Try supplying a filepath that exists and can be read by you")?
    } else {
        String::from(include_str!("environment.yaml"))
    };

    let environment: Arc<HashMap<String, Vec<String>>> = Arc::new(serde_yaml::from_str(&s)?);

    let mut handle = if let Some(filepath) = args.output {
        Some(
            File::create(&filepath)
                .wrap_err(format!("Failed to create file at path {filepath}"))
                .suggestion("Try supplying a filename at a location where you can write to")?,
        )
    } else {
        None
    };

    let theme_data = include_bytes!("MonokaiDarkSoda.tmTheme");
    let mut theme_reader = io::Cursor::new(theme_data);
    let theme = ThemeSet::load_from_reader(&mut theme_reader)?;

    let syntax = SyntaxDefinition::load_from_str(
        include_str!("PowerShellSyntax.sublime-syntax"),
        false,
        None,
    )?;
    let mut syntax_set_builder = SyntaxSetBuilder::new();
    syntax_set_builder.add(syntax);
    let syntax_set = syntax_set_builder.build();

    // This should always result in Some(syntax_reference) since we
    // have explicitly sourced a PowerShell sublime-syntax file.
    let syntax_reference = syntax_set.find_syntax_by_name("PowerShell").unwrap();
    let mut hightlighter = HighlightLines::new(syntax_reference, &theme);

    // The main thread sends jobs to the workers through job_tx
    // and workers receive jobs through job_rx.
    let (job_tx, job_rx) = unbounded();

    // Workers send results back to the main thread through tx
    // and the main thread receives results from the workers through rx.
    let (tx, rx) = unbounded();

    thread::scope(|scope| -> Result<()> {
        {
            let path = path.clone();
            let environment = environment.clone();
            scope.spawn(move |_| -> Result<()> {
                for (value, identifiers) in environment.iter() {
                    let ss = substring::longest_common(&path, value).to_string();
                    if ss.len() > 2 {
                        for identifier in identifiers {
                            job_tx
                                .send(glob::Job {
                                    identifier: identifier.clone(),
                                    substring: ss.clone(),
                                })
                                .wrap_err("Failed to send job to generation thread")?;
                        }
                    }
                }
                Ok(())
            });
        }

        for _ in 0..args.threads {
            let environment = environment.clone();
            let tx = tx.clone();
            let job_rx = job_rx.clone();
            let path = path.clone();
            scope.spawn(move |_| glob::generate(path, environment, job_rx, tx));
        }

        // Once we have sent copies of channels to setup communication with the workers,
        // we must drop our own copy of the channels. Otherwise, the result receiver will
        // keep waiting for results and the program will wait indefinitely after producing
        // all of its possible outputs.
        drop(job_rx);
        drop(tx);

        for p in rx {
            if let Some(length) = args.target_length {
                if p.len() > length {
                    continue;
                }
            }
            if let Some(ref mut f) = handle {
                writeln!(f, "{p}")
                    .wrap_err("Failed to write to output file handle")
                    .suggestion("Try supplying a filename at a location where you can write to")?;
            } else if args.syntax_highlight {
                let ranges: Vec<(Style, &str)> = hightlighter.highlight_line(&p, &syntax_set)?;
                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                println!("{escaped}");
            } else {
                println!("{p}");
            }
        }
        Ok(())
    })
    .expect("Failed to begin thread scope")?;
    Ok(())
}
