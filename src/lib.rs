#[macro_use]
extern crate derive_new;

use crate::crawl::{CrawlJob, CrawlParams, InstanceDetails};
use anyhow::Error;
use futures::future::join_all;
use once_cell::sync::Lazy;
use reqwest::{Client, ClientBuilder};
use semver::Version;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub mod crawl;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

static CLIENT: Lazy<Client> = Lazy::new(|| {
    ClientBuilder::new()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("lemmy-stats-crawler")
        .build()
        .unwrap()
});

pub async fn start_crawl(
    start_instances: Vec<String>,
    exclude_domains: Vec<String>,
    max_distance: i32,
) -> Result<Vec<InstanceDetails>, Error> {
    let params = Arc::new(CrawlParams::new(
        min_lemmy_version().await?,
        exclude_domains,
        max_distance,
        Arc::new(Mutex::new(HashSet::new())),
    ));
    let mut jobs = vec![];
    for domain in start_instances.into_iter() {
        let job = CrawlJob::new(domain, 0, params.clone());
        jobs.push(job.crawl());
    }

    // TODO: log the errors
    let mut instance_details: Vec<InstanceDetails> = join_all(jobs)
        .await
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect();

    // Sort by active monthly users descending
    instance_details.sort_unstable_by_key(|i| {
        i.site_info
            .site_view
            .as_ref()
            .map(|s| s.counts.users_active_month)
            .unwrap_or(0)
    });
    instance_details.reverse();

    Ok(instance_details)
}

/// calculate minimum allowed lemmy version based on current version. in case of current version
/// 0.16.3, the minimum from this function is 0.15.3. this is to avoid rejecting all instances on
/// the previous version when a major lemmy release is published.
async fn min_lemmy_version() -> Result<Version, Error> {
    let lemmy_version_url = "https://raw.githubusercontent.com/LemmyNet/lemmy-ansible/main/VERSION";
    let req = CLIENT
        .get(lemmy_version_url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?;
    let mut version = Version::parse(req.text().await?.trim())?;
    version.minor -= 1;
    Ok(version)
}
