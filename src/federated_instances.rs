use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GetSiteResponse {
    pub online: usize,
    pub federated_instances: Option<FederatedInstances>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FederatedInstances {
    pub linked: Vec<String>,
    pub allowed: Option<Vec<String>>,
    pub blocked: Option<Vec<String>>,
}
