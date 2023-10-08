#[macro_use]
extern crate derive_new;

use crate::crawl::{CrawlJob, CrawlParams, CrawlResult};
use anyhow::Error;
use log::{debug, trace};
use once_cell::sync::Lazy;
use reqwest::redirect::Policy;
use reqwest::{Client, ClientBuilder};
use semver::Version;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, WeakUnboundedSender};
use tokio::sync::{mpsc, Mutex};

pub mod crawl;
mod node_info;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

static CLIENT: Lazy<Client> = Lazy::new(|| {
    ClientBuilder::new()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("lemmy-stats-crawler")
        .pool_idle_timeout(Some(Duration::from_millis(100)))
        .pool_max_idle_per_host(1)
        .redirect(Policy::none())
        .build()
        .expect("build reqwest client")
});

pub async fn start_crawl(
    start_instances: Vec<String>,
    exclude_domains: Vec<String>,
    jobs_count: u32,
    max_distance: u8,
) -> Result<Vec<CrawlResult>, Error> {
    let (crawl_jobs_sender, crawl_jobs_receiver) = mpsc::unbounded_channel::<CrawlJob>();
    let (results_sender, mut results_receiver) = mpsc::unbounded_channel();
    let params = Arc::new(CrawlParams::new(
        min_lemmy_version().await?,
        exclude_domains.into_iter().collect(),
        max_distance,
        Mutex::new(HashSet::new()),
        results_sender,
    ));

    let rcv = Arc::new(Mutex::new(crawl_jobs_receiver));
    let send = crawl_jobs_sender.downgrade();
    for i in 0..jobs_count {
        let rcv = rcv.clone();
        let send = send.clone();
        tokio::spawn(background_task(i, send, rcv));
    }

    for domain in start_instances.into_iter() {
        let job = CrawlJob::new(domain, 0, params.clone());
        crawl_jobs_sender.send(job).unwrap();
    }

    // give time to start background tasks
    tokio::time::sleep(Duration::from_secs(1)).await;
    drop(params);

    let mut results = vec![];
    while let Some(res) = results_receiver.recv().await {
        results.push(res);
    }

    // Sort by active monthly users descending
    results.sort_unstable_by_key(|i| i.site_info.site_view.counts.users_active_month);
    results.reverse();
    Ok(results)
}

async fn background_task(
    i: u32,
    sender: WeakUnboundedSender<CrawlJob>,
    rcv: Arc<Mutex<UnboundedReceiver<CrawlJob>>>,
) {
    loop {
        let maybe_job = {
            let mut lock = rcv.lock().await;
            lock.recv().await
        };
        if let Some(job) = maybe_job {
            let domain = job.domain.clone();
            debug!(
                "Worker {i} starting job {domain} at distance {}",
                job.current_distance
            );
            let sender = sender.upgrade().unwrap();
            let res = job.crawl(sender).await;
            if let Err(e) = res {
                trace!("Job {domain} errored with: {}", e)
            }
        } else {
            return;
        }
    }
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
