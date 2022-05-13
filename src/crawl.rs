use crate::REQUEST_TIMEOUT;
use anyhow::anyhow;
use anyhow::Error;
use lemmy_api_common::site::GetSiteResponse;
use once_cell::sync::Lazy;
use reqwest::Client;
use semver::Version;
use serde::Serialize;
use std::collections::VecDeque;

static CLIENT: Lazy<Client> = Lazy::new(Client::default);

pub async fn crawl(
    start_instances: Vec<String>,
    exclude: Vec<String>,
    max_depth: i32,
) -> Result<(Vec<InstanceDetails>, i32), Error> {
    let mut pending_instances: VecDeque<CrawlInstance> = start_instances
        .iter()
        .map(|s| CrawlInstance::new(s.to_string(), 0))
        .collect();
    let min_lemmy_version = min_lemmy_version().await?;
    let mut crawled_instances = vec![];
    let mut instance_details = vec![];
    let mut failed_instances = 0;
    while let Some(current_instance) = pending_instances.pop_back() {
        crawled_instances.push(current_instance.domain.clone());
        if current_instance.depth > max_depth || exclude.contains(&current_instance.domain) {
            continue;
        }
        match fetch_instance_details(&current_instance.domain, &min_lemmy_version).await {
            Ok(details) => {
                if let Some(federated) = &details.site_info.federated_instances.as_ref() {
                    for i in &federated.linked {
                        let is_in_crawled = crawled_instances.contains(i);
                        let is_in_pending = pending_instances.iter().any(|p| &p.domain == i);
                        if !is_in_crawled && !is_in_pending {
                            let ci = CrawlInstance::new(i.clone(), current_instance.depth + 1);
                            pending_instances.push_back(ci);
                        }
                    }
                }
                instance_details.push(details);
            }
            Err(e) => {
                failed_instances += 1;
                eprintln!("Failed to crawl {}: {}", current_instance.domain, e)
            }
        }
    }

    // Sort by active monthly users descending
    instance_details.sort_by_key(|i| {
        i.site_info
            .site_view
            .as_ref()
            .map(|s| s.counts.users_active_month)
            .unwrap_or(0)
    });
    instance_details.reverse();

    Ok((instance_details, failed_instances))
}

#[derive(Serialize, Debug)]
pub struct InstanceDetails {
    pub domain: String,
    pub site_info: GetSiteResponse,
}

struct CrawlInstance {
    domain: String,
    depth: i32,
}

impl CrawlInstance {
    pub fn new(domain: String, depth: i32) -> CrawlInstance {
        CrawlInstance { domain, depth }
    }
}

async fn fetch_instance_details(
    domain: &str,
    min_lemmy_version: &Version,
) -> Result<InstanceDetails, Error> {
    let client = Client::default();

    let site_info_url = format!("https://{}/api/v3/site", domain);
    let site_info = client
        .get(&site_info_url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?
        .json::<GetSiteResponse>()
        .await?;

    let version = Version::parse(&site_info.version)?;
    if &version < min_lemmy_version {
        return Err(anyhow!("lemmy version is too old ({})", version));
    }

    Ok(InstanceDetails {
        domain: domain.to_owned(),
        site_info,
    })
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
