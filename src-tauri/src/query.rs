use std::collections::HashSet;

use crate::models::{QueryPreview, SearchConfiguration, SearchItem};

pub fn normalize_site(value: &str) -> String {
    let trimmed = value.trim().trim_matches('/');
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    without_scheme
        .split('/')
        .next()
        .unwrap_or_default()
        .to_lowercase()
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
    let mut seen = HashSet::new();
    let mut queries = Vec::new();

    for site in sites {
        let normalized_site = normalize_site(&site.value);
        if normalized_site.is_empty() {
            continue;
        }
        for location in &locations {
            let mut parts = vec![format!("site:{normalized_site}")];
            if !location.value.trim().is_empty() {
                parts.push(quote_term(&location.value));
            }
            let query = parts.join(" ");
            if seen.insert(query.clone()) {
                queries.push(query);
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
            pages_per_query: 1,
        };
        assert_eq!(
            generate_queries(&config),
            vec!["site:jobs.example.com"]
        );
    }

    #[test]
    fn deduplicates_normalized_queries() {
        let config = SearchConfiguration {
            sites: vec![
                item("jobs.example.com", true),
                item("https://jobs.example.com/", true),
            ],
            locations: vec![item("Remote", true)],
            pages_per_query: 1,
        };
        assert_eq!(generate_queries(&config).len(), 1);
    }
}
