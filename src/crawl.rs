use crate::structs::NodeInfo;
use anyhow::{anyhow, Error};
use lemmy_api_common_v019::site::{GetFederatedInstancesResponse, GetSiteResponse};
use log::warn;
use maxminddb::geoip2;
use maxminddb::Reader;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest_middleware::ClientWithMiddleware;
use semver::Version;
use serde::Serialize;
use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::join;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

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

#[derive(Debug, Serialize)]
pub struct GeoIp {
    pub iso_code: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CrawlResult {
    pub domain: String,
    pub node_info: NodeInfo,
    pub site_info: GetSiteResponse,
    pub federated_instances: GetFederatedInstancesResponse,
    pub geo_ip: Option<GeoIp>,
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

        let (node_info, site_info, federated_instances) = self.fetch_instance_details().await?;

        let version = Version::parse(&site_info.version)?;
        if version < self.params.min_lemmy_version {
            return Err(anyhow!("too old lemmy version {version}"));
        }

        if self.current_distance < self.params.max_distance {
            let crawled_instances = self.params.crawled_instances.lock().await;
            federated_instances
                .federated_instances
                .clone()
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

        let crawl_result = CrawlResult {
            domain: self.domain.clone(),
            node_info,
            site_info,
            federated_instances,
            geo_ip: Self::geo_ip(&self.domain)
                .inspect_err(|e| warn!("GeoIp failed for {}: {e}", &self.domain))
                .ok()
                .flatten(),
        };
        self.params.result_sender.send(crawl_result).unwrap();

        Ok(())
    }

    async fn fetch_instance_details(
        &self,
    ) -> Result<(NodeInfo, GetSiteResponse, GetFederatedInstancesResponse), Error> {
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
        let federated_instances = self
            .params
            .client
            .get(format!(
                "https://{}/api/v3/federated_instances",
                &self.domain
            ))
            .send();

        let (node_info, site_info, federated_instances) =
            join!(node_info, site_info, federated_instances);

        let node_info = node_info?.json::<NodeInfo>().await?;
        if node_info.software.name != "lemmy" && node_info.software.name != "lemmybb" {
            return Err(anyhow!("wrong software {}", node_info.software.name));
        }

        let site_info = site_info?.json::<GetSiteResponse>().await?;
        let site_actor = &site_info.site_view.site.actor_id;
        if site_actor.domain() != Some(&self.domain) {
            return Err(anyhow!(
                "wrong domain {}, expected {}",
                site_actor,
                &self.domain
            ));
        }

        let federated_instances = federated_instances?
            .json::<GetFederatedInstancesResponse>()
            .await?;

        Ok((node_info, site_info, federated_instances))
    }

    fn geo_ip(domain: &str) -> Result<Option<GeoIp>, Error> {
        let mut sock_addrs = (domain, 0).to_socket_addrs()?;
        let ip = sock_addrs.next().unwrap().ip();

        // From https://github.com/wp-statistics/GeoLite2-Country
        static READER: LazyLock<Reader<Vec<u8>>> = LazyLock::new(|| {
            Reader::open_readfile("GeoLite2-Country.mmdb").expect("parse geolite db")
        });

        let result = READER.lookup(ip)?.decode::<geoip2::Country>()?;
        let geoip = result
            .and_then(|r| {
                if let (Some(c), Some(n)) = (r.country.iso_code, r.country.names.english) {
                    Some((c, n))
                } else {
                    None
                }
            })
            .map(|(c, n)| GeoIp {
                iso_code: c.to_string(),
                name: n.to_string(),
            });
        Ok(geoip)
    }
}
