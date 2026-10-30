use std::collections::HashSet;

use crate::models::{QueryPreview, SearchConfiguration, SearchItem};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePattern {
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

impl SitePattern {
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().trim_matches(['"', '\'']);
        let value = value
            .strip_prefix("https://")
            .or_else(|| value.strip_prefix("http://"))
            .unwrap_or(value);
        let value = value.split(['?', '#']).next()?.trim_end_matches('/');
        let parsed = Url::parse(&format!("https://{value}")).ok()?;
        let host = parsed
            .host_str()?
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if host.is_empty() {
            return None;
        }
        Some(Self {
            host,
            port: explicit_port(value).or_else(|| parsed.port()),
            path: parsed.path().trim_end_matches('/').to_string(),
        })
    }

    pub fn normalized(&self) -> String {
        let authority = self
            .port
            .map(|port| format!("{}:{port}", self.host))
            .unwrap_or_else(|| self.host.clone());
        if self.path.is_empty() {
            authority
        } else {
            format!("{authority}{}", self.path)
        }
    }

    pub fn matches_url(&self, value: &str) -> bool {
        let Some(url) = Url::parse(value).ok() else {
            return false;
        };
        let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
            return false;
        };
        if host != self.host && !host.ends_with(&format!(".{}", self.host)) {
            return false;
        }
        if self.port.is_some() && url.port_or_known_default() != self.port {
            return false;
        }
        let path = url.path().trim_end_matches('/');
        if self.path.is_empty() {
            return true;
        }
        if self.path.contains('*') {
            glob_matches(&self.path, path)
        } else {
            path == self.path || path.starts_with(&format!("{}/", self.path))
        }
    }
}

fn explicit_port(value: &str) -> Option<u16> {
    let authority = value.split('/').next()?;
    let authority = authority
        .rsplit_once('@')
        .map(|(_, authority)| authority)
        .unwrap_or(authority);
    if let Some(rest) = authority.strip_prefix('[') {
        let closing_bracket = rest.find(']')?;
        return rest[closing_bracket + 1..].strip_prefix(':')?.parse().ok();
    }
    authority.rsplit_once(':')?.1.parse().ok()
}

pub fn site_pattern_from_query(query: &str) -> Option<SitePattern> {
    query
        .split_whitespace()
        .find_map(|part| SitePattern::parse(part.strip_prefix("site:")?))
}

