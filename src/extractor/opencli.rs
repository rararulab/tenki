//! `OpenCLI` adapter — discovers jobs via `opencli boss search` / `opencli
//! linkedin search`.

use serde::Deserialize;
use snafu::ResultExt as _;
use tokio::process::Command;

use super::{DiscoverParams, DiscoveredJob, Extractor};
use crate::error::{self, Result, TenkiError};

/// Default per-source discover limit when caller does not specify one.
const DEFAULT_DISCOVER_LIMIT: u32 = 30;

/// OpenCLI-based job extractor.
pub struct OpenCliExtractor;

impl Extractor for OpenCliExtractor {
    fn name(&self) -> &'static str { "opencli" }

    fn sources(&self) -> &[&str] { &["boss", "linkedin"] }

    async fn discover(&self, params: &DiscoverParams) -> Result<Vec<DiscoveredJob>> {
        let mut all_jobs = Vec::new();
        for source in self.sources() {
            let jobs = search_source(source, params).await?;
            all_jobs.extend(jobs);
        }
        Ok(all_jobs)
    }
}

/// Discover jobs from a single source via opencli subprocess.
pub async fn search_source(source: &str, params: &DiscoverParams) -> Result<Vec<DiscoveredJob>> {
    if !matches!(source, "boss" | "linkedin") {
        return Err(TenkiError::OpencliExecution {
            message: format!("unsupported source: {source}"),
        });
    }

    let mut cmd = Command::new("opencli");
    for arg in build_search_args(source, params) {
        cmd.arg(arg);
    }

    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TenkiError::OpencliNotFound
        } else {
            TenkiError::OpencliExecution {
                message: e.to_string(),
            }
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TenkiError::OpencliExecution {
            message: format!("exit {}: {stderr}", output.status),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw_jobs: Vec<RawOpenCliJob> = serde_json::from_str(&stdout).context(error::JsonSnafu)?;

    Ok(raw_jobs
        .into_iter()
        .map(|raw| raw.into_discovered(source))
        .collect())
}

fn build_search_args(source: &str, params: &DiscoverParams) -> Vec<String> {
    let mut args = vec![
        source.to_string(),
        "search".to_string(),
        normalize_query_for_source(source, &params.query),
        "--format".to_string(),
        "json".to_string(),
    ];

    if let Some(loc) = &params.location {
        let normalized_location = normalize_location_for_source(source, loc);
        match source {
            "linkedin" => {
                args.push("--location".to_string());
                args.push(normalized_location);
            }
            "boss" => {
                args.push("--city".to_string());
                args.push(normalized_location);
            }
            _ => unreachable!(),
        }
    }

    let limit = params.limit.unwrap_or(DEFAULT_DISCOVER_LIMIT);
    args.push("--limit".to_string());
    args.push(limit.to_string());

    args
}

fn normalize_location_for_source(source: &str, location: &str) -> String {
    let location = location.trim();
    if source != "linkedin" {
        return location.to_string();
    }

    if !location.is_ascii() {
        return location.to_string();
    }

    let key = location
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    match key.as_str() {
        // OpenCLI/LinkedIn may resolve "shanghai" to Shanghai, Virginia.
        // Use Chinese form to disambiguate to Shanghai, China.
        "shanghai" => "上海".to_string(),
        _ => location.to_string(),
    }
}

fn normalize_query_for_source(source: &str, query: &str) -> String {
    let query = query.trim();
    if source != "boss" {
        return query.to_string();
    }

    // BOSS search is less tolerant to multi-keyword free text (e.g. "python llm").
    // Use the first token for better compatibility while keeping LinkedIn
    // unchanged.
    query.split_whitespace().next().unwrap_or(query).to_string()
}

/// Raw JSON shape returned by opencli search.
#[derive(Debug, Deserialize)]
struct RawOpenCliJob {
    #[serde(alias = "jobName", alias = "job_name", alias = "title", alias = "name")]
    title:    Option<String>,
    #[serde(
        alias = "brandName",
        alias = "brand_name",
        alias = "companyName",
        alias = "company"
    )]
    company:  Option<String>,
    #[serde(alias = "url", alias = "link")]
    jd_url:   Option<String>,
    #[serde(alias = "description", alias = "jd")]
    jd_text:  Option<String>,
    #[serde(
        alias = "cityName",
        alias = "city_name",
        alias = "city",
        alias = "location",
        alias = "area"
    )]
    location: Option<String>,
    #[serde(alias = "salaryDesc", alias = "salary_desc", alias = "salary")]
    salary:   Option<String>,
}

