use anyhow::Error;
use lemmy_stats_crawler::federated_instances::GetSiteResponse;
use lemmy_stats_crawler::node_info::NodeInfo;

#[derive(Default, Debug)]
struct TotalStats {
    users: i64,
    online_users: i32,
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let mut pending_instances = vec!["lemmy.ml".to_string()];
    let mut crawled_instances = vec![];
    let mut total_stats = TotalStats::default();
    while let Some(pi) = pending_instances.iter().next() {
        crawled_instances.push(pi.to_owned());
        let instance_details = fetch_instance_details(&pi).await.ok();
        pending_instances = pending_instances
            .iter()
            .filter(|i| i != &pi)
            .map(|i| i.to_owned())
            .collect();

        if let Some(details) = instance_details {
            total_stats.online_users += details.online_users;
            total_stats.users += details.total_users;
            // remove all which are in crawled_instances
            for ci in details.linked_instances {
                if !crawled_instances.contains(&ci) {
                    pending_instances.push(ci);
                }
            }
        }
    }
    dbg!(total_stats);
    Ok(())
}

struct InstanceDetails {
    domain: String,
    online_users: i32,
    total_users: i64,
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
        linked_instances: site_info
            .federated_instances
            .map(|f| f.linked)
            .unwrap_or(vec![]),
    })
}
