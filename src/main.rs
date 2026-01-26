use anyhow::Error;
use chrono::Utc;
use clap::Parser;
use lemmy_stats_crawler::{
    aggregate::{full_instance_data, joinlemmy_instance_data, minimal_instance_data},
    start_crawl,
};
use serde::Serialize;
use std::{
    fs::{create_dir_all, File},
    io::Write,
    time::Duration,
};

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
    /// Folder to write crawl results
    #[structopt(short, long, default_value = "out")]
    out_path: String,
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
    let start_time = Utc::now();
    let instance_details = start_crawl(
        params.start_instances,
        params.exclude_instances,
        params.jobs_count,
        params.max_crawl_distance,
        Duration::from_secs(params.timeout),
    )
    .await?;

    let total_stats = full_instance_data(instance_details, start_time);

    eprintln!("Writing output to {}", &params.out_path);

    write(&total_stats, "instances/full.json", &params.out_path)?;

    let joinlemmy = joinlemmy_instance_data(&total_stats);
    write(&joinlemmy, "instances/joinlemmy.json", &params.out_path)?;

    let minimal = minimal_instance_data(&total_stats);
    write(&minimal, "instances/minimal.json", &params.out_path)?;

    eprintln!("Crawl complete");
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

    Ok(())
}

fn write<T: Serialize>(data: &T, file: &'static str, out_path: &str) -> Result<(), Error> {
    create_dir_all(format!("{}/instances", out_path))?;
    let mut file = File::create(format!("{}/{file}", out_path))?;
    file.write_all(serde_json::to_string_pretty(&data)?.as_bytes())?;
    Ok(())
}
