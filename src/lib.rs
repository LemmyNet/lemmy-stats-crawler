use std::time::Duration;

pub mod crawl;
pub mod federated_instances;
pub mod node_info;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const START_INSTANCES: [&'static str; 1] = ["lemmy.ml"];
pub const MAX_CRAWL_DEPTH: i32 = 2;
