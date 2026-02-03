use crate::structs::NodeInfo;
use anyhow::{anyhow, Error};
use flate2::bufread::GzDecoder;
use lemmy_api_common_v019::community::ListCommunitiesResponse;
use lemmy_api_common_v019::lemmy_db_views_actor::structs::CommunityView;
use lemmy_api_common_v019::site::{GetFederatedInstancesResponse, GetSiteResponse};
use log::warn;
use maxminddb::geoip2;
use maxminddb::geoip2::city::City;
use maxminddb::geoip2::city::Continent;
use maxminddb::geoip2::country::Country;
use maxminddb::Reader;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest_middleware::ClientWithMiddleware;
use semver::Version;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::join;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio::try_join;

/// Regex to check that a domain is valid
static DOMAIN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,}$").expect("compile domain regex")
});

#[derive(new, Debug, Clone)]
pub struct CrawlJob {
    pub domain: String,
    pub current_distance: u8,
    params: Arc<CrawlParams>,
}

#[derive(new, Debug)]
pub struct CrawlParams {
    min_lemmy_version: Version,
    exclude_domains: HashSet<String>,
    max_distance: u8,
    crawled_instances: Mutex<HashSet<String>>,
    result_sender: UnboundedSender<CrawlResult>,
    client: ClientWithMiddleware,
}

#[derive(Debug, Serialize, Clone)]
pub struct GeoIp<'a> {
    pub city: City<'a>,
    pub country: Country<'a>,
    pub continent: Continent<'a>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CrawlResult {
    pub domain: String,
    pub site_info: GetSiteResponse,
    pub geo_ip: Option<GeoIp<'static>>,
    pub communities: Vec<CommunityView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub linked_instances: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_instances: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_instances: Vec<String>,
}

impl CrawlJob {
    // TODO: return an enum for crawl states,
    pub async fn crawl(self, sender: UnboundedSender<CrawlJob>) -> Result<(), Error> {
        // need to acquire and release mutex before recursing, otherwise it will deadlock
        {
            let mut crawled_instances = self.params.crawled_instances.lock().await;
            // Need this check to avoid instances being crawled multiple times. Actually the
            // crawled_instances filter below should take care of that, but its not enough).
            if crawled_instances.contains(&self.domain) {
                return Ok(());
            } else {
                crawled_instances.insert(self.domain.clone());
            }
        }

        let (site_info, federated_instances, communities) = self.fetch_instance_details().await?;

        let version = Version::parse(&site_info.version)?;
        if version < self.params.min_lemmy_version {
            return Err(anyhow!("too old lemmy version {version}"));
        }

        if self.current_distance < self.params.max_distance {
            let crawled_instances = self.params.crawled_instances.lock().await;
            federated_instances
                .clone()
                .map(|f| f.federated_instances)
                .unwrap_or_default()
                .map(|f| f.linked)
                .unwrap_or_default()
                .into_iter()
                .filter(|i| !self.params.exclude_domains.contains(&i.instance.domain))
                .filter(|i| !crawled_instances.contains(&i.instance.domain))
                .filter(|i| DOMAIN_REGEX.is_match(&i.instance.domain))
                .map(|i| {
                    CrawlJob::new(
                        i.instance.domain,
                        self.current_distance + 1,
                        self.params.clone(),
                    )
                })
                .for_each(|j| sender.send(j).unwrap());
        }

        let f = federated_instances
            .into_iter()
            .flat_map(|f| f.federated_instances);
        let crawl_result = CrawlResult {
            domain: self.domain.clone(),
            site_info,
            geo_ip: Self::geo_ip(self.domain.clone())
                .inspect_err(|e| warn!("GeoIp failed for {}: {e}", &self.domain))
                .ok()
                .flatten(),
            communities,
            linked_instances: f.clone()
                .flat_map(|f| f.linked.clone())
                .map(|l| l.instance.domain)
                .collect(),
            allowed_instances: f.clone()
                .flat_map(|f| f.allowed.clone())
                .map(|l| l.instance.domain)
                .collect(),
            blocked_instances: f.clone()
                .flat_map(|f| f.blocked.clone())
                .map(|l| l.instance.domain)
                .collect(),
        };
        self.params.result_sender.send(crawl_result).unwrap();

        Ok(())
    }

    async fn fetch_instance_details(
        &self,
    ) -> Result<
        (
            GetSiteResponse,
            Option<GetFederatedInstancesResponse>,
            Vec<CommunityView>,
        ),
        Error,
    > {
        let node_info = self
            .params
            .client
            .get(format!("https://{}/nodeinfo/2.1", &self.domain))
            .send();
        let site_info = self
            .params
            .client
            .get(format!("https://{}/api/v3/site", &self.domain))
            .send();

        let (node_info, site_info) = join!(node_info, site_info);

        let (node_info, site_info): (NodeInfo, GetSiteResponse) =
            try_join!(node_info?.json(), site_info?.json(),)?;
        if node_info.software.name != "lemmy" {
            return Err(anyhow!("wrong software {}", node_info.software.name));
        }

        let site_actor = &site_info.site_view.site.actor_id;
        if site_actor.domain() != Some(&self.domain) {
            return Err(anyhow!(
                "wrong domain {}, expected {}",
                site_actor,
                &self.domain
            ));
        }

        // Fetch communities and ignore errors
        let communities = self
            .fetch_communities()
            .await
            .inspect_err(|e| warn!("Failed to fetch communities from {}: {e}", self.domain))
            .unwrap_or_default();

        // Fetch federated instances and ignore errors
        let federated_instances = self.fetch_federated_instances().await.ok();

        Ok((site_info, federated_instances, communities))
    }

    async fn fetch_communities(&self) -> Result<Vec<CommunityView>, Error> {
        let mut communities = vec![];
        let mut page = 1;
        loop {
            const LIMIT: usize = 50;
            let url = format!(
                "https://{}/api/v3/community/list?type_=Local&sort=Hot&limit={LIMIT}&page={page}",
                &self.domain
            );
            let mut list_communities: ListCommunitiesResponse =
                self.params.client.get(url).send().await?.json().await?;
            let len = list_communities.communities.len();
            communities.append(&mut list_communities.communities);
            if len < LIMIT {
                break;
            }
            page += 1;
        }
        Ok(communities)
    }

    async fn fetch_federated_instances(&self) -> Result<GetFederatedInstancesResponse, Error> {
        Ok(self
            .params
            .client
            .get(format!(
                "https://{}/api/v3/federated_instances",
                &self.domain
            ))
            .send()
            .await?
            .json()
            .await?)
    }

    fn geo_ip(domain: String) -> Result<Option<GeoIp<'static>>, Error> {
        let mut sock_addrs = (domain, 0).to_socket_addrs()?;
        let ip = sock_addrs.next().unwrap().ip();

        // From https://github.com/wp-statistics/GeoLite2-Country
        static READER: LazyLock<Reader<Vec<u8>>> = LazyLock::new(|| {
            let input = BufReader::new(File::open("GeoLite2-City.mmdb.gz").unwrap());
            let mut buffer = vec![];
            let mut gz = GzDecoder::new(input);
            gz.read_to_end(&mut buffer).unwrap();
            Reader::from_source(buffer).unwrap()
        });

        let result = READER.lookup(ip)?.decode::<geoip2::City>()?;
        let geoip = result.map(|r| GeoIp {
            city: r.city,
            country: r.country,
            continent: r.continent,
        });
        Ok(geoip)
    }
}
