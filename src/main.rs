use anyhow::Error;
use clap::Parser;
use lemmy_stats_crawler::crawl::CrawlResult;
use lemmy_stats_crawler::start_crawl;
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Parser)]
pub struct Parameters {
    /// List of Lemmy instance domains where the crawl should be started
    #[structopt(short, long, use_value_delimiter = true, default_value = "lemmy.ml")]
    pub start_instances: Vec<String>,
    /// List of Lemmy instance domains which should not be crawled
    #[structopt(
        short,
        long,
        use_value_delimiter = true,
        default_value = "ds9.lemmy.ml,enterprise.lemmy.ml,voyager.lemmy.ml,test.lemmy.ml"
    )]
    pub exclude_instances: Vec<String>,
    /// Prints output in machine readable JSON format
    #[structopt(long)]
    json: bool,
    /// Maximum crawl distance from start_instances
    #[structopt(short, long, default_value = "10")]
    pub max_crawl_distance: u8,
    /// Number of crawl jobs to run in parallel
    #[structopt(short, long, default_value = "100")]
    pub jobs_count: u32,
    /// Timeout for HTTP requests, in seconds
    #[structopt(short, long, default_value = "10")]
    pub timeout: u64,
    /// Log verbosity, 0 -> Error 1 -> Warn 2 -> Info 3 -> Debug 4 or higher -> Trace
    #[structopt(short, long, default_value = "2")]
    verbose: usize,
    /// Silence all output
    #[structopt(short, long)]
    quiet: bool,
    /// Generate output for joinlemmy, with unneded data filtered out (implies --json)
    #[structopt(long)]
    joinlemmy_output: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let params = Parameters::parse();
    stderrlog::new()
        .module(module_path!())
        .quiet(params.quiet)
        .verbosity(params.verbose)
        .init()?;

    eprintln!("Crawling...");
    let start_time = Instant::now();
    let instance_details = start_crawl(
        params.start_instances,
        params.exclude_instances,
        params.jobs_count,
        params.max_crawl_distance,
        Duration::from_secs(params.timeout),
    )
    .await?;
    let mut total_stats = aggregate(instance_details);

    if params.joinlemmy_output {
        total_stats.instance_details = total_stats
            .instance_details
            .into_iter()
            // Filter out instances with other registration modes (closed dont allow signups and
            // open are often abused by bots)
            .filter(|i| {
                &i.site_info
                    .site_view
                    .local_site
                    .registration_mode
                    .to_string()
                    == "RequireApplication"
            })
            // Require at least 5 monthly users
            .filter(|i| i.site_info.site_view.counts.users_active_month > 5)
            // Exclude some unnecessary data to reduce output size
            .map(|mut i| {
                i.federated_instances.federated_instances = None;
                i.site_info.admins = vec![];
                i.site_info.all_languages = vec![];
                i.site_info.discussion_languages = vec![];
                i.site_info.custom_emojis = vec![];
                i.site_info.taglines = vec![];
                i.site_info.site_view.local_site.application_question = None;
                i.site_info.site_view.local_site.legal_information = None;
                i.site_info.site_view.site.public_key = String::new();
                i
            })
            .collect();
        println!("{}", serde_json::to_value(&total_stats)?);
    } else if params.json {
        println!("{}", serde_json::to_string_pretty(&total_stats)?);
    } else {
        eprintln!("Crawl complete, took {}s", start_time.elapsed().as_secs());
        eprintln!(
            "Number of Lemmy instances: {}",
            total_stats.crawled_instances
        );
        eprintln!("Total users: {}", total_stats.total_users);
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
#[derive(Serialize)]
struct TotalStats {
    crawled_instances: i32,
    total_users: i64,
    users_active_day: i64,
    users_active_week: i64,
    users_active_month: i64,
    users_active_halfyear: i64,
    instance_details: Vec<CrawlResult>,
}

fn aggregate(instance_details: Vec<CrawlResult>) -> TotalStats {
    let mut total_users = 0;
    let mut users_active_day = 0;
    let mut users_active_week = 0;
    let mut users_active_month = 0;
    let mut users_active_halfyear = 0;
    let mut crawled_instances = 0;
    for i in &instance_details {
        crawled_instances += 1;
        total_users += i.site_info.site_view.counts.users;
        users_active_day += i.site_info.site_view.counts.users_active_day;
        users_active_week += i.site_info.site_view.counts.users_active_week;
        users_active_month += i.site_info.site_view.counts.users_active_month;
        users_active_halfyear += i.site_info.site_view.counts.users_active_half_year;
    }
    TotalStats {
        crawled_instances,
        total_users,
        users_active_day,
        users_active_week,
        users_active_halfyear,
        users_active_month,
        instance_details,
    }
}
