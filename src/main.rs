use clap::Parser;
use crossbeam::{channel::unbounded, thread};
use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::Write,
    sync::Arc,
};
mod bitfield;
mod glob;
mod substring;

#[derive(Parser)]
#[command(author, version, about = None)]
#[command(
    long_about = "Generate obfuscated Windows PowerShell payloads that resolve to paths by globbing environment variables."
)]
pub struct Args {
    #[arg(long)]
    custom_environment_map: Option<String>,
    #[arg(short, long)]
    output: Option<String>,
    #[arg()]
    path: String,
    #[arg(short, long, default_value_t = 4)]
    threads: usize,
    #[arg(long)]
    target_length: Option<usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let path = args.path.to_lowercase();

    let s = if let Some(filepath) = args.custom_environment_map {
        fs::read_to_string(filepath)?
    } else {
        String::from(include_str!("environment.yaml"))
    };

    let environment: Arc<HashMap<String, Vec<String>>> = Arc::new(serde_yaml::from_str(&s)?);

    let mut handle = if let Some(filepath) = args.output {
        Some(File::create(filepath)?)
    } else {
        None
    };

    let (job_tx, job_rx) = unbounded();
    let (tx, rx) = unbounded();

    thread::scope(|scope| {
        {
            let path = path.clone();
            let environment = environment.clone();
            scope.spawn(move |_| {
                for (value, identifiers) in environment.iter() {
                    let ss = substring::longest_common(&path, value).to_string();
                    if ss.len() > 2 {
                        for identifier in identifiers {
                            job_tx
                                .send(glob::Job {
                                    identifier: identifier.clone(),
                                    substring: ss.clone(),
                                })
                                .expect("failed to send job to generative thread.");
                        }
                    }
                }
            });
        }

        for _ in 0..args.threads {
            let environment = environment.clone();
            let tx = tx.clone();
            let job_rx = job_rx.clone();
            let path = path.clone();
            scope.spawn(move |_| glob::generate(path, environment, job_rx, tx));
        }

        drop(job_rx);
        drop(tx);

        for p in rx {
            if let Some(length) = args.target_length {
                if p.len() > length {
                    continue;
                }
            }
            if let Some(ref mut f) = handle {
                writeln!(f, "{p}").expect("failed to write to output file handle.");
            } else {
                println!("{p}");
            }
        }
    })
    .expect("failed to begin thread scope.");
    Ok(())
}
