//! Input validation for domain values.

use snafu::ensure;

use crate::error::{self, Result};

/// Validate a date string (YYYY-MM-DD or ISO 8601 datetime).
pub fn validate_date(input: &str) -> Result<()> {
    let valid = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d").is_ok()
        || chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S").is_ok()
        || chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S").is_ok();
    ensure!(valid, error::InvalidDateSnafu { input });
    Ok(())
}

/// Validate a URL string (must start with http:// or https://).
pub fn validate_url(input: &str) -> Result<()> {
    ensure!(
        input.starts_with("http://") || input.starts_with("https://"),
        error::InvalidUrlSnafu { input }
    );
    Ok(())
}
