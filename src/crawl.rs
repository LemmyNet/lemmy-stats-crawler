use crate::node_info::{NodeInfo, NodeInfoWellKnown};
use crate::CLIENT;
use anyhow::{anyhow, Error};
use lemmy_api_common::site::GetSiteResponse;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Url;
use semver::Version;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

#[derive(new, Debug, Clone)]
pub struct CrawlJob {
    pub domain: String,
    pub current_distance: u8,
    params: Arc<CrawlParams>,
}

#[derive(new, Debug)]
pub struct CrawlParams {
    min_lemmy_version: Version,
    exclude_domains: HashSet<String>,
    max_distance: u8,
    crawled_instances: Mutex<HashSet<String>>,
    result_sender: UnboundedSender<CrawlResult>,
}

#[derive(Debug)]
pub struct CrawlResult {
    pub domain: String,
    pub node_info: NodeInfo,
    pub site_info: GetSiteResponse,
}

/// Regex to check that a domain is valid
static DOMAIN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^([a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,}$"#).expect("compile domain regex")
});

impl CrawlJob {
    // TODO: return an enum for crawl states,
    pub async fn crawl(self, sender: UnboundedSender<CrawlJob>) -> Result<(), Error> {
        // need to acquire and release mutex before recursing, otherwise it will deadlock
        {
            let mut crawled_instances = self.params.crawled_instances.lock().await;
            // Need this check to avoid instances being crawled multiple times. Actually the
            // crawled_instances filter below should take care of that, but its not enough).
            if crawled_instances.contains(&self.domain) {
                return Ok(());
            } else {
                crawled_instances.insert(self.domain.clone());
            }
        }

        let (node_info, site_info) = self.fetch_instance_details().await?;

        let version = Version::parse(&site_info.version)?;
        if version < self.params.min_lemmy_version {
            return Err(anyhow!("too old lemmy version {version}"));
        }

        if self.current_distance < self.params.max_distance {
            let crawled_instances = self.params.crawled_instances.lock().await;
            site_info
                .federated_instances
                .clone()
                .map(|f| f.linked)
                .unwrap_or_default()
                .into_iter()
                .filter(|i| !self.params.exclude_domains.contains(i))
                .filter(|i| !crawled_instances.contains(i))
                .filter(|i| DOMAIN_REGEX.is_match(i))
                .map(|i| CrawlJob::new(i, self.current_distance + 1, self.params.clone()))
                .for_each(|j| sender.send(j).unwrap());
        }

        let crawl_result = CrawlResult {
            domain: self.domain.clone(),
            node_info,
            site_info,
        };
        self.params.result_sender.send(crawl_result).unwrap();

        Ok(())
    }

    async fn fetch_instance_details(&self) -> Result<(NodeInfo, GetSiteResponse), Error> {
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
        if node_info.software.name != "lemmy" && node_info.software.name != "lemmybb" {
            return Err(anyhow!("wrong software {}", node_info.software.name));
        }

        let site_info = CLIENT
            .get(&format!("https://{}/api/v3/site", &self.domain))
            .send()
            .await?
            .json::<GetSiteResponse>()
            .await?;
        Ok((node_info, site_info))
    }
}
