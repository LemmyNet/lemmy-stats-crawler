use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::crawl::CrawlResult;

// TODO: lemmy stores these numbers in SiteAggregates, would be good to simply use that as a member
//       (to avoid many members). but SiteAggregates also has id, site_id fields
#[derive(Serialize, Clone)]
pub struct TotalStats<T: Clone + Serialize> {
    pub crawled_instances: i32,
    pub total_users: i64,
    pub users_active_day: i64,
    pub users_active_week: i64,
    pub users_active_month: i64,
    pub users_active_halfyear: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub instance_details: Vec<T>,
}

pub fn full_instance_data(
    instance_details: Vec<CrawlResult>,
    start_time: DateTime<Utc>,
) -> TotalStats<CrawlResult> {
    let mut total_users = 0;
    let mut users_active_day = 0;
    let mut users_active_week = 0;
    let mut users_active_month = 0;
    let mut users_active_halfyear = 0;
    let mut crawled_instances = 0;
    for i in &instance_details {
        crawled_instances += 1;
        total_users += i.site_info.site_view.counts.users;
        users_active_day += i.site_info.site_view.counts.users_active_day;
        users_active_week += i.site_info.site_view.counts.users_active_week;
        users_active_month += i.site_info.site_view.counts.users_active_month;
        users_active_halfyear += i.site_info.site_view.counts.users_active_half_year;
    }
    TotalStats {
        crawled_instances,
        total_users,
        users_active_day,
        users_active_week,
        users_active_halfyear,
        users_active_month,
        start_time,
        end_time: Utc::now(),
        instance_details,
    }
}

pub fn joinlemmy_instance_data(total_stats: &TotalStats<CrawlResult>) -> TotalStats<CrawlResult> {
    let mut joinlemmy_stats = total_stats.clone();
    joinlemmy_stats.instance_details = joinlemmy_stats
        .instance_details
        .into_iter()
        // Filter out instances with other registration modes (closed dont allow signups and
        // open are often abused by bots)
        .filter(|i| {
            &i.site_info
                .site_view
                .local_site
                .registration_mode
                .to_string()
                == "RequireApplication"
        })
        // Require at least 5 monthly users
        .filter(|i| i.site_info.site_view.counts.users_active_month > 5)
        // Exclude nsfw instances
        .filter(|i| i.site_info.site_view.site.content_warning.is_none())
        // Exclude some unnecessary data to reduce output size
        .map(|mut i| {
            i.site_info.admins = vec![];
            i.site_info.all_languages = vec![];
            i.site_info.discussion_languages = vec![];
            i.site_info.custom_emojis = vec![];
            i.site_info.taglines = vec![];
            i.site_info.site_view.local_site.application_question = None;
            i.site_info.site_view.local_site.legal_information = None;
            i.site_info.site_view.local_site.slur_filter_regex = None;
            i.site_info.site_view.site.public_key = String::new();
            i.site_info.blocked_urls = vec![];
            i.allowed_instances = vec![];
            i.blocked_instances = vec![];
            i.linked_instances = vec![];
            i
        })
        .collect();
    joinlemmy_stats
}

#[derive(Serialize, Clone)]
pub struct MinimalInstanceData {
    domain: String,
    users: i64,
    posts: i64,
    comments: i64,
    communities: i64,
    users_active_day: i64,
    users_active_week: i64,
    users_active_month: i64,
    users_active_half_year: i64,
}

pub fn minimal_instance_data(
    total_stats: &TotalStats<CrawlResult>,
) -> TotalStats<MinimalInstanceData> {
    let instance_details = total_stats
        .instance_details
        .iter()
        .map(|i| MinimalInstanceData {
            domain: i.domain.clone(),
            users: i.site_info.site_view.counts.users,
            posts: i.site_info.site_view.counts.posts,
            comments: i.site_info.site_view.counts.comments,
            communities: i.site_info.site_view.counts.communities,
            users_active_day: i.site_info.site_view.counts.users_active_day,
            users_active_week: i.site_info.site_view.counts.users_active_week,
            users_active_month: i.site_info.site_view.counts.users_active_month,
            users_active_half_year: i.site_info.site_view.counts.users_active_half_year,
        })
        .collect();
    TotalStats {
        instance_details,
        crawled_instances: total_stats.crawled_instances,
        total_users: total_stats.total_users,
        users_active_day: total_stats.users_active_day,
        users_active_week: total_stats.users_active_week,
        users_active_month: total_stats.users_active_month,
        users_active_halfyear: total_stats.users_active_halfyear,
        start_time: total_stats.start_time,
        end_time: total_stats.end_time,
    }
}
