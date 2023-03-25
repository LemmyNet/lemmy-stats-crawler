use anyhow::Error;
use lemmy_stats_crawler::{start_crawl, CrawlResult2};
use serde::Serialize;
use std::time::Instant;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct Parameters {
    /// List of Lemmy instance domains where the crawl should be started
    #[structopt(short, long, use_delimiter = true, default_value = "lemmy.ml")]
    pub start_instances: Vec<String>,
    /// List of Lemmy instance domains which should not be crawled
    #[structopt(
        short,
        long,
        use_delimiter = true,
        default_value = "ds9.lemmy.ml,enterprise.lemmy.ml,voyager.lemmy.ml,test.lemmy.ml"
    )]
    pub exclude_instances: Vec<String>,
    /// Prints output in machine readable JSON format
    #[structopt(long)]
    json: bool,
    #[structopt(short, long, default_value = "10")]
    pub max_crawl_distance: u8,
    #[structopt(long, default_value = "100")]
    pub jobs_count: u32,
    #[structopt(short, long, parse(from_occurrences))]
    verbose: usize,
    /// Silence all output
    #[structopt(short, long)]
    quiet: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let params = Parameters::from_args();
    let verbosity = if params.verbose == 0 {
        2
    } else {
        params.verbose
    };
    stderrlog::new()
        .module(module_path!())
        .quiet(params.quiet)
        .verbosity(verbosity)
        .init()?;

    eprintln!("Crawling...");
    let start_time = Instant::now();
    let instance_details = start_crawl(
        params.start_instances,
        params.exclude_instances,
        params.jobs_count,
        params.max_crawl_distance,
    )
    .await?;
    let total_stats = aggregate(instance_details);

    if params.json {
        println!("{}", serde_json::to_string_pretty(&total_stats)?);
    } else {
        eprintln!("Crawl complete, took {}s", start_time.elapsed().as_secs());
        eprintln!(
            "Number of Lemmy instances: {}",
            total_stats.crawled_instances
        );
        eprintln!("Total users: {}", total_stats.total_users);
        eprintln!("Online users: {}", total_stats.online_users);
        eprintln!(
            "Half year active users: {}",
            total_stats.users_active_halfyear
        );
        eprintln!("Monthly active users: {}", total_stats.users_active_month);
        eprintln!("Weekly active users: {}", total_stats.users_active_week);
        eprintln!("Daily active users: {}", total_stats.users_active_day);
        eprintln!();
        eprintln!("Use --json flag to get machine readable output");
    }
    Ok(())
}

// TODO: lemmy stores these numbers in SiteAggregates, would be good to simply use that as a member
//       (to avoid many members). but SiteAggregates also has id, site_id fields
#[derive(Serialize, Debug)]
struct TotalStats {
    crawled_instances: i32,
    online_users: usize,
    total_users: i64,
    users_active_day: i64,
    users_active_week: i64,
    users_active_month: i64,
    users_active_halfyear: i64,
    instance_details: Vec<CrawlResult2>,
}

fn aggregate(instance_details: Vec<CrawlResult2>) -> TotalStats {
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
        let counts = &i.site_info.site_view.counts;
        total_users += counts.users;
        users_active_day += counts.users_active_day;
        users_active_week += counts.users_active_week;
        users_active_month += counts.users_active_month;
        users_active_halfyear += counts.users_active_half_year;
    }
    TotalStats {
        crawled_instances,
        online_users,
        total_users,
        users_active_day,
        users_active_week,
        users_active_halfyear,
        users_active_month,
        instance_details,
    }
}
