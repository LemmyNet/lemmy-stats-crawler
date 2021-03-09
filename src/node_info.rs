use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub version: String,
    pub software: NodeInfoSoftware,
    pub protocols: Vec<String>,
    pub usage: NodeInfoUsage,
    pub open_registrations: bool,
}

#[derive(Deserialize, Debug)]
pub struct NodeInfoSoftware {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoUsage {
    pub users: NodeInfoUsers,
    pub local_posts: i64,
    pub local_comments: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoUsers {
    pub total: i64,
    pub active_halfyear: i64,
    pub active_month: i64,
}
