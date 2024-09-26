use async_channel::{unbounded, Sender};
use clap::Parser;
use color_eyre::{eyre::Result, eyre::WrapErr, Help};
use glob::Job;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};
use tokio::task::JoinSet;
mod bitfield;
mod env;
mod glob;
mod substring;
mod syntax;

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

    /// Number of workers to spawn
    #[arg(short, long, default_value_t = 4)]
    workers: usize,

    /// Generate payloads of length less than or equal to the given length
    #[arg(short = 'n', long, value_name = "LENGTH")]
    target_length: Option<usize>,

    /// Syntax highlight the PowerShell output
    #[arg(short = 'H', long)]
    syntax_highlight: bool,
}

async fn generator_thunk(
    environment: Arc<BTreeMap<String, Vec<String>>>,
    job_tx: Sender<Job>,
    path: String,
) -> Result<()> {
    for (value, identifiers) in environment.iter() {
        let ss = substring::longest_common(&path, value);
        if ss.len() > 2 {
            let ss: Arc<str> = Arc::from(ss);
            for identifier in identifiers {
                job_tx
                    .send(glob::Job {
                        identifier: identifier.clone(),
                        substring: ss.clone(),
                    })
                    .await
                    .wrap_err("Failed to send job to generation worker")?;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

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

    // Use this channel to send jobs to workers
    let (job_tx, job_rx) = unbounded();

    // Workers return results through this channel
    let (tx, rx) = unbounded();

    let mut handles = JoinSet::new();

    for _ in 0..args.workers {
        handles.spawn(glob::generate(
            path.clone(),
            environment.clone(),
            job_rx.clone(),
            tx.clone(),
        ));
    }

    // prevent rx from waiting indefinitely
    drop(job_rx);
    drop(tx);

    handles.spawn(generator_thunk(environment, job_tx, path));

    while let Ok(p) = rx.recv().await {
        if let Some(length) = args.target_length {
            if p.len() > length {
                continue;
            }
        }
        if let Err(e) = writeln!(
            handle,
            "{}",
            if args.syntax_highlight && args.output.is_none() {
                syntax::highlight(&p)?
            } else {
                p
            }
        ) {
            match e.kind() {
                std::io::ErrorKind::BrokenPipe => return Ok(()),
                _ => {
                    Err(e)
                        .wrap_err("Failed to write to output file handle")
                        .suggestion(
                            "Try supplying a filename at a location where you can write to",
                        )?;
                }
            }
        }
    }

    while let Some(res) = handles.join_next().await {
        res??;
    }

    Ok(())
}
