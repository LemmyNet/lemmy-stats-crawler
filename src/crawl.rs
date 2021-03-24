use crate::federated_instances::GetSiteResponse;
use crate::node_info::NodeInfo;
use crate::REQUEST_TIMEOUT;
use anyhow::Error;
use futures::try_join;
use reqwest::Client;
use serde::Serialize;
use std::collections::VecDeque;

pub async fn crawl(
    start_instances: Vec<String>,
    max_depth: i32,
) -> Result<(Vec<InstanceDetails>, i32), Error> {
    let mut pending_instances: VecDeque<CrawlInstance> = start_instances
        .iter()
        .map(|s| CrawlInstance::new(s.to_string(), 0))
        .collect();
    let mut crawled_instances = vec![];
    let mut instance_details = vec![];
    let mut failed_instances = 0;
    while let Some(current_instance) = pending_instances.pop_back() {
        crawled_instances.push(current_instance.domain.clone());
        if current_instance.depth > max_depth {
            continue;
        }
        match fetch_instance_details(&current_instance.domain).await {
            Ok(details) => {
                instance_details.push(details.to_owned());
                for i in details.linked_instances {
                    let is_in_crawled = crawled_instances.contains(&i);
                    let is_in_pending = pending_instances.iter().any(|p| p.domain == i);
                    if !is_in_crawled && !is_in_pending {
                        let ci = CrawlInstance::new(i, current_instance.depth + 1);
                        pending_instances.push_back(ci);
                    }
                }
            }
            Err(e) => {
                failed_instances += 1;
                eprintln!("Failed to crawl {}: {}", current_instance.domain, e)
            }
        }
    }

    // Sort by active monthly users descending
    instance_details.sort_by_key(|i| i.users_active_month);
    instance_details.reverse();
    
    Ok((instance_details, failed_instances))
}

#[derive(Serialize, Clone)]
pub struct InstanceDetails {
    pub domain: String,
    pub name: String,
    pub version: String,
    pub icon: Option<String>,
    pub online_users: i32,
    pub total_users: i64,
    pub users_active_halfyear: i64,
    pub users_active_month: i64,
    pub open_registrations: bool,
    pub linked_instances_count: i32,
    // The following fields are only used for aggregation, but not shown in output
    #[serde(skip)]
    pub linked_instances: Vec<String>,
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

async fn fetch_instance_details(domain: &str) -> Result<InstanceDetails, Error> {
    let client = Client::default();

    let node_info_url = format!("https://{}/nodeinfo/2.0.json", domain);
    let node_info_request = client.get(&node_info_url).timeout(REQUEST_TIMEOUT).send();

    let site_info_url = format!("https://{}/api/v2/site", domain);
    let site_info_request = client.get(&site_info_url).timeout(REQUEST_TIMEOUT).send();

    let (node_info, site_info) = try_join!(node_info_request, site_info_request)?;
    let node_info: NodeInfo = node_info.json().await?;
    let site_info: GetSiteResponse = site_info.json().await?;

    let linked_instances = site_info
        .federated_instances
        .map(|f| f.linked)
        .unwrap_or_default();
    Ok(InstanceDetails {
        domain: domain.to_owned(),
        name: site_info.site_view.site.name,
        version: node_info.software.version,
        icon: site_info.site_view.site.icon,
        online_users: site_info.online as i32,
        total_users: node_info.usage.users.total,
        users_active_halfyear: node_info.usage.users.active_halfyear,
        users_active_month: node_info.usage.users.active_month,
        open_registrations: node_info.open_registrations,
        linked_instances_count: linked_instances.len() as i32,
        linked_instances,
    })
}
