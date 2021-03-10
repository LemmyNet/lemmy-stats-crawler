use anyhow::Error;
use lemmy_stats_crawler::federated_instances::GetSiteResponse;
use lemmy_stats_crawler::node_info::NodeInfo;
use serde::Serialize;

#[derive(Default, Debug)]
struct TotalStats {
    users: i64,
    online_users: i32,
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let mut pending_instances = vec!["lemmy.ml".to_string()];
    let mut crawled_instances = vec![];
    let mut instance_details = vec![];
    let mut total_stats = TotalStats::default();
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
            total_stats.online_users += details.online_users;
            total_stats.users += details.total_users;
            // add all unknown, linked instances to pending
            for ci in details.linked_instances {
                if !crawled_instances.contains(&ci) {
                    pending_instances.push(ci);
                }
            }
        }
    }
    instance_details = instance_details
        .iter()
        .filter(|i| i.open_registrations)
        .map(|i| i.to_owned())
        .collect();
    instance_details.sort_by(|a, b| b.users_active_halfyear.cmp(&a.users_active_halfyear));
    print!("{}", serde_json::to_string(&instance_details)?);
    dbg!(total_stats);
    Ok(())
}

#[derive(Serialize, Clone)]
struct InstanceDetails {
    domain: String,
    online_users: i32,
    total_users: i64,
    users_active_halfyear: i64,
    users_active_month: i64,
    open_registrations: bool,
    #[serde(skip)]
    linked_instances: Vec<String>,
}

async fn fetch_instance_details(domain: &str) -> Result<InstanceDetails, Error> {
    dbg!(domain);

    let node_info_url = format!("https://{}/nodeinfo/2.0.json", domain);
    let node_info: NodeInfo = reqwest::get(&node_info_url).await?.json().await?;

    let site_info_url = format!("https://{}/api/v2/site", domain);
    let site_info: GetSiteResponse = reqwest::get(&site_info_url).await?.json().await?;

    Ok(InstanceDetails {
        domain: domain.to_owned(),
        online_users: site_info.online as i32,
        total_users: node_info.usage.users.total,
        users_active_halfyear: node_info.usage.users.active_halfyear,
        users_active_month: node_info.usage.users.active_month,
        open_registrations: node_info.open_registrations,
        linked_instances: site_info
            .federated_instances
            .map(|f| f.linked)
            .unwrap_or(vec![]),
    })
}
