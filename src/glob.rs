use crate::bitfield::BitField;
use color_eyre::{eyre::WrapErr, Result};
use crossbeam::channel::{Receiver, Sender};
use std::{collections::HashMap, sync::Arc};

pub struct Job {
    pub identifier: String,
    pub substring: Arc<str>,
}

fn is_match(expression: &str, input: &str) -> bool {
    let n = input.len();
    let m = expression.len();
    let mut j = 0;
    {
        let mut i = 0;
        let mut start = None;
        let mut matched = 0;
        let expression: Vec<char> = expression.chars().collect();
        let input: Vec<char> = input.chars().collect();

        while i < n {
            if j < m && (expression[j] == '?' || expression[j] == input[i]) {
                i += 1;
                j += 1;
            } else if j < m && expression[j] == '*' {
                start = Some(j);
                matched = i;
                j += 1;
            } else if let Some(index) = start {
                j = index + 1;
                matched += 1;
                i = matched;
            } else {
                return false;
            }
        }
        while j < m && expression[j] == '*' {
            j += 1
        }
    }

    j == m
}

fn matches(needle: &str, expression: &str, env: &HashMap<String, Vec<String>>) -> Option<String> {
    let mut full_match = None;
    for (value, identifiers) in env.iter() {
        for identifier in identifiers {
            if is_match(expression, identifier) {
                // If there was a match earlier we return None.
                // There can be at most one full match.
                if full_match.is_some() {
                    return None;
                }
                if identifier.eq(needle) {
                    full_match = Some(value.to_string());
                }
            }
        }
    }
    full_match
}

pub fn generate(
    path: String,
    environment: Arc<HashMap<String, Vec<String>>>,
    job_rx: Receiver<Job>,
    tx: Sender<String>,
) -> Result<()> {
    for job in job_rx {
        let n = job.identifier.len();

        let mut i = BitField::new(n);
        // The case where the bitfield is all zeros,
        // it results in a simple '*' glob. This does
        // not helop us a lot because everything can
        // match that wildcard. So, we start with 1.
        i.increment();

        let variable = &job.identifier;
        while !i.maxed() {
            let expression: String = variable
                .chars()
                .enumerate()
                .take(n)
                .filter_map(|(x, v)| {
                    if i.at(x) {
                        Some(v)
                    } else if x > 0 && i.at(x - 1) && (i.at(x + 1) || x == n - 1) {
                        Some('?')
                    } else if i.at(x + 1) || x == n - 1 {
                        Some('*')
                    } else {
                        None
                    }
                })
                .collect();
            if let Some(value) = matches(&job.identifier, &expression, &environment) {
                tx.send(format(
                    &value,
                    &expression,
                    &job.identifier,
                    &job.substring,
                    &path,
                ))
                .wrap_err("unable to send result from worker")?;
            }
            i.increment();
        }
    }
    Ok(())
}

fn format(value: &str, expression: &str, identifier: &str, substring: &str, path: &str) -> String {
    if substring == identifier {
        let env_crib = format!("${{env:{expression}}}");
        path.replace(substring, &env_crib)
    } else {
        let begin = value.find(substring).unwrap();

        let end = begin + substring.len() - 1;
        let env_crib = format!("(\"${{env:{expression}}}\"[{begin}..{end}]-join'')");
        let mut interpolated = Vec::new();
        let parts = path.split(substring).collect::<Vec<&str>>();

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
    }
}
