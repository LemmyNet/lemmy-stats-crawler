use std::time::Duration;

pub mod federated_instances;
pub mod node_info;
pub mod crawl;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const START_INSTANCES: [&'static str; 1] = ["lemmy.ml"];
