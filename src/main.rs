use clap::Parser;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use threadpool::ThreadPool;
mod bitfield;
mod substring;
use bitfield::BitField;

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

pub struct Job {
    identifier: String,
    substring: String,
}

pub struct JobResult {
    expression: String,
    job: Arc<Job>,
}

#[derive(Debug)]
pub struct HayStack(HashMap<String, Vec<String>>);

impl HayStack {
    pub fn matches(&self, needle: &str, expression: &str) -> bool {
        let exp = format!(
            "^{}$",
            expression
                .replace("*", ".*")
                .replace("?", ".")
                .replace("(", "\\(")
                .replace(")", "\\)")
        );
        let regexp = Regex::new(&exp).unwrap();
        let mut matches = 0;
        let mut matched = false;
        for identifiers in self.0.values() {
            for identifier in identifiers {
                if regexp.is_match(identifier) {
                    if identifier.eq(needle) {
                        matched = true;
                    }
                    matches += 1;
                }
            }
        }
        matched && matches == 1
    }

    pub fn generate(&self, job: Arc<Job>, tx: Sender<JobResult>) {
        let n = job.identifier.len();

        let mut i = BitField::new(n);
        // The case where the bitfield is all zeros,
        // it results in a simple '*' glob. This does
        // not helop us a lot because everything can
        // match that wildcard. So, we start with 1.
        i.increment();

        let variable = job.identifier.as_bytes();
        while !i.maxed() {
            let mut expression_bytes = Vec::new();
            for x in 0..n {
                if i.at(x) {
                    expression_bytes.push(variable[x]);
                } else if x > 0 && i.at(x - 1) && (i.at(x + 1) || x == n - 1) {
                    expression_bytes.push('?' as u8);
                } else if i.at(x + 1) || x == n - 1 {
                    expression_bytes.push('*' as u8);
                }
            }

            // This string parsing is safe to unwrap, it will always
            // be valid.
            let s = String::from_utf8(expression_bytes).unwrap();
            if self.matches(&job.identifier, &s) {
                tx.send(JobResult {
                    expression: s,
                    job: job.clone(),
                })
                .unwrap();
            }
            i.increment();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let path = args.path.to_lowercase();

    let s = fs::read_to_string(
        args.custom_environment_map
            .unwrap_or(String::from("environment.yaml")),
    )?;
    let environment: HashMap<String, Vec<String>> = serde_yaml::from_str(&s)?;

    let pool = ThreadPool::new(args.threads);

    let (tx, rx) = channel::<JobResult>();
    let h = Arc::new(HayStack(environment.clone()));

    for (value, identifiers) in &environment {
        let ss = substring::longest_common(&path, &value);
        if ss.len() > 2 {
            for identifier in identifiers {
                let job = Arc::new(Job {
                    identifier: identifier.to_string(),
                    substring: ss.to_string(),
                });
                let send_job = job.clone();
                let tx = tx.clone();
                let h = h.clone();
                pool.execute(move || {
                    h.generate(send_job, tx);
                })
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
                for (value, identifiers) in &environment {
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
                        if part.len() == 0 {
                            if i == 0 {
                                interpolated.push(env_crib.clone());
                            }
                            continue;
                        }
                        interpolated.push(format!("\"{}\"", part));
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
            println!("{}", payload);
            if let Some(ref mut f) = handle {
                writeln!(f, "{}", payload)?;
            }
        }
    }
    Ok(())
}
