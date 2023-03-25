#[macro_use]
extern crate derive_new;

use crate::crawl::{CrawlJob, CrawlParams, CrawlResult};
use crate::node_info::{NodeInfo, NodeInfoUsage, NodeInfoUsers};
use anyhow::Error;
use lemmy_api_common::site::GetSiteResponse;
use log::{debug, trace};
use once_cell::sync::Lazy;
use reqwest::{Client, ClientBuilder};
use semver::Version;
use serde::Serialize;
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
        .build()
        .expect("build reqwest client")
});

#[derive(Serialize, Debug)]
pub struct CrawlResult2 {
    pub domain: String,
    pub site_info: GetSiteResponse,
    pub federated_counts: Option<NodeInfoUsage>,
}

pub async fn start_crawl(
    start_instances: Vec<String>,
    exclude_domains: Vec<String>,
    jobs_count: u32,
    max_distance: u8,
) -> Result<Vec<CrawlResult2>, Error> {
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

    let mut crawl_results = calculate_federated_site_aggregates(results)?;

    // Sort by active monthly users descending
    crawl_results.sort_unstable_by_key(|i| i.site_info.site_view.counts.users_active_month);
    crawl_results.reverse();
    Ok(crawl_results)
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

// TODO: not quite sure what this is doing
fn calculate_federated_site_aggregates(
    crawl_results: Vec<CrawlResult>,
) -> Result<Vec<CrawlResult2>, Error> {
    let node_info: Vec<(String, NodeInfo)> = crawl_results
        .iter()
        .map(|c| (c.domain.clone(), c.node_info.clone()))
        .collect();
    let lemmy_instances: Vec<(String, GetSiteResponse)> = crawl_results
        .into_iter()
        .map(|c| {
            let domain = c.domain;
            (domain, c.site_info)
        })
        .collect();
    let mut ret = vec![];
    for instance in &lemmy_instances {
        let federated_counts = if let Some(federated_instances) = &instance.1.federated_instances {
            node_info
                .iter()
                .filter(|i| federated_instances.linked.contains(&i.0) || i.0 == instance.0)
                .map(|i| i.1.usage.clone())
                .reduce(|a, b| NodeInfoUsage {
                    users: NodeInfoUsers {
                        total: a.users.total + b.users.total,
                        active_halfyear: a.users.active_halfyear + b.users.active_halfyear,
                        active_month: a.users.active_month + b.users.active_month,
                    },
                    posts: a.posts + b.posts,
                    comments: a.comments + b.comments,
                })
        } else {
            None
        };
        ret.push(CrawlResult2 {
            domain: instance.0.clone(),
            site_info: instance.1.clone(),
            federated_counts,
        });
    }
    Ok(ret)
}
