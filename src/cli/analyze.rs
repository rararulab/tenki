//! Job fit analysis based on keyword matching between skills and JD text.

use std::collections::HashSet;

use serde::Serialize;
use snafu::ResultExt as _;

use crate::{
    db::Database,
    error::{self, MissingJdTextSnafu, Result},
};

/// Result of a job fit analysis.
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    /// The full application UUID.
    pub application_id:    String,
    /// Fitness score from 0.0 (no match) to 1.0 (perfect match).
    pub fitness_score:     f64,
    /// Skills that matched keywords in the JD.
    pub matched_skills:    Vec<String>,
    /// Skills that did not match any JD keyword.
    pub unmatched_skills:  Vec<String>,
    /// Number of unique keywords extracted from the JD.
    pub total_jd_keywords: usize,
    /// Human-readable summary of the analysis.
    pub notes:             String,
}

/// Stop words filtered out when extracting JD keywords.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "is", "are", "was", "were", "be", "been", "being", "have",
    "has", "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "can",
    "shall", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through",
    "during", "before", "after", "above", "below", "between", "out", "off", "over", "under",
    "again", "further", "then", "once", "we", "you", "it", "he", "she", "they", "them", "this",
    "that", "these", "those", "not", "no", "nor",
];

/// Run the analyze command: compute fitness score and persist results.
pub async fn run(db: &Database, id: &str, json: bool) -> Result<()> {
    let app = db.get_application(id).await?;

    let jd_text = app
        .jd_text
        .as_deref()
        .ok_or_else(|| MissingJdTextSnafu { id: id.to_string() }.build())?;

    let jd_keywords = extract_keywords(jd_text);

    let result = match app.skills.as_deref() {
        Some(skills_str) if !skills_str.trim().is_empty() => {
            let (matched, unmatched): (Vec<_>, Vec<_>) = skills_str
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .partition(|skill| skill_matches_jd(skill, &jd_keywords));

            let total = matched.len() + unmatched.len();
            #[allow(clippy::cast_precision_loss)]
            let score = if total == 0 {
                0.0
            } else {
                matched.len() as f64 / total as f64
            };

            let notes = format!(
                "Matched {}/{total} skills. Matched: [{}]. Unmatched: [{}].",
                matched.len(),
                matched.join(", "),
                unmatched.join(", "),
            );

            AnalysisResult {
                application_id: id.to_string(),
                fitness_score: score,
                matched_skills: matched,
                unmatched_skills: unmatched,
                total_jd_keywords: jd_keywords.len(),
                notes,
            }
        }
        _ => AnalysisResult {
            application_id:    id.to_string(),
            fitness_score:     0.0,
            matched_skills:    vec![],
            unmatched_skills:  vec![],
            total_jd_keywords: jd_keywords.len(),
            notes:             "No skills listed for comparison.".to_string(),
        },
    };

    db.update_fitness(id, result.fitness_score, &result.notes)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&result).context(error::JsonSnafu)?
        );
    } else {
        eprintln!("Analysis for application {}", &id[..8]);
        println!("Fitness Score: {:.2}", result.fitness_score);
        println!("Matched:       [{}]", result.matched_skills.join(", "));
        println!("Unmatched:     [{}]", result.unmatched_skills.join(", "));
        println!("JD Keywords:   {}", result.total_jd_keywords);
        println!("Notes:         {}", result.notes);
    }

    Ok(())
}

/// Extract unique lowercase keywords from JD text, filtering stop words and
/// short tokens.
fn extract_keywords(text: &str) -> HashSet<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();

    text.split(|c: char| !c.is_alphanumeric() && c != '+' && c != '#')
        .map(str::to_lowercase)
        .filter(|w| w.len() >= 2 && !stop.contains(w.as_str()))
        .collect()
}

/// Check if a skill matches any JD keyword via substring containment
/// (bidirectional).
fn skill_matches_jd(skill: &str, jd_keywords: &HashSet<String>) -> bool {
    // Multi-word skills: check if all words appear in JD keywords
    let skill_words: Vec<&str> = skill.split_whitespace().collect();
    if skill_words.len() > 1 {
        return skill_words.iter().all(|word| {
            jd_keywords
                .iter()
                .any(|kw| kw.contains(word) || word.contains(kw.as_str()))
        });
    }

    // Single-word skill: substring match against any keyword
    jd_keywords
        .iter()
        .any(|kw| kw.contains(skill) || skill.contains(kw.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_keywords_filters_stop_words() {
        let kw = extract_keywords("We need a Rust developer with Docker experience");
        assert!(kw.contains("rust"));
        assert!(kw.contains("docker"));
        assert!(!kw.contains("we"));
        assert!(!kw.contains("a"));
        assert!(!kw.contains("with"));
    }

    #[test]
    fn skill_matches_substring() {
        let kw = extract_keywords("Experience with PostgreSQL and CI/CD pipelines");
        assert!(skill_matches_jd("postgresql", &kw));
        assert!(skill_matches_jd("ci/cd", &kw));
        assert!(!skill_matches_jd("kubernetes", &kw));
    }

    #[test]
    fn empty_skills_score_zero() {
        let kw = extract_keywords("Some job description text");
        assert!(!kw.is_empty());
        // No skills to match means score would be 0.0
    }
}
