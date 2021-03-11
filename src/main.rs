use anyhow::Error;
use lemmy_stats_crawler::federated_instances::GetSiteResponse;
use lemmy_stats_crawler::node_info::NodeInfo;
use reqwest::Client;
use serde::Serialize;
use tokio::time::Duration;

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let start_instances = vec!["lemmy.ml".to_string()];
    let instance_details = crawl(start_instances).await?;
    let instance_details = cleanup(instance_details);
    let total_stats = aggregate(instance_details);

    print!("{}", serde_json::to_string(&total_stats)?);
    Ok(())
}

#[derive(Serialize)]
struct TotalStats {
    total_users: i64,
    total_online_users: i32,
    instance_details: Vec<InstanceDetails>,
}

fn aggregate(instance_details: Vec<InstanceDetails>) -> TotalStats {
    let mut total_users = 0;
    let mut total_online_users = 0;
    for i in &instance_details {
        total_users += i.total_users;
        total_online_users += i.online_users;
    }
    TotalStats {
        total_users,
        total_online_users,
        instance_details,
    }
}

fn cleanup(instance_details: Vec<InstanceDetails>) -> Vec<InstanceDetails> {
    let mut instance_details: Vec<InstanceDetails> = instance_details
        .iter()
        .filter(|i| i.open_registrations)
        .map(|i| i.to_owned())
        .collect();
    instance_details.sort_by(|a, b| b.users_active_halfyear.cmp(&a.users_active_halfyear));
    instance_details
}

async fn crawl(start_instances: Vec<String>) -> Result<Vec<InstanceDetails>, Error> {
    let mut pending_instances = start_instances;
    let mut crawled_instances = vec![];
    let mut instance_details = vec![];
    while let Some(pi) = pending_instances.iter().next() {
        crawled_instances.push(pi.to_owned());
        let current_instance_details = fetch_instance_details(&pi).await.ok();
        pending_instances = pending_instances
            .iter()
            .filter(|i| i != &pi)
            .map(|i| i.to_owned())
            .collect();

        if let Some(details) = current_instance_details {
            instance_details.push(details.to_owned());
            // add all unknown, linked instances to pending
            for ci in details.linked_instances {
                if !crawled_instances.contains(&ci) {
                    pending_instances.push(ci);
                }
            }
        }
    }

    Ok(instance_details)
}

#[derive(Serialize, Clone)]
struct InstanceDetails {
    domain: String,
    name: String,
    icon: String,
    online_users: i32,
    total_users: i64,
    users_active_halfyear: i64,
    users_active_month: i64,
    open_registrations: bool,
    linked_instances_count: i32,
    // The following fields are only used for aggregation, but not shown in output
    #[serde(skip)]
    linked_instances: Vec<String>,
}

async fn fetch_instance_details(domain: &str) -> Result<InstanceDetails, Error> {
    dbg!(domain);

    let client = Client::default();
    let timeout = Duration::from_secs(10);

    let node_info_url = format!("https://{}/nodeinfo/2.0.json", domain);
    let node_info: NodeInfo = client
        .get(&node_info_url)
        .timeout(timeout)
        .send()
        .await?
        .json()
        .await?;

    let site_info_url = format!("https://{}/api/v2/site", domain);
    let site_info: GetSiteResponse = client
        .get(&site_info_url)
        .timeout(timeout)
        .send()
        .await?
        .json()
        .await?;

    let linked_instances = site_info
        .federated_instances
        .map(|f| f.linked)
        .unwrap_or(vec![]);
    Ok(InstanceDetails {
        domain: domain.to_owned(),
        name: site_info.site_view.site.name,
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
