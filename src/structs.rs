use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub version: String,
    pub software: NodeInfoSoftware,
    pub protocols: Vec<String>,
    pub usage: NodeInfoUsage,
    pub open_registrations: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NodeInfoSoftware {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoUsage {
    pub users: NodeInfoUsers,
    #[serde(rename(deserialize = "localPosts"))]
    pub posts: i64,
    #[serde(rename(deserialize = "localComments"))]
    pub comments: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoUsers {
    pub total: i64,
    pub active_halfyear: i64,
    pub active_month: i64,
}
