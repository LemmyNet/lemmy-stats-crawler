use anyhow::Error;
use clap::{Arg, Command};
use lemmy_stats_crawler::crawl::{crawl, InstanceDetails};
use lemmy_stats_crawler::{DEFAULT_MAX_CRAWL_DEPTH, DEFAULT_START_INSTANCES, EXCLUDE_INSTANCES};
use serde::Serialize;

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let matches = Command::new("Lemmy Stats Crawler")
        .arg(
            Arg::new("start-instances")
                .long("start-instances")
                .takes_value(true),
        )
        .arg(Arg::new("exclude").long("exclude").takes_value(true))
        .arg(
            Arg::new("max-crawl-depth")
                .long("max-crawl-depth")
                .takes_value(true),
        )
        .get_matches();
    let start_instances: Vec<String> = matches
        .value_of("start-instances")
        .unwrap_or(DEFAULT_START_INSTANCES)
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let exclude: Vec<String> = matches
        .value_of("exclude")
        .unwrap_or(EXCLUDE_INSTANCES)
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let max_crawl_depth: i32 = matches
        .value_of("max-crawl-depth")
        .unwrap_or(DEFAULT_MAX_CRAWL_DEPTH)
        .parse()?;

    eprintln!("Crawling...");
    let (instance_details, failed_instances) =
        crawl(start_instances, exclude, max_crawl_depth).await?;
    let total_stats = aggregate(instance_details, failed_instances);

    println!("{}", serde_json::to_string_pretty(&total_stats)?);
    Ok(())
}

#[derive(Serialize)]
struct TotalStats {
    crawled_instances: i32,
    failed_instances: i32,
    online_users: usize,
    total_users: i64,
    users_active_day: i64,
    users_active_week: i64,
    users_active_month: i64,
    users_active_halfyear: i64,
    instance_details: Vec<InstanceDetails>,
}

fn aggregate(instance_details: Vec<InstanceDetails>, failed_instances: i32) -> TotalStats {
    let mut online_users = 0;
    let mut total_users = 0;
    let mut users_active_day = 0;
    let mut users_active_week = 0;
    let mut users_active_month = 0;
    let mut users_active_halfyear = 0;
    let mut crawled_instances = 0;
    for i in &instance_details {
        crawled_instances += 1;
        online_users += i.site_info.online;
        if let Some(site_view) = &i.site_info.site_view {
            total_users += site_view.counts.users;
            users_active_day += site_view.counts.users_active_day;
            users_active_week += site_view.counts.users_active_week;
            users_active_month += site_view.counts.users_active_month;
            users_active_halfyear += site_view.counts.users_active_half_year;
        }
    }
    TotalStats {
        crawled_instances,
        failed_instances,
        online_users,
        total_users,
        users_active_day,
        users_active_week,
        users_active_halfyear,
        users_active_month,
        instance_details,
    }
}
