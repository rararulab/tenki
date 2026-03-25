//! Shared output parsing utilities for agent CLI responses.
//!
//! When backends emit stream-json (NDJSON), system/hook event lines precede
//! the actual assistant response. These helpers extract the meaningful result
//! so that downstream JSON parsing is not confused by framework events.

/// Extract the final result text from Claude stream-json NDJSON output.
///
/// Stream-json output contains one JSON event per line. System/hook events
/// (e.g. `{"type":"system","subtype":"hook_started",...}`) appear before the
/// actual assistant response. The result lives in a line with
/// `{"type":"result","result":"<text>"}`. Returns `None` if the output does
/// not look like NDJSON stream-json, or if stream events are present but no
/// result line was found.
pub fn extract_result_from_stream_json(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
            && v.get("type").and_then(serde_json::Value::as_str) == Some("result")
        {
            return v
                .get("result")
                .and_then(serde_json::Value::as_str)
                .map(String::from);
        }
    }
    // No result line found — return None so callers fall back to raw output.
    None
}

/// Extract JSON content from markdown code fences.
pub fn extract_fenced_json(text: &str) -> Option<&str> {
    let start_markers = ["```json\n", "```json\r\n", "```\n", "```\r\n"];
    for marker in &start_markers {
        if let Some(start) = text.find(marker) {
            let json_start = start + marker.len();
            if let Some(end) = text[json_start..].find("```") {
                return Some(text[json_start..json_start + end].trim());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_stream_json_with_result() {
        let input = r#"{"type":"system","subtype":"hook_started","hook_id":"abc"}
{"type":"system","subtype":"hook_completed","hook_id":"abc"}
{"type":"assistant","message":{"content":[{"type":"text","text":"here is the score"}]}}
{"type":"result","result":"{\"key\":\"value\"}","subtype":"success"}"#;
        let result = extract_result_from_stream_json(input);
        assert_eq!(result, Some(r#"{"key":"value"}"#.to_string()));
    }

    #[test]
    fn test_extract_stream_json_no_result_returns_none() {
        let input = r#"{"type":"system","subtype":"hook_started","hook_id":"abc"}
{"type":"system","subtype":"hook_completed","hook_id":"abc"}"#;
        assert!(extract_result_from_stream_json(input).is_none());
    }

    #[test]
    fn test_extract_stream_json_plain_text_returns_none() {
        let input = "Just some plain text output with no NDJSON";
        assert!(extract_result_from_stream_json(input).is_none());
    }

    #[test]
    fn test_extract_fenced_json() {
        let input = "Here:\n```json\n{\"a\":1}\n```\n";
        assert_eq!(extract_fenced_json(input), Some("{\"a\":1}"));
    }

    #[test]
    fn test_extract_fenced_json_none() {
        assert!(extract_fenced_json("no fences here").is_none());
    }
}
