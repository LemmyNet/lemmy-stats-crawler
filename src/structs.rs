use lemmy_api_common_v018::lemmy_db_schema::source::instance::Instance as Instance018;
use lemmy_api_common_v018::site::{
    GetFederatedInstancesResponse as GetFederatedInstancesResponse018,
    GetSiteResponse as GetSiteResponse018,
};
use lemmy_api_common_v019::lemmy_db_schema::newtypes::InstanceId;
use lemmy_api_common_v019::lemmy_db_schema::source::instance::Instance as Instance019;
use lemmy_api_common_v019::site::{
    FederatedInstances as FederatedInstances019,
    GetFederatedInstancesResponse as GetFederatedInstancesResponse019,
    GetSiteResponse as GetSiteResponse019,
};
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
    V018(GetSiteResponse018),
    V019(GetSiteResponse019),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetFederatedInstancesResponse {
    V018(GetFederatedInstancesResponse018),
    V019(GetFederatedInstancesResponse019),
}

impl GetSiteResponse {
    pub fn version(&self) -> String {
        match self {
            GetSiteResponse::V018(s) => s.version.clone(),
            GetSiteResponse::V019(s) => s.version.clone(),
        }
    }

    pub fn total_users(&self) -> i64 {
        match self {
            GetSiteResponse::V018(s) => s.site_view.counts.users,
            GetSiteResponse::V019(s) => s.site_view.counts.users,
        }
    }

    pub fn users_active_day(&self) -> i64 {
        match self {
            GetSiteResponse::V018(s) => s.site_view.counts.users_active_day,
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_day,
        }
    }

    pub fn users_active_week(&self) -> i64 {
        match self {
            GetSiteResponse::V018(s) => s.site_view.counts.users_active_week,
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_week,
        }
    }

    pub fn users_active_month(&self) -> i64 {
        match self {
            GetSiteResponse::V018(s) => s.site_view.counts.users_active_month,
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_month,
        }
    }

    pub fn users_active_half_year(&self) -> i64 {
        match self {
            GetSiteResponse::V018(s) => s.site_view.counts.users_active_half_year,
            GetSiteResponse::V019(s) => s.site_view.counts.users_active_half_year,
        }
    }
}

impl GetFederatedInstancesResponse {
    pub fn federated_instances(&self) -> Option<FederatedInstances019> {
        match self {
            GetFederatedInstancesResponse::V018(f) => {
                f.federated_instances
                    .as_ref()
                    .map(|f| FederatedInstances019 {
                        linked: f.linked.iter().map(convert_instance).collect(),
                        allowed: vec![],
                        blocked: vec![],
                    })
            }
            GetFederatedInstancesResponse::V019(f) => f.federated_instances.clone(),
        }
    }
}

fn convert_instance(instance: &Instance018) -> Instance019 {
    Instance019 {
        // id field is private so we cant convert it
        id: InstanceId::default(),
        domain: instance.domain.clone(),
        published: instance.published.clone().and_utc(),
        updated: instance.updated.map(|u| u.and_utc()),
        software: instance.software.clone(),
        version: instance.version.clone(),
    }
}
