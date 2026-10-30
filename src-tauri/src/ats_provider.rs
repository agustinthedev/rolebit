use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::{
    ats::{AtsType, SavedAtsBoard},
    extractor::{infer_remote, normalize_url, strip_html},
    models::CandidateJob,
};

const LEVER_PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct AtsClient {
    client: Client,
    retries: u32,
}

pub struct AtsFetchResult {
    pub jobs: Vec<CandidateJob>,
    pub result_count: usize,
    pub filtered_count: usize,
    pub requests: usize,
}

pub struct AtsFetchError {
    pub message: String,
    pub requests: usize,
}

impl AtsClient {
    pub fn new(timeout_seconds: u32, retries: u32) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds.max(1) as u64))
            .user_agent("Rolebit/0.1")
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { client, retries })
    }

    pub async fn fetch_board(
        &self,
        board: &SavedAtsBoard,
    ) -> Result<AtsFetchResult, AtsFetchError> {
        match board.ats_type {
            AtsType::Ashby => self.fetch_ashby(board).await,
            AtsType::Greenhouse => self.fetch_greenhouse(board).await,
            AtsType::Lever => self.fetch_lever(board).await,
            AtsType::Workable => self.fetch_workable(board).await,
        }
    }

    async fn fetch_ashby(&self, board: &SavedAtsBoard) -> Result<AtsFetchResult, AtsFetchError> {
        let mut url = api_url("https://api.ashbyhq.com")?;
        push_path(&mut url, &["posting-api", "job-board", &board.board_name])?;
        url.query_pairs_mut()
            .append_pair("includeCompensation", "true");
        let (body, requests) = self.get_json(url).await?;
        let raw_jobs = body.get("jobs").and_then(Value::as_array).ok_or_else(|| {
            AtsFetchError::invalid_response("Ashby returned no jobs list", requests)
        })?;
        let mut filtered_count = 0;
        let jobs = raw_jobs
            .iter()
            .filter_map(|job| {
                if job.get("isListed").and_then(Value::as_bool) == Some(false) {
                    filtered_count += 1;
                    return None;
                }
                match candidate_from_fields(
                    board,
                    job,
                    &["jobUrl", "applyUrl"],
                    &["id"],
                    &["title"],
                    &["descriptionPlain", "descriptionHtml"],
                    &["location"],
                    &["workplaceType", "location"],
                    &["employmentType"],
                    &["publishedAt"],
                ) {
                    Some(candidate) => Some(candidate),
                    None => {
                        filtered_count += 1;
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        Ok(AtsFetchResult {
            jobs,
            result_count: raw_jobs.len(),
            filtered_count,
            requests,
        })
    }

    async fn fetch_greenhouse(
        &self,
        board: &SavedAtsBoard,
    ) -> Result<AtsFetchResult, AtsFetchError> {
        let mut url = api_url("https://boards-api.greenhouse.io")?;
        push_path(&mut url, &["v1", "boards", &board.board_name, "jobs"])?;
        url.query_pairs_mut().append_pair("content", "true");
        let (body, requests) = self.get_json(url).await?;
        let raw_jobs = body.get("jobs").and_then(Value::as_array).ok_or_else(|| {
            AtsFetchError::invalid_response("Greenhouse returned no jobs list", requests)
        })?;
        let mut filtered_count = 0;
        let jobs = raw_jobs
            .iter()
            .filter_map(|job| {
                match candidate_from_fields(
                    board,
                    job,
                    &["absolute_url"],
                    &["id"],
                    &["title"],
                    &["content"],
                    &["location.name"],
                    &["location.name"],
                    &["employment_type"],
                    &["updated_at"],
                ) {
                    Some(candidate) => Some(candidate),
                    None => {
                        filtered_count += 1;
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        Ok(AtsFetchResult {
            jobs,
            result_count: raw_jobs.len(),
            filtered_count,
            requests,
        })
    }

    async fn fetch_lever(&self, board: &SavedAtsBoard) -> Result<AtsFetchResult, AtsFetchError> {
        let global = self
            .fetch_lever_from_host(board, "https://api.lever.co")
            .await;
        if matches!(&global, Ok(result) if !result.jobs.is_empty()) {
            return global;
        }

        match self
            .fetch_lever_from_host(board, "https://api.eu.lever.co")
            .await
        {
            Ok(eu) if !eu.jobs.is_empty() => Ok(eu),
            Ok(mut eu) => match global {
                Ok(mut global) => {
                    global.requests += eu.requests;
                    global.result_count += eu.result_count;
                    global.filtered_count += eu.filtered_count;
                    Ok(global)
                }
                Err(error) => {
                    eu.requests += error.requests;
                    Ok(eu)
                }
            },
            Err(mut eu_error) => match global {
                Ok(mut global) => {
                    global.requests += eu_error.requests;
                    Ok(global)
                }
                Err(global_error) => {
                    eu_error.requests += global_error.requests;
                    Err(AtsFetchError {
                        message: format!(
                            "Lever global and EU feeds failed: {}; {}",
                            global_error.message, eu_error.message
                        ),
                        requests: eu_error.requests,
                    })
                }
            },
        }
    }

    async fn fetch_lever_from_host(
        &self,
        board: &SavedAtsBoard,
        base: &str,
    ) -> Result<AtsFetchResult, AtsFetchError> {
        let mut jobs = Vec::new();
        let mut requests = 0;
        let mut result_count = 0;
        let mut filtered_count = 0;
        let mut skip = 0;
        loop {
            let mut url = api_url(base).map_err(|mut error| {
                error.requests += requests;
                error
            })?;
            push_path(&mut url, &["v0", "postings", &board.board_name]).map_err(|mut error| {
                error.requests += requests;
                error
            })?;
            url.query_pairs_mut()
                .append_pair("mode", "json")
                .append_pair("skip", &skip.to_string())
                .append_pair("limit", &LEVER_PAGE_SIZE.to_string());
            let (body, attempts) = self.get_json(url).await.map_err(|mut error| {
                error.requests += requests;
                error
            })?;
            requests += attempts;
            let raw_jobs = body
                .as_array()
                .or_else(|| body.get("data").and_then(Value::as_array))
                .ok_or_else(|| {
                    AtsFetchError::invalid_response("Lever returned no postings list", requests)
                })?;
            let page_count = raw_jobs.len();
            result_count += page_count;
            for raw_job in raw_jobs {
                if let Some(candidate) = candidate_from_fields(
                    board,
                    raw_job,
                    &["hostedUrl", "urls.show", "applyUrl"],
                    &["id"],
                    &["text"],
                    &["descriptionPlain", "description", "content.description"],
                    &["categories.location"],
                    &["workplaceType", "categories.location"],
                    &["categories.commitment"],
                    &["createdAt", "publishedAt"],
                ) {
                    jobs.push(candidate);
                } else {
                    filtered_count += 1;
                }
            }
            if page_count < LEVER_PAGE_SIZE {
                break;
            }
            skip += page_count;
        }
        Ok(AtsFetchResult {
            jobs,
            result_count,
            filtered_count,
            requests,
        })
    }

    async fn fetch_workable(&self, board: &SavedAtsBoard) -> Result<AtsFetchResult, AtsFetchError> {
        let mut url = api_url("https://www.workable.com")?;
        push_path(&mut url, &["api", "accounts", &board.board_name])?;
        url.query_pairs_mut().append_pair("details", "true");
        let (body, requests) = self.get_json(url).await?;
        let raw_jobs = body
            .get("jobs")
            .or_else(|| body.get("data"))
            .and_then(Value::as_array)
            .or_else(|| body.as_array())
            .ok_or_else(|| {
                AtsFetchError::invalid_response("Workable returned no jobs list", requests)
            })?;
        let mut filtered_count = 0;
        let jobs = raw_jobs
            .iter()
            .filter_map(|job| {
                match candidate_from_fields(
                    board,
                    job,
                    &["url", "application_url", "shortlink"],
                    &["shortcode", "id", "code"],
                    &["title"],
                    &["description", "description_plain"],
                    &["location.name", "location", "location.city"],
                    &["location.telecommuting", "remote", "location.name"],
                    &["employment_type"],
                    &["created_at", "published_at"],
                ) {
                    Some(candidate) => Some(candidate),
                    None => {
                        filtered_count += 1;
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        Ok(AtsFetchResult {
            jobs,
            result_count: raw_jobs.len(),
            filtered_count,
            requests,
        })
    }

    async fn get_json(&self, url: Url) -> Result<(Value, usize), AtsFetchError> {
        let mut requests = 0;
        for attempt in 0..=self.retries {
            requests += 1;
            let response = match self.client.get(url.clone()).send().await {
                Ok(response) => response,
                Err(_error) if attempt < self.retries => {
                    tokio::time::sleep(retry_delay(attempt)).await;
                    continue;
                }
                Err(error) => {
                    return Err(AtsFetchError {
                        message: format!("ATS request failed: {error}"),
                        requests,
                    });
                }
            };
            let status = response.status();
            let body = response.text().await.map_err(|error| AtsFetchError {
                message: format!("Could not read ATS response: {error}"),
                requests,
            })?;
            if status.is_success() {
                return serde_json::from_str(&body)
                    .map(|value| (value, requests))
                    .map_err(|error| AtsFetchError {
                        message: format!("ATS returned invalid JSON: {error}"),
                        requests,
                    });
            }
            if should_retry(status) && attempt < self.retries {
                tokio::time::sleep(retry_delay(attempt)).await;
                continue;
            }
            return Err(AtsFetchError {
                message: format!("ATS returned HTTP {status}: {}", response_preview(&body)),
                requests,
            });
        }
        Err(AtsFetchError {
            message: "ATS request failed after retries".into(),
            requests,
        })
    }
}

impl AtsFetchError {
    fn invalid_response(message: &str, requests: usize) -> Self {
        Self {
            message: message.into(),
            requests,
        }
    }
}

fn candidate_from_fields(
    board: &SavedAtsBoard,
    item: &Value,
    url_paths: &[&str],
    id_paths: &[&str],
    title_paths: &[&str],
    description_paths: &[&str],
    location_paths: &[&str],
    remote_paths: &[&str],
    employment_paths: &[&str],
    published_paths: &[&str],
) -> Option<CandidateJob> {
    let title = text_at(item, title_paths)?;
    let source_url = text_at(item, url_paths)?;
    let source_url = normalize_url(&source_url, &source_url)?;
    let parsed_url = Url::parse(&source_url).ok()?;
    if parsed_url.scheme() != "https" || !is_expected_host(board.ats_type, parsed_url.host_str()?) {
        return None;
    }
    let canonical_url = normalize_url(&source_url, &source_url)?;
    let source_domain = parsed_url.host_str()?.to_string();
    let external_id = text_at(item, id_paths).or_else(|| {
        parsed_url
            .path_segments()?
            .filter(|segment| !segment.is_empty())
            .next_back()
            .map(str::to_string)
    });
    let description = text_at(item, description_paths).map(|value| strip_html(&value));
    let location = location_at(item, location_paths);
    let remote_text = text_at(item, remote_paths).unwrap_or_default();
    let is_remote = item
        .pointer("/location/telecommuting")
        .or_else(|| item.get("isRemote"))
        .and_then(Value::as_bool)
        == Some(true);
    let remote_status = if is_remote && !remote_text.to_ascii_lowercase().contains("hybrid") {
        "remote".into()
    } else {
        infer_remote(&format!(
            "{remote_text} {}",
            location.as_deref().unwrap_or_default()
        ))
    };
    Some(CandidateJob {
        source_url,
        canonical_url,
        external_id,
        source_domain,
        ats_board: Some(crate::ats::AtsBoard {
            ats_type: board.ats_type,
            board_name: board.board_name.clone(),
        }),
        title: strip_html(&title),
        company: board.company_name.clone(),
        description,
        location,
        remote_status,
        employment_type: text_at(item, employment_paths),
        published_at: text_at(item, published_paths).and_then(normalize_date),
        expires_at: None,
        extraction_status: "ats_api".into(),
    })
}

fn location_at(item: &Value, paths: &[&str]) -> Option<String> {
    text_at(item, paths).or_else(|| {
        paths.iter().find_map(|path| {
            let value = path_value(item, path)?;
            let parts = ["city", "region", "country", "name"]
                .iter()
                .filter_map(|key| value.get(key).and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join(", "))
        })
    })
}

fn text_at(item: &Value, paths: &[&str]) -> Option<String> {
    paths.iter().find_map(|path| {
        let value = path_value(item, path)?;
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                value
                    .as_bool()
                    .map(|value| value.to_string())
                    .filter(|value| value != "false")
            })
            .or_else(|| value.as_i64().map(|value| value.to_string()))
    })
}

fn path_value<'a>(item: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(item, |value, segment| value.get(segment))
}

fn normalize_date(value: String) -> Option<String> {
    if let Ok(date) = DateTime::parse_from_rfc3339(&value) {
        return Some(date.with_timezone(&Utc).to_rfc3339());
    }
    value.parse::<i64>().ok().and_then(|timestamp| {
        let date = if timestamp > 10_000_000_000 {
            DateTime::<Utc>::from_timestamp_millis(timestamp)
        } else {
            DateTime::<Utc>::from_timestamp(timestamp, 0)
        }?;
        Some(date.to_rfc3339())
    })
}

fn is_expected_host(ats_type: AtsType, host: &str) -> bool {
    match ats_type {
        AtsType::Ashby => host == "jobs.ashbyhq.com",
        AtsType::Lever => matches!(host, "jobs.lever.co" | "jobs.eu.lever.co"),
        AtsType::Greenhouse => {
            host == "boards.greenhouse.io"
                || host == "job-boards.greenhouse.io"
                || host.starts_with("boards.") && host.ends_with(".greenhouse.io")
                || host.starts_with("job-boards.") && host.ends_with(".greenhouse.io")
        }
        AtsType::Workable => matches!(host, "apply.workable.com" | "www.workable.com"),
    }
}

fn api_url(base: &str) -> Result<Url, AtsFetchError> {
    Url::parse(base).map_err(|error| AtsFetchError {
        message: format!("Invalid ATS endpoint: {error}"),
        requests: 0,
    })
}

fn push_path(url: &mut Url, segments: &[&str]) -> Result<(), AtsFetchError> {
    let mut path = url.path_segments_mut().map_err(|_| AtsFetchError {
        message: "ATS endpoint cannot accept a path".into(),
        requests: 0,
    })?;
    path.pop_if_empty();
    for segment in segments {
        path.push(segment);
    }
    Ok(())
}

fn should_retry(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(attempt: u32) -> Duration {
    Duration::from_millis(250 * (1_u64 << attempt.min(6)))
}

fn response_preview(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > 180 {
        format!("{}…", compact.chars().take(180).collect::<String>())
    } else {
        compact
    }
}
