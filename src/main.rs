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
mod substring;
mod glob;

#[derive(Parser)]
#[command(author, version, about = None)]
#[command(
    long_about = "Generate obfuscated Windows PowerShell paths by globbing environment variables."
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

    let (tx, rx) = channel::<glob::JobResult>();

    for (value, identifiers) in environment.iter() {
        let ss = substring::longest_common(&path, value).to_string();
        if ss.len() > 2 {
            for identifier in identifiers {
                let tx = tx.clone();
                let environment = environment.clone();
                let job = glob::Job {
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

    loop {
        if pool.active_count() == 0 && pool.queued_count() == 0 {
            break;
        }

        if let Some(res) = recv.next() {
            let payload = if res.job.substring == res.job.identifier {
                let env_crib = format!("${{env:{}}}", &res.expression);
                path.replace(&res.job.substring, &env_crib)
            } else {
                let mut original_value: Option<String> = None;
                for (value, identifiers) in environment.iter() {
                    if identifiers.iter().any(|i| res.job.identifier.eq(i)) {
                        original_value = Some(value.clone());
                        break;
                    }
                }
                if let Some(value) = original_value {
                    let begin = value.find(&res.job.substring).unwrap();

                    let end = begin + res.job.substring.len() - 1;
                    let env_crib = format!(
                        "(\"${{env:{}}}\"[{}..{}]-join'')",
                        &res.expression, begin, end
                    );
                    let mut interpolated = Vec::new();
                    let parts = path.split(&res.job.substring).collect::<Vec<&str>>();

                    for (i, part) in parts.iter().enumerate() {
                        if part.is_empty() {
                            if i == 0 {
                                interpolated.push(env_crib.clone());
                            }
                            continue;
                        }
                        interpolated.push(format!("\"{part}\""));
                        if i == parts.len() - 1 {
                            continue;
                        }
                        interpolated.push(env_crib.clone());
                    }
                    interpolated.join("+")
                } else {
                    "".to_string()
                }
            };
            if let Some(length) = args.target_length {
                if payload.len() > length {
                    continue;
                }
            }
            println!("{payload}");
            if let Some(ref mut f) = handle {
                writeln!(f, "{payload}")?;
            }
        }
    }
    Ok(())
}
