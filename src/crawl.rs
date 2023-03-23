use crate::node_info::{NodeInfo, NodeInfoWellKnown};
use crate::CLIENT;
use anyhow::{anyhow, Error};
use async_recursion::async_recursion;
use futures::future::join_all;
use lemmy_api_common::site::GetSiteResponse;
use log::debug;
use reqwest::Url;
use semver::Version;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(new)]
pub struct CrawlJob {
    domain: String,
    current_distance: i32,
    params: Arc<CrawlParams>,
}

#[derive(new)]
pub struct CrawlParams {
    min_lemmy_version: Version,
    exclude_domains: Vec<String>,
    max_depth: i32,
    crawled_instances: Arc<Mutex<HashSet<String>>>,
}

#[derive(Debug)]
pub struct CrawlResult {
    pub domain: String,
    pub node_info: NodeInfo,
    pub site_info: Option<GetSiteResponse>,
}

impl CrawlJob {
    #[async_recursion]
    pub async fn crawl(self) -> Vec<Result<CrawlResult, Error>> {
        // need to acquire and release mutex before recursing, otherwise it will deadlock
        {
            let mut crawled_instances = self.params.crawled_instances.deref().lock().await;
            if crawled_instances.contains(&self.domain) {
                return vec![];
            } else {
                crawled_instances.insert(self.domain.clone());
            }
        }

        if self.current_distance > self.params.max_depth
            || self.params.exclude_domains.contains(&self.domain)
        {
            return vec![];
        }

        debug!(
            "Starting crawl for {}, distance {}",
            &self.domain, &self.current_distance
        );
        let (node_info, site_info) = match self.fetch_instance_details().await {
            Ok(o) => o,
            Err(e) => return vec![Err(e)],
        };
        let mut crawl_result = CrawlResult {
            domain: self.domain.clone(),
            node_info,
            site_info: None,
        };

        if let Some(site_info) = site_info {
            match Version::parse(&site_info.version) {
                Ok(version) => {
                    if version < self.params.min_lemmy_version {
                        return vec![Ok(crawl_result)];
                    }
                }
                Err(e) => return vec![Err(e.into())],
            }

            let mut result = vec![];
            if let Some(federated) = &site_info.federated_instances {
                for domain in federated.linked.iter() {
                    let crawl_job = CrawlJob::new(
                        domain.clone(),
                        self.current_distance + 1,
                        self.params.clone(),
                    );
                    result.push(crawl_job.crawl());
                }
            }

            let mut result2: Vec<Result<CrawlResult, Error>> =
                join_all(result).await.into_iter().flatten().collect();
            debug!("Successfully finished crawl for {}", &self.domain);
            crawl_result.site_info = Some(site_info);
            result2.push(Ok(crawl_result));

            result2
        } else {
            vec![Ok(crawl_result)]
        }
    }

    async fn fetch_instance_details(&self) -> Result<(NodeInfo, Option<GetSiteResponse>), Error> {
        // Wait a little while to slow down the crawling and avoid too many open connections, which
        // results in "error trying to connect: dns error: Too many open files".
        sleep(Duration::from_millis(10));

        let rel_node_info: Url = Url::parse("http://nodeinfo.diaspora.software/ns/schema/2.0")
            .expect("parse nodeinfo relation url");
        let node_info_well_known = CLIENT
            .get(&format!("https://{}/.well-known/nodeinfo", &self.domain))
            .send()
            .await?
            .json::<NodeInfoWellKnown>()
            .await?;
        let node_info_url = node_info_well_known
            .links
            .into_iter()
            .find(|l| l.rel == rel_node_info)
            .ok_or_else(|| anyhow!("failed to find nodeinfo link for {}", &self.domain))?
            .href;
        let node_info = CLIENT
            .get(node_info_url)
            .send()
            .await?
            .json::<NodeInfo>()
            .await?;

        let site_info = CLIENT
            .get(&format!("https://{}/api/v3/site", &self.domain))
            .send()
            .await?
            .json::<GetSiteResponse>()
            .await
            .ok();
        Ok((node_info, site_info))
    }
}
