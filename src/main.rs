use aggregate_map::AggregateMap;
use anyhow::Error;
use chrono::Utc;
use clap::Parser;
use flate2::{Compression, write::GzEncoder};
use lemmy_stats_crawler::aggregate::TotalInstanceStats;
use lemmy_stats_crawler::{
    aggregate::{
        full_instance_data, joinlemmy_instance_data, minimal_community_data, minimal_instance_data,
    },
    crawl::CrawlResult,
    start_crawl,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::{
    collections::HashMap,
    fs::{File, create_dir_all},
    io::{BufReader, Write},
    time::Duration,
};

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let file = File::open("/home/felix/Downloads/full.json")?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let instances: TotalInstanceStats<CrawlResult> = serde_json::from_reader(reader)?;

    let res: AggregateMap<HashMap<_, Vec<_>>> = instances
        .instance_details
        .into_iter()
        .map(|i| (i.site_info.discussion_languages.len(), i.domain))
        .collect();
    let res2: BTreeMap<_, _> = res.into_inner().into_iter().collect();
    println!("{}", serde_json::to_string_pretty(&res2)?);
    Ok(())
}

fn write<T: Serialize>(data: &T, file: &'static str, out_path: &str) -> Result<(), Error> {
    let mut file = File::create(format!("{}/{file}", out_path))?;
    file.write_all(serde_json::to_string_pretty(&data)?.as_bytes())?;
    Ok(())
}

fn write_compressed<T: Serialize>(
    data: &T,
    file: &'static str,
    out_path: &str,
) -> Result<(), Error> {
    let mut e = GzEncoder::new(Vec::new(), Compression::best());
    e.write_all(serde_json::to_string_pretty(&data)?.as_bytes())?;
    let compressed_bytes = e.finish()?;
    let mut file = File::create(format!("{}/{file}", out_path))?;
    file.write_all(&compressed_bytes)?;
    Ok(())
}
