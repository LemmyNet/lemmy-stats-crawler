use chrono::{DateTime, Utc};
use lemmy_api_common_v019::{
    lemmy_db_schema::RegistrationMode, lemmy_db_views_actor::structs::CommunityView,
};
use serde::Serialize;

use crate::crawl::CrawlResult;

// TODO: lemmy stores these numbers in SiteAggregates, would be good to simply use that as a member
//       (to avoid many members). but SiteAggregates also has id, site_id fields
#[derive(Serialize, Clone)]
pub struct TotalInstanceStats<T: Clone + Serialize> {
    pub crawled_instances: usize,
    pub total_users: i64,
    pub users_active_day: i64,
    pub users_active_week: i64,
    pub users_active_month: i64,
    pub users_active_halfyear: i64,
    pub posts: i64,
    pub comments: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub instance_details: Vec<T>,
}

#[derive(Serialize, Clone)]
pub struct TotalCommunityStats<T: Clone + Serialize> {
    pub crawled_communities: usize,
    pub subscribers: i64,
    pub posts: i64,
    pub comments: i64,
    pub subscribers_local: i64,
    pub users_active_day: i64,
    pub users_active_week: i64,
    pub users_active_month: i64,
    pub users_active_halfyear: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub community_details: Vec<T>,
}

pub fn full_instance_data(
    mut instance_details: Vec<CrawlResult>,
    start_time: DateTime<Utc>,
) -> (
    TotalInstanceStats<CrawlResult>,
    TotalCommunityStats<CommunityView>,
) {
    let mut communities = vec![];
    let mut total_users = 0;
    let mut posts = 0;
    let mut comments = 0;
    let mut users_active_day = 0;
    let mut users_active_week = 0;
    let mut users_active_month = 0;
    let mut users_active_halfyear = 0;
    for i in &mut instance_details {
        communities.append(&mut i.communities);
        total_users += i.site_info.site_view.counts.users;
        posts += i.site_info.site_view.counts.posts;
        comments += i.site_info.site_view.counts.comments;
        users_active_day += i.site_info.site_view.counts.users_active_day;
        users_active_week += i.site_info.site_view.counts.users_active_week;
        users_active_month += i.site_info.site_view.counts.users_active_month;
        users_active_halfyear += i.site_info.site_view.counts.users_active_half_year;
    }
    let total_instances = TotalInstanceStats {
        crawled_instances: instance_details.len(),
        total_users,
        posts,
        comments,
        users_active_day,
        users_active_week,
        users_active_halfyear,
        users_active_month,
        start_time,
        end_time: Utc::now(),
        instance_details,
    };

    let mut subscribers = 0;
    let mut subscribers_local = 0;
    let mut posts = 0;
    let mut comments = 0;
    let mut users_active_day = 0;
    let mut users_active_week = 0;
    let mut users_active_month = 0;
    let mut users_active_halfyear = 0;
    for c in &communities {
        subscribers += c.counts.subscribers;
        subscribers_local += c.counts.subscribers_local;
        posts += c.counts.posts;
        comments += c.counts.comments;
        users_active_day += c.counts.users_active_day;
        users_active_week += c.counts.users_active_week;
        users_active_month += c.counts.users_active_month;
        users_active_halfyear += c.counts.users_active_half_year;
    }
    let total_communities = TotalCommunityStats {
        crawled_communities: communities.len(),
        subscribers,
        posts,
        comments,
        subscribers_local,
        users_active_day,
        users_active_week,
        users_active_month,
        users_active_halfyear,
        start_time,
        end_time: Utc::now(),
        community_details: communities,
    };
    (total_instances, total_communities)
}

pub fn joinlemmy_instance_data(
    total_stats: &TotalInstanceStats<CrawlResult>,
) -> TotalInstanceStats<CrawlResult> {
    let mut joinlemmy_stats = total_stats.clone();
    joinlemmy_stats.instance_details = joinlemmy_stats
        .instance_details
        .into_iter()
        // Filter out instances with other registration modes (closed dont allow signups and
        // open are often abused by bots)
        .filter(|i| {
            let local_site = &i.site_info.site_view.local_site;
            local_site.registration_mode == RegistrationMode::RequireApplication
                || local_site.captcha_enabled
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
    total_stats: &TotalInstanceStats<CrawlResult>,
) -> TotalInstanceStats<MinimalInstanceData> {
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
    TotalInstanceStats {
        instance_details,
        crawled_instances: total_stats.crawled_instances,
        total_users: total_stats.total_users,
        posts: total_stats.posts,
        comments: total_stats.comments,
        users_active_day: total_stats.users_active_day,
        users_active_week: total_stats.users_active_week,
        users_active_month: total_stats.users_active_month,
        users_active_halfyear: total_stats.users_active_halfyear,
        start_time: total_stats.start_time,
        end_time: total_stats.end_time,
    }
}

#[derive(Serialize, Clone)]
pub struct MinimalCommunityData {
    ap_id: String,
    subscribers: i64,
    subscribers_local: i64,
    posts: i64,
    comments: i64,
    users_active_day: i64,
    users_active_week: i64,
    users_active_month: i64,
    users_active_half_year: i64,
}

pub fn minimal_community_data(
    total_stats: &TotalCommunityStats<CommunityView>,
) -> TotalCommunityStats<MinimalCommunityData> {
    let community_details: Vec<MinimalCommunityData> = total_stats
        .community_details
        .iter()
        .map(|c| MinimalCommunityData {
            ap_id: c.community.actor_id.to_string(),
            subscribers: c.counts.subscribers,
            subscribers_local: c.counts.subscribers_local,
            posts: c.counts.posts,
            comments: c.counts.comments,
            users_active_day: c.counts.users_active_day,
            users_active_week: c.counts.users_active_week,
            users_active_month: c.counts.users_active_month,
            users_active_half_year: c.counts.users_active_half_year,
        })
        .collect();
    TotalCommunityStats {
        community_details,
        crawled_communities: total_stats.crawled_communities,
        subscribers: total_stats.subscribers,
        subscribers_local: total_stats.subscribers_local,
        posts: total_stats.posts,
        comments: total_stats.comments,
        users_active_day: total_stats.users_active_day,
        users_active_week: total_stats.users_active_week,
        users_active_month: total_stats.users_active_month,
        users_active_halfyear: total_stats.users_active_halfyear,
        start_time: total_stats.start_time,
        end_time: total_stats.end_time,
    }
}
