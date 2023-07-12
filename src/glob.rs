use crate::bitfield::BitField;
use color_eyre::eyre::WrapErr;
use crossbeam::channel::{Receiver, Sender};
use std::{cmp::max, collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct Glob {
    pattern: Vec<State>,
    max_questionmarks: usize,
}

pub struct Job {
    pub identifier: String,
    pub substring: String,
}

struct State {
    character: Option<char>,
    has_wildcard: bool,
}

impl Glob {
    /// Constructor with pattern which can be used for matching.
    pub fn new(expression: &str) -> Glob {
        if expression.is_empty() {
            return Glob::default();
        }
        let mut pattern: Vec<State> = Vec::with_capacity(expression.len());
        let mut has_wildcard = false;
        let mut max_questionmarks: usize = 0;
        let mut questionmarks: usize = 0;
        for char in expression.chars() {
            has_wildcard = char == '*';
            if has_wildcard {
                max_questionmarks = max(max_questionmarks, questionmarks + 1);
                questionmarks = 0;
                continue;
            }
            if char == '?' {
                questionmarks += 1;
            }
            pattern.push(State {
                character: Some(char),
                has_wildcard, // previous was star
            });
        }

        pattern.push(State {
            character: None,
            has_wildcard,
        });

        Glob {
            pattern,
            max_questionmarks,
        }
    }

    /// Returns true if pattern applies to the given input string
    pub fn matches(&self, input: &str) -> bool {
        if self.pattern.is_empty() {
            return input.is_empty();
        }
        let mut index = 0;
        let mut wildcard_at = None;
        let mut question_matches = Vec::with_capacity(self.max_questionmarks);
        for input_char in input.chars() {
            match self.pattern.get(index) {
                None => return false,
                Some(p) if p.character == Some('?') => {
                    if p.has_wildcard {
                        wildcard_at = Some(index);
                    }
                    index += 1;
                    question_matches.push(input_char);
                }
                Some(p) if p.character == Some(input_char) => {
                    if p.has_wildcard {
                        wildcard_at = Some(index);
                        question_matches.clear();
                    }
                    index += 1;
                }
                Some(p) if p.has_wildcard && p.character.is_none() => return true,

                _ => match wildcard_at {
                    None => return false,
                    Some(last_wildcard_index) => {
                        if question_matches.is_empty() {
                            // Directly go back to the last wildcard
                            index = last_wildcard_index
                        } else {
                            index = match_different_set(
                                &self.pattern,
                                index,
                                &question_matches,
                                last_wildcard_index,
                            );
                            if index == last_wildcard_index {
                                question_matches.clear();
                            }
                        }
                        // Match last char again
                        let last_char = self.pattern[index].character;
                        if last_char == Some('?') || last_char == Some(input_char) {
                            index += 1;
                        }
                    }
                },
            }
        }
        self.pattern[index].character.is_none()
    }
}

fn match_different_set(
    pattern: &[State],
    prev_index: usize,
    question_matches: &[char],
    last_wildcard_index: usize,
) -> usize {
    let mut question_idx = 0;
    let mut index = last_wildcard_index;
    for prev_state in pattern[index + 1..prev_index].iter() {
        let current_char = pattern[index].character;
        if current_char == Some('?') {
            index += 1;
            continue;
        }
        let mut prev_char = prev_state.character;
        if prev_char == Some('?') {
            prev_char.replace(question_matches[question_idx]);
            question_idx += 1;
        }
        if current_char != prev_char {
            return last_wildcard_index;
        }
        index += 1;
    }
    index
}

fn matches(
    needle: &str,
    expression: &str,
    env: &Arc<HashMap<String, Vec<String>>>,
) -> Option<String> {
    let mut matched = false;
    let mut full_match = None;
    let pattern = Glob::new(expression);
    for (value, identifiers) in env.iter() {
        for identifier in identifiers {
            if pattern.matches(identifier) {
                // If there was a match earlier we return None.
                // There can be at most one full match.
                if matched {
                    return None;
                }
                if identifier.eq(needle) {
                    full_match = Some(value.to_string());
                }
                matched = true;
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
) {
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
