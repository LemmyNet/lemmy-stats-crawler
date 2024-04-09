use lemmy_api_common_v019::site::{
    FederatedInstances as FederatedInstances019,
    GetFederatedInstancesResponse as GetFederatedInstancesResponse019,
    GetSiteResponse as GetSiteResponse019,
};
use reqwest::Url;
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetSiteResponse {
    V019(GetSiteResponse019),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetFederatedInstancesResponse {
    V019(GetFederatedInstancesResponse019),
}

impl GetSiteResponse {
    pub fn version(&self) -> String {
        match self {
            GetSiteResponse::V019(s) => s.version.clone(),
        }
    }

    pub fn total_users(&self) -> i64 {
        match self {
            GetSiteResponse::V019(s) => s.site_view.counts.users,
        }
    }

    pub fn users_active_day(&self) -> i64 {
        match self {
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_day,
        }
    }

    pub fn users_active_week(&self) -> i64 {
        match self {
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_week,
        }
    }

    pub fn users_active_month(&self) -> i64 {
        match self {
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_month,
        }
    }

    pub fn users_active_half_year(&self) -> i64 {
        match self {
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_half_year,
        }
    }

    pub fn actor_id(&self) -> Url {
        match self {
            GetSiteResponse::V019(s) => s.site_view.site.actor_id.inner().clone(),
        }
    }
}

impl GetFederatedInstancesResponse {
    pub fn federated_instances(&self) -> Option<FederatedInstances019> {
        match self {
            GetFederatedInstancesResponse::V019(f) => f.federated_instances.clone(),
        }
    }
}
