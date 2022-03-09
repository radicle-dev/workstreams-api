use super::Workstream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Create a user struct to hold the workstreams, in case we want to expand the user information
/// stored in the API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub workstreams: HashMap<String, Workstream>,
}
