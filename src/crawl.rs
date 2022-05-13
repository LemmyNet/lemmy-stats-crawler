use crate::CLIENT;
use anyhow::Error;
use async_recursion::async_recursion;
use futures::future::join_all;
use lemmy_api_common::site::GetSiteResponse;
use log::debug;
use semver::Version;
use serde::Serialize;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
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

#[derive(Serialize, Debug)]
pub struct InstanceDetails {
    pub domain: String,
    pub site_info: GetSiteResponse,
}

impl CrawlJob {
    #[async_recursion]
    pub async fn crawl(self) -> Vec<Result<InstanceDetails, Error>> {
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
        let site_info = match self.fetch_instance_details().await {
            Ok(o) => o,
            Err(e) => return vec![Err(e)],
        };

        if site_info.1 < self.params.min_lemmy_version {
            return vec![];
        }

        let mut result = vec![];
        if let Some(federated) = &site_info.0.federated_instances {
            for domain in federated.linked.iter() {
                let crawl_job = CrawlJob::new(
                    domain.clone(),
                    self.current_distance + 1,
                    self.params.clone(),
                );
                result.push(crawl_job.crawl());
            }
        }

        let mut result2: Vec<Result<InstanceDetails, Error>> =
            join_all(result).await.into_iter().flatten().collect();
        debug!("Successfully finished crawl for {}", &self.domain);
        result2.push(Ok(InstanceDetails {
            domain: self.domain,
            site_info: site_info.0,
        }));

        result2
    }

    async fn fetch_instance_details(&self) -> Result<(GetSiteResponse, Version), Error> {
        let site_info_url = format!("https://{}/api/v3/site", &self.domain);
        let site_info = CLIENT
            .get(&site_info_url)
            .send()
            .await?
            .json::<GetSiteResponse>()
            .await?;
        let version = Version::parse(&site_info.version)?;
        Ok((site_info, version))
    }
}
