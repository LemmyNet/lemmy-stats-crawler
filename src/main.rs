use anyhow::Error;
use lemmy_stats_crawler::crawl::InstanceDetails;
use lemmy_stats_crawler::start_crawl;
use serde::Serialize;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt()]
struct Parameters {
    #[structopt(short, long, default_value = "lemmy.ml")]
    start_instances: Vec<String>,
    #[structopt(
        short,
        long,
        default_value = "ds9.lemmy.ml, enterprise.lemmy.ml, voyager.lemmy.ml, test.lemmy.ml"
    )]
    exclude_instances: Vec<String>,
    #[structopt(short, long, default_value = "20")]
    max_crawl_distance: i32,

    /// Silence all output
    #[structopt(short, long)]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let params = Parameters::from_args();

    stderrlog::new()
        .module(module_path!())
        .quiet(params.quiet)
        .verbosity(params.verbose)
        .init()?;

    eprintln!("Crawling...");
    let instance_details = start_crawl(
        params.start_instances,
        params.exclude_instances,
        params.max_crawl_distance,
    )
    .await?;
    let total_stats = aggregate(instance_details);

    println!("{}", serde_json::to_string_pretty(&total_stats)?);
    Ok(())
}

// TODO: lemmy stores these numbers in SiteAggregates, would be good to simply use that as a member
//       (to avoid many members). but SiteAggregates also has id, site_id fields
#[derive(Serialize)]
struct TotalStats {
    crawled_instances: i32,
    online_users: usize,
    total_users: i64,
    users_active_day: i64,
    users_active_week: i64,
    users_active_month: i64,
    users_active_halfyear: i64,
    instance_details: Vec<InstanceDetails>,
}

fn aggregate(instance_details: Vec<InstanceDetails>) -> TotalStats {
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
        online_users,
        total_users,
        users_active_day,
        users_active_week,
        users_active_halfyear,
        users_active_month,
        instance_details,
    }
}
