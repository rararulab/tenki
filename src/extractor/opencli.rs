//! `OpenCLI` adapter — discovers jobs via `opencli boss search` / `opencli
//! linkedin search`.

use serde::Deserialize;
use snafu::ResultExt as _;
use tokio::{
    process::Command,
    time::{Duration, sleep},
};

use super::{DiscoverParams, DiscoveredJob, Extractor};
use crate::error::{self, Result, TenkiError};

/// Default per-source discover limit when caller does not specify one.
const DEFAULT_DISCOVER_LIMIT: u32 = 30;
const BOSS_RETRY_ATTEMPTS: usize = 3;

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

    let args = build_search_args(source, params);
    let max_attempts = if source == "boss" {
        BOSS_RETRY_ATTEMPTS
    } else {
        1
    };
    let mut last_error: Option<TenkiError> = None;

    for attempt in 1..=max_attempts {
        match run_opencli_once(&args).await {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let raw_jobs: Vec<RawOpenCliJob> =
                        serde_json::from_str(&stdout).context(error::JsonSnafu)?;

                    return Ok(raw_jobs
                        .into_iter()
                        .map(|raw| raw.into_discovered(source))
                        .collect());
                }

                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let err = TenkiError::OpencliExecution {
                    message: format!("exit {}: {stderr}", output.status),
                };

                if source == "boss"
                    && attempt < max_attempts
                    && is_transient_opencli_failure(&stderr)
                {
                    eprintln!(
                        "[discover] boss transient error on attempt {attempt}/{max_attempts}, \
                         retrying..."
                    );
                    sleep(backoff_delay(attempt)).await;
                    continue;
                }

                return Err(err);
            }
            Err(err) => {
                if source == "boss"
                    && attempt < max_attempts
                    && is_transient_opencli_failure(&err.to_string())
                {
                    eprintln!(
                        "[discover] boss transient failure on attempt {attempt}/{max_attempts}, \
                         retrying..."
                    );
                    last_error = Some(err);
                    sleep(backoff_delay(attempt)).await;
                    continue;
                }
                return Err(err);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| TenkiError::OpencliExecution {
        message: "boss search failed without detailed error".to_string(),
    }))
}

