use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub(crate) static ref TIKTOK_FULL_LINK: Regex =
        Regex::new(r#"(https://www\.tiktok\.com/(@.+)/video/([0-9]+))"#).unwrap();
    pub(crate) static ref TIKTOK_SHORT_LINK: Regex =
        Regex::new(r#"(https://vm\.tiktok\.com/[^[:punct:]\s]+/)"#).unwrap();
    pub(crate) static ref TWITTER_LINK: Regex =
        Regex::new(r#"(https://twitter\.com/(.+)/status/([0-9]+))"#).unwrap();
}
