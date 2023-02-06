use std::sync::Arc;

pub struct Job {
    pub identifier: String,
    pub substring: String,
}

pub struct Result {
    pub value: String,
    pub expression: String,
    pub job: Arc<Job>,
}

impl Result {
    pub fn format(&self, path: &str) -> String {
        if self.job.substring == self.job.identifier {
            let env_crib = format!("${{env:{}}}", &self.expression);
            path.replace(&self.job.substring, &env_crib)
        } else {
            let begin = self.value.find(&self.job.substring).unwrap();

            let end = begin + self.job.substring.len() - 1;
            let env_crib = format!(
                "(\"${{env:{}}}\"[{}..{}]-join'')",
                &self.expression, begin, end
            );
            let mut interpolated = Vec::new();
            let parts = path.split(&self.job.substring).collect::<Vec<&str>>();

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
}
