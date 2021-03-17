use anyhow::Error;
use clap::{App, Arg};
use lemmy_stats_crawler::crawl::{crawl, InstanceDetails};
use lemmy_stats_crawler::{DEFAULT_MAX_CRAWL_DEPTH, DEFAULT_START_INSTANCES};
use serde::Serialize;

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let matches = App::new("Lemmy Stats Crawler")
        .arg(
            Arg::with_name("start-instances")
                .long("start-instances")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-crawl-depth")
                .long("max-crawl-depth")
                .takes_value(true),
        )
        .get_matches();
    let start_instances: Vec<String> = matches
        .value_of("start-instances")
        .unwrap_or(DEFAULT_START_INSTANCES)
        .split(',')
        .map(|s| s.to_string())
        .collect();
    let max_crawl_depth: i32 = matches
        .value_of("max-crawl-depth")
        .unwrap_or(DEFAULT_MAX_CRAWL_DEPTH)
        .parse()?;

    eprintln!("Crawling...");
    let (instance_details, failed_instances) = crawl(start_instances, max_crawl_depth).await?;
    let total_stats = aggregate(instance_details, failed_instances);

    println!("{}", serde_json::to_string_pretty(&total_stats)?);
    Ok(())
}

#[derive(Serialize)]
struct TotalStats {
    crawled_instances: i32,
    failed_instances: i32,
    total_users: i64,
    total_online_users: i32,
    instance_details: Vec<InstanceDetails>,
}

fn aggregate(instance_details: Vec<InstanceDetails>, failed_instances: i32) -> TotalStats {
    let mut crawled_instances = 0;
    let mut total_users = 0;
    let mut total_online_users = 0;
    for i in &instance_details {
        crawled_instances += 1;
        total_users += i.total_users;
        total_online_users += i.online_users;
    }
    TotalStats {
        crawled_instances,
        failed_instances,
        total_users,
        total_online_users,
        instance_details,
    }
}
