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
