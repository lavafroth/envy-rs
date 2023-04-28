use clap::Parser;
use color_eyre::{eyre::Result, eyre::WrapErr, Help};
use crossbeam::{channel::unbounded, thread};
use std::{fs::File, io, io::Write, path::PathBuf};
mod bitfield;
mod env;
mod glob;
mod substring;
mod syntax;
use signal_hook::{consts::SIGPIPE, iterator::Signals};

#[derive(Parser)]
#[command(author, version, about = None)]
#[command(
    long_about = "Generate obfuscated Windows PowerShell payloads that resolve to paths by globbing environment variables."
)]
pub struct Args {
    /// A map of environment variables and corresponding identifiers
    ///
    /// [default: builtin map]
    ///
    /// For using a custom map, check out:
    /// https://github.com/lavafroth/envy-rs/wiki/Custom-Environment-Map
    #[arg(short, long, value_name = "FILE")]
    environment_map: Option<PathBuf>,

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

    let mut signals = Signals::new(&[SIGPIPE])?;

    let args = Args::parse();
    let path = args.path.to_lowercase();
    let environment = env::load_or_default(args.environment_map)?;
    let mut handle: Box<dyn io::Write> = if let Some(filepath) = args.output.clone() {
        Box::new(
            File::create(&filepath)
                .wrap_err(format!("Failed to create file at path {filepath}"))
                .suggestion("Try supplying a filename at a location where you can write to")?,
        )
    } else {
        Box::new(io::stdout())
    };
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

        scope.spawn(|_| {
            for sig in signals.forever() {
                if SIGPIPE == sig {
                    std::process::exit(1);
                }
            }
        });

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
            writeln!(
                handle,
                "{}",
                if args.syntax_highlight && args.output.is_none() {
                    syntax::highlight(&p)?
                } else {
                    p
                }
            )
            .wrap_err("Failed to write to output file handle")
            .suggestion("Try supplying a filename at a location where you can write to")?;
        }
        Ok(())
    })
    .expect("Failed to begin thread scope")?;
    Ok(())
}
