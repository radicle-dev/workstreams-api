use super::Workstream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct User {
    pub workstreams: HashMap<String, Workstream>,
}
