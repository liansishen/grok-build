//! Trust predicates for first-party xAI HTTPS endpoints.

use ::url::{Host, Url};
use xai_grok_env::PROD_CLI_CHAT_PROXY_BASE_URL;

fn matches_trusted_base_url(candidate: &str, trusted_base: &str) -> bool {
    let Ok(candidate) = Url::parse(candidate) else {
        return false;
    };
    let Ok(trusted) = Url::parse(trusted_base) else {
        return false;
    };
    let trusted_path = trusted.path();
    let candidate_path = candidate.path();
    let path_matches = candidate_path == trusted_path
        || candidate_path
            .strip_prefix(trusted_path)
            .is_some_and(|suffix| suffix.starts_with('/'));
    candidate.scheme() == trusted.scheme()
        && candidate.host_str() == trusted.host_str()
        && candidate.port_or_known_default() == trusted.port_or_known_default()
        && path_matches
}

fn is_loopback_host(parsed: &Url) -> bool {
    match parsed.host() {
        Some(Host::Domain(host)) => host == "localhost",
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// True only for first-party xAI HTTPS routes that may receive a session bearer.
/// Invalid URLs, loopback hosts, and lookalike domains are rejected.
pub fn is_xai_api_bearer_url(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" || is_loopback_host(&parsed) {
        return false;
    }
    matches_trusted_base_url(url, PROD_CLI_CHAT_PROXY_BASE_URL)
        || parsed
            .host_str()
            .is_some_and(|host| host == "x.ai" || host.ends_with(".x.ai"))
}
