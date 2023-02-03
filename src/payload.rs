use crate::worker;
use std::collections::HashMap;

pub fn format(res: worker::Result, environment: &HashMap<String, Vec<String>>, path: &str) -> String {
    if res.job.substring == res.job.identifier {
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
    }
}