impl RawOpenCliJob {
    fn into_discovered(self, source: &str) -> DiscoveredJob {
        DiscoveredJob::builder()
            .title(self.title.unwrap_or_default())
            .company(self.company.unwrap_or_default())
            .maybe_jd_url(self.jd_url)
            .maybe_jd_text(self.jd_text)
            .maybe_location(self.location)
            .maybe_salary(self.salary)
            .source(source.to_string())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_boss_json() {
        let json = r#"[
            {
                "jobName": "Rust Developer",
                "brandName": "ByteDance",
                "url": "https://boss.com/job/123",
                "cityName": "Shanghai",
                "salaryDesc": "30-50K"
            }
        ]"#;
        let raw: Vec<RawOpenCliJob> = serde_json::from_str(json).unwrap();
        let jobs: Vec<DiscoveredJob> = raw.into_iter().map(|r| r.into_discovered("boss")).collect();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].title, "Rust Developer");
        assert_eq!(jobs[0].company, "ByteDance");
        assert_eq!(jobs[0].source, "boss");
        assert_eq!(jobs[0].salary.as_deref(), Some("30-50K"));
    }

    #[test]
    fn parse_linkedin_json() {
        let json = r#"[
            {
                "title": "Backend Engineer",
                "company": "Stripe",
                "url": "https://linkedin.com/jobs/456",
                "location": "Remote",
                "salary": "$180-220K"
            }
        ]"#;
        let raw: Vec<RawOpenCliJob> = serde_json::from_str(json).unwrap();
        let jobs: Vec<DiscoveredJob> = raw
            .into_iter()
            .map(|r| r.into_discovered("linkedin"))
            .collect();

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].title, "Backend Engineer");
        assert_eq!(jobs[0].company, "Stripe");
        assert_eq!(jobs[0].source, "linkedin");
        assert_eq!(jobs[0].location.as_deref(), Some("Remote"));
        assert_eq!(
            jobs[0].jd_url.as_deref(),
            Some("https://linkedin.com/jobs/456")
        );
    }

    #[test]
    fn parse_empty_array() {
        let json = "[]";
        let raw: Vec<RawOpenCliJob> = serde_json::from_str(json).unwrap();
        assert!(raw.is_empty());
    }

    #[test]
    fn missing_fields_default_to_empty() {
        let json = r#"[{"jobName": "Dev"}]"#;
        let raw: Vec<RawOpenCliJob> = serde_json::from_str(json).unwrap();
        let job = raw.into_iter().next().unwrap().into_discovered("boss");
        assert_eq!(job.title, "Dev");
        assert_eq!(job.company, ""); // missing → default
        assert!(job.jd_url.is_none());
    }

    #[test]
    fn boss_query_uses_first_token() {
        assert_eq!(normalize_query_for_source("boss", "python llm"), "python");
        assert_eq!(
            normalize_query_for_source("boss", "  python   llm  "),
            "python"
        );
        assert_eq!(normalize_query_for_source("boss", "python"), "python");
    }

    #[test]
    fn linkedin_query_keeps_all_tokens() {
        assert_eq!(
            normalize_query_for_source("linkedin", "python llm"),
            "python llm"
        );
    }

    #[test]
    fn default_limit_is_applied_when_missing() {
        let params = DiscoverParams::builder().query("rust".to_string()).build();
        let args = build_search_args("linkedin", &params);

        assert!(args.windows(2).any(|w| w == ["--limit", "30"]));
    }

    #[test]
    fn explicit_limit_overrides_default() {
        let params = DiscoverParams::builder()
            .query("rust".to_string())
            .limit(55)
            .build();
        let args = build_search_args("linkedin", &params);

        assert!(args.windows(2).any(|w| w == ["--limit", "55"]));
    }

    #[test]
    fn linkedin_location_shanghai_is_disambiguated() {
        assert_eq!(normalize_location_for_source("linkedin", "shanghai"), "上海");
        assert_eq!(normalize_location_for_source("linkedin", "  shanghai "), "上海");
    }

    #[test]
    fn linkedin_location_unknown_ascii_is_preserved() {
        assert_eq!(normalize_location_for_source("linkedin", "tokyo"), "tokyo");
    }

    #[test]
    fn non_linkedin_location_is_preserved() {
        assert_eq!(normalize_location_for_source("boss", "shanghai"), "shanghai");
    }
}
