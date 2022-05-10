use crate::CLIENT;
use crate::REQUEST_TIMEOUT;
use anyhow::Error;
use async_recursion::async_recursion;
use futures::future::join_all;
use lemmy_api_common::site::GetSiteResponse;
use log::info;
use semver::Version;
use serde::Serialize;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Debug)]
pub struct InstanceDetails {
    pub domain: String,
    pub site_info: GetSiteResponse,
}

#[derive(new)]
pub struct CrawlParams {
    min_lemmy_version: Version,
    exclude_domains: Vec<String>,
    max_depth: i32,
    crawled_instances: Arc<Mutex<HashSet<String>>>,
}

#[derive(new)]
pub struct CrawlJob {
    domain: String,
    current_depth: i32,
    params: Arc<CrawlParams>,
}

impl CrawlJob {
    #[async_recursion]
    pub async fn crawl(self) -> Result<Vec<Result<InstanceDetails, Error>>, Error> {
        // need to acquire and release mutix before recursing, otherwise it will deadlock
        {
            let mut crawled_instances = self.params.crawled_instances.deref().lock().await;
            if crawled_instances.contains(&self.domain) {
                return Ok(vec![]);
            } else {
                crawled_instances.insert(self.domain.clone());
            }
        }

        if self.current_depth > self.params.max_depth
            || self.params.exclude_domains.contains(&self.domain)
        {
            return Ok(vec![]);
        }
        info!("Starting crawl for {}", &self.domain);

        let site_info_url = format!("https://{}/api/v3/site", &self.domain);
        let site_info = CLIENT
            .get(&site_info_url)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?
            .json::<GetSiteResponse>()
            .await?;

        let version = Version::parse(&site_info.version)?;
        if version < self.params.min_lemmy_version {
            return Ok(vec![]);
        }

        let mut result = vec![];
        if let Some(federated) = &site_info.federated_instances {
            for domain in federated.linked.iter() {
                let crawl_job =
                    CrawlJob::new(domain.clone(), self.current_depth + 1, self.params.clone());
                result.push(crawl_job.crawl());
            }
        }

        let mut result2: Vec<Result<InstanceDetails, Error>> = join_all(result)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .flat_map(|r| r.into_iter())
            .collect();
        info!("Successfully finished crawl for {}", &self.domain);
        result2.push(Ok(InstanceDetails {
            domain: self.domain,
            site_info,
        }));

        Ok(result2)
    }
}
