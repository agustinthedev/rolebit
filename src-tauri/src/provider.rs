use std::time::Duration;

use reqwest::{Client, StatusCode, header::CONTENT_TYPE};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::time::sleep;

use crate::models::{AppSettings, SearchResult};

#[derive(Clone)]
pub struct BrightDataProvider {
    client: Client,
    api_key: String,
    zone: String,
    gl: String,
    hl: String,
    retries: u32,
}

impl BrightDataProvider {
    pub fn new(settings: &AppSettings, api_key: String) -> Result<Self, String> {
        if api_key.trim().is_empty() || settings.serp_zone.trim().is_empty() {
            return Err("Bright Data API key and SERP zone are required".into());
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(settings.request_timeout_seconds as u64))
            .user_agent("Rolebit/0.1")
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            api_key,
            zone: settings.serp_zone.clone(),
            gl: settings.google_gl.clone(),
            hl: settings.google_hl.clone(),
            retries: settings.retry_count,
        })
    }

    pub async fn search(&self, query: &str, page: u32) -> Result<Vec<SearchResult>, String> {
        let start = page.saturating_sub(1) * 10;
        let google_url = format!(
            "https://www.google.com/search?q={}&gl={}&hl={}&start={}&brd_json=1",
            encode_component(query),
            encode_component(&self.gl),
            encode_component(&self.hl),
            start
        );
        let body = json!({ "zone": self.zone, "url": google_url, "format": "raw" });
        let mut attempt = 0;
        loop {
            let response = self
                .client
                .post("https://api.brightdata.com/request")
                .bearer_auth(&self.api_key)
                .header("x-unblock-data-format", "json")
                .json(&body)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    let status = response.status();
                    let content_type = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("unknown")
                        .to_string();
                    let text = response.text().await.map_err(|error| error.to_string())?;
                    if text.trim().is_empty() {
                        return Err(format!(
                            "Bright Data returned an empty response (HTTP {status}; content-type {content_type})"
                        ));
                    }
                    return parse_serp_response(&text).map_err(|error| {
                        let kind = if looks_like_html(&text) {
                            "Bright Data returned an HTML/interstitial response"
                        } else {
                            "Bright Data returned invalid JSON"
                        };
                        format!(
                            "{kind}: {error} (HTTP {status}; content-type {content_type}; body preview: {})",
                            response_preview(&text)
                        )
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let content_type = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("unknown")
                        .to_string();
                    let message = response.text().await.unwrap_or_default();
                    if attempt >= self.retries || !retryable(status) {
                        return Err(format!(
                            "Bright Data returned {status} (content-type {content_type}): {}",
                            response_preview(&message)
                        ));
                    }
                }
                Err(error) => {
                    if attempt >= self.retries {
                        return Err(format!("Bright Data request failed: {error}"));
                    }
                }
            }
            attempt += 1;
            sleep(Duration::from_millis(350 * 2_u64.pow(attempt.min(5)))).await;
        }
    }

    pub async fn test_connection(&self) -> Result<String, String> {
        let results = self
            .search("site:example.com \"Rolebit connection test\"", 1)
            .await?;
        Ok(format!(
            "Connection succeeded. Bright Data returned {} organic results.",
            results.len()
        ))
    }
}

pub fn mock_results(query: &str, page: u32) -> Vec<SearchResult> {
    if page > 1 {
        return Vec::new();
    }
    let digest = hex::encode(Sha256::digest(query.as_bytes()));
    let site = query
        .split_whitespace()
        .find_map(|part| part.strip_prefix("site:"))
        .unwrap_or("jobs.example.com");
    vec![SearchResult {
        url: format!("https://{site}/rolebit-demo/{}", &digest[..12]),
        title: Some(format!(
            "{} — sample opportunity",
            quoted_terms(query)
                .last()
                .cloned()
                .unwrap_or_else(|| "Software Engineer".into())
        )),
        snippet: Some(
            "A realistic local mock result used to exercise Rolebit without paid requests.".into(),
        ),
    }]
}

pub fn parse_serp_response(text: &str) -> Result<Vec<SearchResult>, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("Invalid Bright Data JSON: {error}"))?;
    let normalized = match value.get("body") {
        Some(Value::String(body)) => serde_json::from_str(body).unwrap_or_else(|_| value.clone()),
        Some(body @ Value::Object(_)) => body.clone(),
        _ => value,
    };
    let organic = normalized
        .get("organic")
        .or_else(|| normalized.pointer("/results/organic"))
        .or_else(|| normalized.pointer("/result/organic"))
        .and_then(Value::as_array);
    let Some(organic) = organic else {
        if normalized.get("error").is_some() {
            return Err(format!(
                "Bright Data response error: {}",
                concise(&normalized.to_string())
            ));
        }
        return Ok(Vec::new());
    };
    Ok(organic
        .iter()
        .filter_map(|entry| {
            let url = entry
                .get("link")
                .or_else(|| entry.get("url"))
                .and_then(Value::as_str)?;
            Some(SearchResult {
                url: url.to_string(),
                title: entry
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                snippet: entry
                    .get("description")
                    .or_else(|| entry.get("snippet"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect())
}

pub fn quoted_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in query.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            if quoted {
                terms.push(current.clone());
                current.clear();
            }
            quoted = !quoted;
        } else if quoted {
            current.push(character);
        }
    }
    terms
}

fn retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn concise(value: &str) -> String {
    value.chars().take(240).collect()
}

fn response_preview(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        "<empty>".into()
    } else {
        concise(&normalized)
    }
}

fn looks_like_html(value: &str) -> bool {
    value.trim_start().starts_with('<')
}

fn encode_component(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_organic_results() {
        let response = r#"{"organic":[{"link":"https://jobs.example.com/1","title":"QA","description":"Remote"}]}"#;
        let parsed = parse_serp_response(response).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title.as_deref(), Some("QA"));
    }

    #[test]
    fn parses_full_json_response() {
        let response = r#"{
            "general": {"query": "site:jobs.example.com QA"},
            "input": {"url": "https://www.google.com/search?q=QA"},
            "organic": [{"global_rank": 1, "link": "https://jobs.example.com/1", "title": "QA"}]
        }"#;
        let parsed = parse_serp_response(response).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].url, "https://jobs.example.com/1");
    }

    #[test]
    fn previews_empty_provider_responses() {
        assert_eq!(response_preview("  \n\t"), "<empty>");
        assert_eq!(
            response_preview("<html> upstream failure </html>"),
            "<html> upstream failure </html>"
        );
    }

    #[test]
    fn identifies_html_interstitial_responses() {
        assert!(looks_like_html("  <!doctype html><html></html>"));
        assert!(looks_like_html("<html>blocked</html>"));
        assert!(!looks_like_html(r#"{"organic": []}"#));
    }

    #[test]
    fn treats_empty_results_as_success() {
        assert!(parse_serp_response(r#"{"organic":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn builds_distinct_mock_pages_without_paid_requests() {
        let query = r#"site:jobs.example.com "Remote" "QA Engineer""#;
        assert_eq!(mock_results(query, 1).len(), 1);
        assert!(mock_results(query, 2).is_empty());
    }

    #[test]
    fn encodes_google_query_components() {
        assert_eq!(
            encode_component(r#"site:example.com "QA & SDET""#),
            "site%3Aexample.com+%22QA+%26+SDET%22"
        );
    }

    #[test]
    fn retries_rate_limits_and_server_errors_only() {
        assert!(retryable(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable(StatusCode::BAD_GATEWAY));
        assert!(!retryable(StatusCode::BAD_REQUEST));
        assert!(!retryable(StatusCode::UNAUTHORIZED));
    }
}
