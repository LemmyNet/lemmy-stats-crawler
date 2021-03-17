use std::time::Duration;

pub mod crawl;
pub mod federated_instances;
pub mod node_info;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_START_INSTANCES: &str = "lemmy.ml";
pub const DEFAULT_MAX_CRAWL_DEPTH: &str = "1";
