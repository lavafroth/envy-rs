use clap::Parser;
use crossbeam::sync::WaitGroup;
use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::Write,
    sync::{mpsc::channel, Arc},
};
mod bitfield;
mod glob;
mod payload;
mod substring;
mod worker;
mod wildmatch;

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

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads + 1)
        .build()?;

    let (tx, rx) = channel::<worker::Result>();

    let mut handle = if let Some(filepath) = args.output {
        Some(File::create(filepath)?)
    } else {
        None
    };

    let wg = WaitGroup::new();

    {
        let path = path.clone();
        pool.spawn(move || {
            for res in rx {
                let p = payload::format(res, &path);
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
        });
    }

    for (value, identifiers) in environment.iter() {
        let ss = substring::longest_common(&path, value).to_string();
        if ss.len() > 2 {
            for identifier in identifiers {
                let tx = tx.clone();
                let environment = environment.clone();
                let wg = wg.clone();
                let job = worker::Job {
                    identifier: identifier.clone(),
                    substring: ss.clone(),
                };
                pool.spawn(move || glob::generate(job, &environment, tx, wg))
            }
        }
    }

    wg.wait();
    Ok(())
}
