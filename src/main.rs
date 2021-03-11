use anyhow::Error;
use futures::try_join;
use lemmy_stats_crawler::federated_instances::GetSiteResponse;
use lemmy_stats_crawler::node_info::NodeInfo;
use reqwest::Client;
use serde::Serialize;
use tokio::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const START_INSTANCES: [&'static str; 1] = ["lemmy.ml"];

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let start_instances = START_INSTANCES.iter().map(|s| s.to_string()).collect();
    let instance_details = crawl(start_instances).await?;
    let instance_details = cleanup(instance_details);
    let total_stats = aggregate(instance_details);

    print!("{}", serde_json::to_string(&total_stats)?);
    Ok(())
}

#[derive(Serialize)]
struct TotalStats {
    total_instances: i32,
    total_users: i64,
    total_online_users: i32,
    instance_details: Vec<InstanceDetails>,
}

fn aggregate(instance_details: Vec<InstanceDetails>) -> TotalStats {
    let mut total_instances = 0;
    let mut total_users = 0;
    let mut total_online_users = 0;
    for i in &instance_details {
        total_instances += 1;
        total_users += i.total_users;
        total_online_users += i.online_users;
    }
    TotalStats {
        total_instances,
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
    while let Some(current_instance) = pending_instances.to_owned().first() {
        crawled_instances.push(current_instance.to_owned());
        // remove curent instance from pending
        pending_instances = pending_instances
            .iter()
            .filter(|i| i != &current_instance)
            .map(|i| i.to_owned())
            .collect();

        match fetch_instance_details(&current_instance).await {
            Ok(details) => {
                instance_details.push(details.to_owned());
                // add all unknown, linked instances to pending
                for i in details.linked_instances {
                    if !crawled_instances.contains(&i) {
                        pending_instances.push(i);
                    }
                }
            }
            Err(e) => eprintln!("Failed to crawl {}: {}", current_instance, e),
        }
    }

    Ok(instance_details)
}

#[derive(Serialize, Clone)]
struct InstanceDetails {
    domain: String,
    name: String,
    icon: Option<String>,
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
