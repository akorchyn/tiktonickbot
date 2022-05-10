use crate::api::Api;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub(crate) static ref TIKTOK_FULL_LINK: Regex =
        Regex::new(r#"(https://www\.tiktok\.com/(@.+)/video/([0-9]+))"#).unwrap();
    pub(crate) static ref TIKTOK_SHORT_LINK: Regex =
        Regex::new(r#"(https://vm\.tiktok\.com/[^[:punct:]\s]+/)"#).unwrap();
    pub(crate) static ref TWITTER_LINK: Regex =
        Regex::new(r#"(https://twitter\.com/(.+)/status/([0-9]+))"#).unwrap();
    pub(crate) static ref INSTAGRAM_LINK: Regex =
        Regex::new(r#"((?:https://)?www\.instagram\.com/(?:tv|reel|p|stories/[^/]+)/([^/]+))"#)
            .unwrap();
}

pub(crate) fn match_api(url: &str) -> Option<Api> {
    match () {
        #[cfg(feature = "tiktok")]
        _ if TIKTOK_FULL_LINK.is_match(url) => Some(Api::Tiktok),
        #[cfg(feature = "twitter")]
        _ if TWITTER_LINK.is_match(url) => Some(Api::Twitter),
        #[cfg(feature = "instagram")]
        _ if INSTAGRAM_LINK.is_match(url) => Some(Api::Instagram),
        _ => None,
    }
}