pub fn normalize_site(value: &str) -> String {
    SitePattern::parse(value)
        .map(|pattern| pattern.normalized())
        .unwrap_or_default()
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut pattern_index = 0;
    let mut value_index = 0;
    let mut star_index = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && pattern[pattern_index] != b'*'
            && pattern[pattern_index] == value[value_index]
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn quote_term(value: &str) -> String {
    let escaped = value.trim().replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn enabled(items: &[SearchItem]) -> Vec<&SearchItem> {
    items.iter().filter(|item| item.enabled).collect()
}

pub fn generate_queries(config: &SearchConfiguration) -> Vec<String> {
    let sites = enabled(&config.sites);
    let locations = enabled(&config.locations);
    let roles: Vec<_> = if config.include_roles {
        enabled(&config.roles)
            .into_iter()
            .filter(|role| !role.value.trim().is_empty())
            .map(Some)
            .collect()
    } else {
        Vec::new()
    };
    let roles = if roles.is_empty() { vec![None] } else { roles };
    let mut seen = HashSet::new();
    let mut queries = Vec::new();

    for site in sites {
        let normalized_site = normalize_site(&site.value);
        if normalized_site.is_empty() {
            continue;
        }
        for location in &locations {
            for role in &roles {
                let mut parts = vec![format!("site:{normalized_site}")];
                if !location.value.trim().is_empty() {
                    parts.push(quote_term(&location.value));
                }
                if let Some(role) = role {
                    parts.push(quote_term(&role.value));
                }
                let query = parts.join(" ");
                if seen.insert(query.clone()) {
                    queries.push(query);
                }
            }
        }
    }
    queries
}

pub fn preview(
    config: &SearchConfiguration,
    schedule_interval_hours: u32,
    cost_per_request: Option<f64>,
) -> QueryPreview {
    let queries = generate_queries(config);
    let total_requests = queries.len() * config.pages_per_query.max(1) as usize;
    let executions_per_month = if schedule_interval_hours == 0 {
        0
    } else {
        (24 * 30) / schedule_interval_hours as usize
    };
    let estimated_monthly_requests = total_requests * executions_per_month;
    QueryPreview {
        queries,
        enabled_sites: enabled(&config.sites).len(),
        enabled_locations: enabled(&config.locations).len(),
        total_requests,
        estimated_monthly_requests,
        estimated_cost: cost_per_request.map(|cost| total_requests as f64 * cost),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: &str, enabled: bool) -> SearchItem {
        SearchItem {
            id: None,
            value: value.into(),
            enabled,
        }
    }

    #[test]
    fn generates_deterministic_cartesian_product() {
        let config = SearchConfiguration {
            sites: vec![item("https://jobs.example.com/", true)],
            locations: vec![item("Remote", true), item("Uruguay", true)],
            roles: vec![],
            include_roles: false,
            pages_per_query: 2,
        };
        assert_eq!(
            generate_queries(&config),
            vec![
                "site:jobs.example.com \"Remote\"",
                "site:jobs.example.com \"Uruguay\""
            ]
        );
    }

    #[test]
    fn preserves_empty_terms_and_escapes_quotes() {
        let config = SearchConfiguration {
            sites: vec![item("jobs.example.com", true)],
            locations: vec![item("", true)],
            roles: vec![],
            include_roles: false,
            pages_per_query: 1,
        };
        assert_eq!(generate_queries(&config), vec!["site:jobs.example.com"]);
    }

    #[test]
    fn deduplicates_normalized_queries() {
        let config = SearchConfiguration {
            sites: vec![
                item("jobs.example.com", true),
                item("https://jobs.example.com/", true),
            ],
            locations: vec![item("Remote", true)],
            roles: vec![],
            include_roles: false,
            pages_per_query: 1,
        };
        assert_eq!(generate_queries(&config).len(), 1);
    }

    #[test]
    fn preserves_site_paths_and_wildcards() {
        let config = SearchConfiguration {
            sites: vec![
                item("https://job-boards.greenhouse.io/*/jobs/*/", true),
                item("https://apply.workable.com/*/j/*/", true),
            ],
            locations: vec![item("Uruguay", true)],
            roles: vec![],
            include_roles: false,
            pages_per_query: 1,
        };
        assert_eq!(
            generate_queries(&config),
            vec![
                "site:job-boards.greenhouse.io/*/jobs/* \"Uruguay\"",
                "site:apply.workable.com/*/j/* \"Uruguay\"",
            ]
        );
    }

    #[test]
    fn matches_site_paths_and_subdomains() {
        let pattern = SitePattern::parse("https://apply.workable.com/*/j/*/").unwrap();
        assert!(pattern.matches_url("https://apply.workable.com/acme/j/123"));
        assert!(pattern.matches_url("https://sub.apply.workable.com/acme/j/123/"));
        assert!(!pattern.matches_url("https://apply.workable.com/acme/careers"));
        assert!(!pattern.matches_url("https://other.example.com/acme/j/123"));
    }

    #[test]
    fn normalizes_site_ports_and_encoded_paths() {
        let pattern = SitePattern::parse("https://JOBS.example.com:443/carrières/*").unwrap();
        assert_eq!(
            pattern.normalized(),
            "jobs.example.com:443/carri%C3%A8res/*"
        );
        assert!(pattern.matches_url("https://jobs.example.com:443/carri%C3%A8res/123"));
        assert!(!pattern.matches_url("https://jobs.example.com:8443/carri%C3%A8res/123"));
    }
}
