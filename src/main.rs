use anyhow::Error;
use serde::Serialize;
use lemmy_stats_crawler::START_INSTANCES;
use lemmy_stats_crawler::crawl::{crawl, InstanceDetails};

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let start_instances = START_INSTANCES.iter().map(|s| s.to_string()).collect();

    eprintln!("Crawling...");
    let instance_details = crawl(start_instances).await?;
    let total_stats = aggregate(instance_details);

    println!("{}", serde_json::to_string(&total_stats)?);
    Ok(())
}

#[derive(Serialize)]
struct TotalStats {
    total_instances: i32,
    total_users: i64,
    total_online_users: i32,
    instance_details: Vec<InstanceDetails>,
}

fn aggregate(instance_details: Vec<InstanceDetails>) -> TotalStats {
    let mut total_instances = 0;
    let mut total_users = 0;
    let mut total_online_users = 0;
    for i in &instance_details {
        total_instances += 1;
        total_users += i.total_users;
        total_online_users += i.online_users;
    }
    TotalStats {
        total_instances,
        total_users,
        total_online_users,
        instance_details,
    }
}