async fn run_opencli_once(args: &[String]) -> Result<std::process::Output> {
    let mut cmd = Command::new("opencli");
    for arg in args {
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
    Ok(output)
}

fn is_transient_opencli_failure(message: &str) -> bool {
    let text = message.to_ascii_lowercase();
    text.contains("network error")
        || text.contains("inspected target navigated or closed")
        || text.contains("target closed")
        || text.contains("navigation failed")
        || text.contains("timed out")
}

const fn backoff_delay(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_millis(700),
        2 => Duration::from_millis(1400),
        _ => Duration::from_millis(2200),
    }
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

/// Exported for CLI logging so users can see source-specific location rewrite.
pub fn normalized_location_for_source(source: &str, location: &str) -> String {
    normalize_location_for_source(source, location)
}

fn normalize_location_for_source(source: &str, location: &str) -> String {
    let location = location.trim();
    if source != "linkedin" {
        return location.to_string();
    }

    // For Chinese cities, explicitly add country to avoid LinkedIn resolving
    // to similarly named US locations.
    let han_key = location
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(*c, ',' | '，' | '-' | '·'))
        .collect::<String>();
    match han_key.as_str() {
        "上海" | "上海市" => return "Shanghai China".to_string(),
        "北京" | "北京市" => return "Beijing China".to_string(),
        "深圳" | "深圳市" => return "Shenzhen China".to_string(),
        "广州" | "广州市" => return "Guangzhou China".to_string(),
        "杭州" | "杭州市" => return "Hangzhou China".to_string(),
        "苏州" | "苏州市" => return "Suzhou China".to_string(),
        "南京" | "南京市" => return "Nanjing China".to_string(),
        "成都" | "成都市" => return "Chengdu China".to_string(),
        "武汉" | "武汉市" => return "Wuhan China".to_string(),
        "西安" | "西安市" => return "Xi'an China".to_string(),
        "天津" | "天津市" => return "Tianjin China".to_string(),
        "重庆" | "重庆市" => return "Chongqing China".to_string(),
        "香港" => return "Hong Kong China".to_string(),
        _ => {}
    }

    if !location.is_ascii() {
        return location.to_string();
    }

    let key = location
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();

    match key.as_str() {
        "shanghai" => "Shanghai China".to_string(),
        "beijing" => "Beijing China".to_string(),
        "shenzhen" => "Shenzhen China".to_string(),
        "guangzhou" => "Guangzhou China".to_string(),
        "hangzhou" => "Hangzhou China".to_string(),
        "suzhou" => "Suzhou China".to_string(),
        "nanjing" => "Nanjing China".to_string(),
        "chengdu" => "Chengdu China".to_string(),
        "wuhan" => "Wuhan China".to_string(),
        "xian" => "Xi'an China".to_string(),
        "tianjin" => "Tianjin China".to_string(),
        "chongqing" => "Chongqing China".to_string(),
        "hongkong" => "Hong Kong China".to_string(),
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
    title:     Option<String>,
    #[serde(
        alias = "brandName",
        alias = "brand_name",
        alias = "companyName",
        alias = "company"
    )]
    company:   Option<String>,
    #[serde(alias = "url", alias = "link")]
    jd_url:    Option<String>,
    #[serde(alias = "description", alias = "jd")]
    jd_text:   Option<String>,
    #[serde(
        alias = "cityName",
        alias = "city_name",
        alias = "city",
        alias = "location",
        alias = "area"
    )]
    location:  Option<String>,
    #[serde(alias = "salaryDesc", alias = "salary_desc", alias = "salary")]
    salary:    Option<String>,
    #[serde(
        alias = "listed",
        alias = "posted_at",
        alias = "postedAt",
        alias = "publish_time",
        alias = "publishedAt",
        alias = "activeTime"
    )]
    posted_at: Option<String>,
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
            .maybe_posted_at(self.posted_at)
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
                "salary": "$180-220K",
                "listed": "1 day ago"
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
        assert_eq!(jobs[0].posted_at.as_deref(), Some("1 day ago"));
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
    fn linkedin_location_common_cn_cities_are_disambiguated() {
        assert_eq!(
            normalize_location_for_source("linkedin", "shanghai"),
            "Shanghai China"
        );
        assert_eq!(
            normalize_location_for_source("linkedin", "  shanghai "),
            "Shanghai China"
        );
        assert_eq!(
            normalize_location_for_source("linkedin", "beijing"),
            "Beijing China"
        );
        assert_eq!(
            normalize_location_for_source("linkedin", "Xi'an"),
            "Xi'an China"
        );
        assert_eq!(
            normalize_location_for_source("linkedin", "hong kong"),
            "Hong Kong China"
        );
    }

    #[test]
    fn linkedin_location_chinese_input_is_disambiguated() {
        assert_eq!(
            normalize_location_for_source("linkedin", "上海"),
            "Shanghai China"
        );
        assert_eq!(
            normalize_location_for_source("linkedin", "上海市"),
            "Shanghai China"
        );
    }

    #[test]
    fn linkedin_location_unknown_ascii_is_preserved() {
        assert_eq!(normalize_location_for_source("linkedin", "tokyo"), "tokyo");
    }

    #[test]
    fn non_linkedin_location_is_preserved() {
        assert_eq!(
            normalize_location_for_source("boss", "shanghai"),
            "shanghai"
        );
    }

    #[test]
    fn transient_failure_detection_matches_known_boss_errors() {
        assert!(is_transient_opencli_failure("Error: Network Error"));
        assert!(is_transient_opencli_failure(
            "Inspected target navigated or closed"
        ));
        assert!(!is_transient_opencli_failure("unsupported source"));
    }
}
