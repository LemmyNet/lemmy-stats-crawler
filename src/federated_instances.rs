use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GetSiteResponse {
    pub site_view: SiteView,
    pub online: usize,
    pub federated_instances: Option<FederatedInstances>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FederatedInstances {
    pub linked: Vec<String>,
    pub allowed: Option<Vec<String>>,
    pub blocked: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SiteView {
    pub site: Site,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Site {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub require_application: Option<bool>,
}
