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

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ashby" => Some(Self::Ashby),
            "lever" => Some(Self::Lever),
            "greenhouse" => Some(Self::Greenhouse),
            "workable" => Some(Self::Workable),
            _ => None,
        }
    }

    pub fn provider_name(self) -> &'static str {
        match self {
            Self::Ashby => "ashby_api",
            Self::Lever => "lever_api",
            Self::Greenhouse => "greenhouse_api",
            Self::Workable => "workable_api",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ashby => "Ashby",
            Self::Lever => "Lever",
            Self::Greenhouse => "Greenhouse",
            Self::Workable => "Workable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AtsBoard {
    pub ats_type: AtsType,
    pub board_name: String,
}

#[derive(Debug, Clone)]
pub struct SavedAtsBoard {
    pub ats_type: AtsType,
    pub board_name: String,
    pub company_name: Option<String>,
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
        let path_board = first_segment()?;
        let board_name = if path_board == "embed" {
            url.query_pairs()
                .find(|(key, _)| key == "for")
                .map(|(_, value)| value.into_owned())?
        } else {
            path_board
        };
        (AtsType::Greenhouse, board_name)
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

#[cfg(test)]
mod tests {
    use super::{AtsBoard, AtsType, detect_ats_board};

    #[test]
    fn detects_greenhouse_embed_board_from_for_parameter() {
        assert_eq!(
            detect_ats_board(
                "https://boards.greenhouse.io/embed/job_app?for=canonical&token=5150422"
            ),
            Some(AtsBoard {
                ats_type: AtsType::Greenhouse,
                board_name: "canonical".into(),
            })
        );
    }

    #[test]
    fn leaves_greenhouse_embed_url_unclassified_without_board_parameter() {
        assert_eq!(
            detect_ats_board("https://boards.greenhouse.io/embed/job_app?token=5150422"),
            None
        );
    }

    #[test]
    fn detects_greenhouse_board_from_standard_path() {
        assert_eq!(
            detect_ats_board("https://boards.greenhouse.io/canonical/jobs/5150422"),
            Some(AtsBoard {
                ats_type: AtsType::Greenhouse,
                board_name: "canonical".into(),
            })
        );
    }
}
