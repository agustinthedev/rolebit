use serde::Serialize;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AtsType {
    Ashby,
    Lever,
    Greenhouse,
    Workable,
}

impl AtsType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ashby => "ashby",
            Self::Lever => "lever",
            Self::Greenhouse => "greenhouse",
            Self::Workable => "workable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AtsBoard {
    pub ats_type: AtsType,
    pub board_name: String,
}

pub fn detect_ats_board(value: &str) -> Option<AtsBoard> {
    let url = Url::parse(value).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let first_segment = || {
        url.path_segments()?
            .find(|segment| !segment.is_empty())
            .map(str::to_string)
    };

    let (ats_type, board_name) = if host == "jobs.ashbyhq.com" {
        (AtsType::Ashby, first_segment()?)
    } else if host == "jobs.lever.co" || host == "jobs.eu.lever.co" {
        (AtsType::Lever, first_segment()?)
    } else if host == "boards.greenhouse.io"
        || host == "job-boards.greenhouse.io"
        || host.starts_with("boards.") && host.ends_with(".greenhouse.io")
        || host.starts_with("job-boards.") && host.ends_with(".greenhouse.io")
    {
        (AtsType::Greenhouse, first_segment()?)
    } else if host == "apply.workable.com" {
        let board_name = first_segment()?;
        if board_name == "j" {
            return None;
        }
        (AtsType::Workable, board_name)
    } else if let Some(subdomain) = host.strip_suffix(".workable.com") {
        if subdomain.is_empty() || matches!(subdomain, "www" | "api" | "apply") {
            return None;
        }
        (AtsType::Workable, subdomain.to_string())
    } else {
        return None;
    };

    (!board_name.trim().is_empty()).then_some(AtsBoard {
        ats_type,
        board_name,
    })
}
