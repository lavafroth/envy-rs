use crate::bitfield::BitField;
use color_eyre::eyre::WrapErr;
use crossbeam::channel::{Receiver, Sender};
use std::{collections::HashMap, sync::Arc};

pub struct Glob {
    pattern: Vec<State>,
    max_questionmarks: usize,
}

pub struct Job {
    pub identifier: String,
    pub substring: String,
}

struct State {
    next_char: Option<char>,
    has_wildcard: bool,
}

impl Glob {
    /// Constructor with pattern which can be used for matching.
    pub fn new(pattern: &str) -> Glob {
        let mut simplified: Vec<State> = Vec::with_capacity(pattern.len());
        let mut has_wildcard = false;
        let mut max_questionmarks: usize = 0;
        let mut questionmarks: usize = 0;
        for char in pattern.chars() {
            has_wildcard = char == '*';
            if has_wildcard {
                max_questionmarks = std::cmp::max(max_questionmarks, questionmarks + 1);
                questionmarks = 0;
                continue;
            }
            if char == '?' {
                questionmarks += 1;
            }
            simplified.push(State {
                next_char: Some(char),
                has_wildcard, // previous was star
            });
        }

        if !pattern.is_empty() {
            let final_state = State {
                next_char: None,
                has_wildcard,
            };
            simplified.push(final_state);
        }

        Glob {
            pattern: simplified,
            max_questionmarks,
        }
    }

    /// Returns true if pattern applies to the given input string
    pub fn matches(&self, input: &str) -> bool {
        if self.pattern.is_empty() {
            return input.is_empty();
        }
        let mut pattern_idx = 0;
        const NONE: usize = usize::MAX;
        let mut last_wildcard_idx = NONE;
        let mut questionmark_matches: Vec<char> = Vec::with_capacity(self.max_questionmarks);
        for input_char in input.chars() {
            match self.pattern.get(pattern_idx) {
                None => {
                    return false;
                }
                Some(p) if p.next_char == Some('?') => {
                    if p.has_wildcard {
                        last_wildcard_idx = pattern_idx;
                    }
                    pattern_idx += 1;
                    questionmark_matches.push(input_char);
                }
                Some(p) if p.next_char == Some(input_char) => {
                    if p.has_wildcard {
                        last_wildcard_idx = pattern_idx;
                        questionmark_matches.clear();
                    }
                    pattern_idx += 1;
                }
                Some(p) if p.has_wildcard => {
                    if p.next_char.is_none() {
                        return true;
                    }
                }
                _ => {
                    if last_wildcard_idx == NONE {
                        return false;
                    }
                    if !questionmark_matches.is_empty() {
                        // Try to match a different set for questionmark
                        let mut questionmark_idx = 0;
                        let current_idx = pattern_idx;
                        pattern_idx = last_wildcard_idx;
                        for prev_state in self.pattern[last_wildcard_idx + 1..current_idx].iter() {
                            if self.pattern[pattern_idx].next_char == Some('?') {
                                pattern_idx += 1;
                                continue;
                            }
                            let mut prev_input_char = prev_state.next_char;
                            if prev_input_char == Some('?') {
                                prev_input_char = Some(questionmark_matches[questionmark_idx]);
                                questionmark_idx += 1;
                            }
                            if self.pattern[pattern_idx].next_char == prev_input_char {
                                pattern_idx += 1;
                            } else {
                                pattern_idx = last_wildcard_idx;
                                questionmark_matches.clear();
                                break;
                            }
                        }
                    } else {
                        // Directly go back to the last wildcard
                        pattern_idx = last_wildcard_idx;
                    }

                    // Match last char again
                    if self.pattern[pattern_idx].next_char == Some('?')
                        || self.pattern[pattern_idx].next_char == Some(input_char)
                    {
                        pattern_idx += 1;
                    }
                }
            }
        }
        self.pattern[pattern_idx].next_char.is_none()
    }
}

fn matches(
    needle: &str,
    expression: &str,
    env: &Arc<HashMap<String, Vec<String>>>,
) -> Option<String> {
    let mut count = 0;
    let mut matched = None;
    for (value, identifiers) in env.iter() {
        for identifier in identifiers {
            if Glob::new(expression).matches(identifier) {
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
    path: String,
    environment: Arc<HashMap<String, Vec<String>>>,
    job_rx: Receiver<Job>,
    tx: Sender<String>,
) {
    for job in job_rx {
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
                tx.send(format(
                    &value,
                    &expression,
                    &job.identifier,
                    &job.substring,
                    &path,
                ))
                .wrap_err("broken pipe or forced halt")
                .expect("unable to send result from worker");
            }
            i.increment();
        }
    }

    drop(tx);
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
