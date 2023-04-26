use color_eyre::{eyre::Result, eyre::WrapErr, Help};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;

pub fn load_or_default(path: Option<String>) -> Result<Arc<HashMap<String, Vec<String>>>> {
    let s = if let Some(filepath) = path {
        read_to_string(filepath)
            .wrap_err("Failed to read custom environment map from YAML file")
            .suggestion("Try supplying a filepath that exists and can be read by you")?
    } else {
        String::from(include_str!("environment.yaml"))
    };

    Ok(Arc::new(serde_yaml::from_str(&s)?))
}
