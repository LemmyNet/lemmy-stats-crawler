use anyhow::Error;
use url::Url;
use lemmy_stats_crawler::node_info::NodeInfo;
use lemmy_stats_crawler::federated_instances::GetSiteResponse;

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let url = Url::parse("https://lemmy.ml/nodeinfo/2.0.json")?;
    let node_info: NodeInfo = reqwest::get(url).await?.json().await?;

    dbg!(node_info);

    let url = Url::parse("https://lemmy.ml/api/v2/site")?;
    let site_info: GetSiteResponse = reqwest::get(url).await?.json().await?;

    dbg!(site_info);
    Ok(())
}