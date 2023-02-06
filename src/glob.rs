use crate::bitfield::BitField;
use crate::wildmatch::WildMatch;
use crate::worker;
use crossbeam::channel::{Receiver, Sender};
use std::{collections::HashMap, sync::Arc};

fn matches(
    needle: &str,
    expression: &str,
    env: &Arc<HashMap<String, Vec<String>>>,
) -> Option<String> {
    let mut count = 0;
    let mut matched = None;
    for (value, identifiers) in env.iter() {
        for identifier in identifiers {
            if WildMatch::new(expression).matches(identifier) {
                if identifier.eq(needle) {
                    matched = Some(value.to_string());
                }
                count += 1;
            }
        }
    }
    if count == 1 {
        return matched;
    }
    None
}

pub fn generate(
    environment: Arc<HashMap<String, Vec<String>>>,
    job_rx: Receiver<worker::Job>,
    tx: Sender<worker::Result>,
) {
    for job in job_rx {
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
            let expression = String::from_utf8(expression_bytes).unwrap();
            if let Some(value) = matches(&job.identifier, &expression, &environment) {
                let job = job.clone();
                tx.send(worker::Result {
                    value,
                    expression,
                    job,
                })
                .unwrap();
            }
            i.increment();
        }
    }

    drop(tx);
}
