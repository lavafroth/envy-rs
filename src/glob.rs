use crate::bitfield::BitField;
use std::sync::mpsc::Sender;
use std::collections::HashMap;
use regex::Regex;
use std::sync::Arc;

pub struct Job {
    pub identifier: String,
    pub substring: String,
}

pub struct JobResult {
    pub expression: String,
    pub job: Arc<Job>,
}

fn matches(needle: &str, expression: &str, env: &Arc<HashMap<String, Vec<String>>>) -> bool {
    let exp = format!(
        "^{}$",
        expression
            .replace('*', ".*")
            .replace('?', ".")
            .replace('(', "\\(")
            .replace(')', "\\)")
    );
    let regexp = Regex::new(&exp).unwrap();
    let mut matches = 0;
    let mut matched = false;
    for identifiers in env.values() {
        for identifier in identifiers {
            if regexp.is_match(identifier) {
                if needle.to_string().eq(identifier) {
                    matched = true;
                }
                matches += 1;
            }
        }
    }
    matched && matches == 1
}

pub fn generate(job: Job, environment: Arc<HashMap<String, Vec<String>>>, tx: Sender<JobResult>) {

    let job = Arc::new(job);
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
        for (x, v) in variable.iter().enumerate().take(n) {
            if i.at(x) {
                expression_bytes.push(*v);
            } else if x > 0 && i.at(x - 1) && (i.at(x + 1) || x == n - 1) {
                expression_bytes.push(b'?');
            } else if i.at(x + 1) || x == n - 1 {
                expression_bytes.push(b'*');
            }
        }

        // This string parsing is safe to unwrap, it will always
        // be valid.
        let s = String::from_utf8(expression_bytes).unwrap();
        if matches(&job.identifier, &s, &environment) {
            tx.send(JobResult {
                expression: s,
                job: job.clone(),
            })
            .unwrap();
        }
        i.increment();
    }
}
