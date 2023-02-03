use clap::Parser;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::Arc;
use threadpool::ThreadPool;
mod bitfield;
mod glob;
mod payload;
mod substring;
mod worker;

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

    let s = fs::read_to_string(
        args.custom_environment_map
            .unwrap_or(String::from("environment.yaml")),
    )?;

    let environment: Arc<HashMap<String, Vec<String>>> = Arc::new(serde_yaml::from_str(&s)?);

    let pool = ThreadPool::new(args.threads);

    let (tx, rx) = channel::<worker::Result>();

    for (value, identifiers) in environment.iter() {
        let ss = substring::longest_common(&path, value).to_string();
        if ss.len() > 2 {
            for identifier in identifiers {
                let tx = tx.clone();
                let environment = environment.clone();
                let job = worker::Job {
                    identifier: identifier.clone(),
                    substring: ss.clone(),
                };
                pool.execute(move || glob::generate(job, environment, tx))
            }
        }
    }

    let mut handle = if let Some(filepath) = args.output {
        Some(File::create(filepath)?)
    } else {
        None
    };

    let mut recv = rx.iter();

    while pool.active_count() != 0 && pool.queued_count() != 0 {
        if let Some(res) = recv.next() {
            let p = payload::format(res, &environment, &path);
            if let Some(length) = args.target_length {
                if p.len() > length {
                    continue;
                }
            }
            println!("{p}");
            if let Some(ref mut f) = handle {
                writeln!(f, "{p}")?;
            }
        }
    }
    Ok(())
}
