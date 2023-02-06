use crate::worker;

pub fn format(res: worker::Result, path: &str) -> String {
    if res.job.substring == res.job.identifier {
        let env_crib = format!("${{env:{}}}", &res.expression);
        path.replace(&res.job.substring, &env_crib)
    } else {
        let begin = res.value.find(&res.job.substring).unwrap();

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
    }
}
