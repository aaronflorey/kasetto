use reqwest::header::{HeaderMap, AUTHORIZATION, RETRY_AFTER};
use reqwest::StatusCode;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::banner::print_banner_or_plain;
use crate::colors::{ACCENT, INFO, RESET, SECONDARY};
use crate::error::{err, Result};
use crate::fsops::http_client;
use crate::profile::{format_updated_ago, list_color_enabled};
use crate::ui::{animations_enabled, print_field, print_json, print_section_header, with_spinner};

const SKILLSMP_API_BASE: &str = "https://skillsmp.com/api/v1/skills";
const SKILLSMP_API_KEY_ENV: &str = "SKILLSMP_API_KEY";

#[derive(serde::Deserialize)]
struct Skill {
    id: String,
    name: String,
    author: String,
    description: String,
    #[serde(rename = "githubUrl")]
    github_url: String,
    #[serde(rename = "skillUrl")]
    skill_url: String,
    stars: u64,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

#[derive(serde::Deserialize)]
struct KeywordSearchResponse {
    data: KeywordSearchData,
    meta: Option<SearchRequestMeta>,
}

#[derive(serde::Deserialize)]
struct KeywordSearchData {
    skills: Vec<Skill>,
    pagination: SearchPagination,
}

#[derive(serde::Deserialize)]
struct SemanticSearchResponse {
    data: SemanticSearchData,
    meta: Option<SearchRequestMeta>,
}

#[derive(serde::Deserialize)]
struct SemanticSearchData {
    data: Vec<SemanticMatch>,
}

#[derive(serde::Deserialize)]
struct SemanticMatch {
    #[serde(rename = "file_id")]
    file_id: String,
    filename: String,
    score: f64,
    skill: Skill,
}

#[derive(serde::Deserialize)]
struct SearchApiErrorResponse {
    error: SearchApiError,
}

#[derive(serde::Deserialize)]
struct SearchApiError {
    code: String,
    message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SearchRequestMeta {
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "responseTimeMs")]
    response_time_ms: u64,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct SearchPagination {
    page: u64,
    limit: u64,
    total: u64,
    #[serde(rename = "totalPages")]
    total_pages: u64,
    #[serde(default, rename = "hasNext")]
    has_next: Option<bool>,
    #[serde(default, rename = "hasPrev")]
    has_prev: Option<bool>,
    #[serde(default, rename = "totalIsExact")]
    total_is_exact: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
struct RateLimitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    daily_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    daily_remaining: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minute_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minute_remaining: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
struct SearchOutput {
    query: String,
    semantic: bool,
    endpoint: String,
    result_count: usize,
    results: Vec<SearchResultItem>,
    rate_limit: RateLimitInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pagination: Option<SearchPagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<SearchRequestMeta>,
}

#[derive(Debug, serde::Serialize)]
struct SearchResultItem {
    rank: usize,
    id: String,
    name: String,
    author: String,
    description: String,
    github_url: String,
    skill_url: String,
    stars: u64,
    updated_at: String,
    updated_ago: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_file_id: Option<String>,
}

pub(crate) fn run(
    query_terms: &[String],
    as_json: bool,
    semantic: bool,
    api_key_arg: Option<&str>,
) -> Result<()> {
    let query = normalize_query(query_terms);
    if query.is_empty() {
        return Err(err("Search query cannot be empty."));
    }
    let api_key = resolve_api_key(api_key_arg, std::env::var(SKILLSMP_API_KEY_ENV).ok());

    if semantic && api_key.is_none() {
        return Err(err(format!(
            "SkillsMP semantic search requires an API key. Pass --api-key or set ${SKILLSMP_API_KEY_ENV}."
        )));
    }

    let color = list_color_enabled();
    let animate = animations_enabled(false, as_json, !color);

    if !as_json && std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        print_banner_or_plain(!color || !animate);
        println!();
    }

    let label = if semantic {
        format!("Checking SkillsMP semantic search for \"{query}\"")
    } else {
        format!("Checking SkillsMP search for \"{query}\"")
    };

    let output = with_spinner(animate, !color, label, || {
        fetch_search_results(&query, semantic, api_key.as_deref())
    })?;

    if as_json {
        return print_json(&output);
    }

    render_search_results(&output, color);
    Ok(())
}

fn fetch_search_results(
    query: &str,
    semantic: bool,
    api_key: Option<&str>,
) -> Result<SearchOutput> {
    let endpoint = if semantic { "ai-search" } else { "search" };
    let url = format!("{SKILLSMP_API_BASE}/{endpoint}");

    let mut request = http_client()?.get(&url).query(&[("q", query)]);
    if let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) {
        request = request.header(AUTHORIZATION, format!("Bearer {api_key}"));
    }

    let response = request
        .send()
        .map_err(|e| err(format!("failed to query SkillsMP: {e}")))?;

    let status = response.status();
    let rate_limit = RateLimitInfo::from_headers(response.headers());
    let text = response
        .text()
        .map_err(|e| err(format!("failed to read SkillsMP response: {e}")))?;

    if !status.is_success() {
        return Err(err(format_api_error(status, &text, semantic, &rate_limit)));
    }

    if semantic {
        let parsed: SemanticSearchResponse = serde_json::from_str(&text).map_err(|e| {
            err(format!(
                "failed to parse SkillsMP semantic search response: {e}"
            ))
        })?;
        Ok(normalize_semantic_output(
            query, endpoint, parsed, rate_limit,
        ))
    } else {
        let parsed: KeywordSearchResponse = serde_json::from_str(&text)
            .map_err(|e| err(format!("failed to parse SkillsMP search response: {e}")))?;
        Ok(normalize_keyword_output(
            query, endpoint, parsed, rate_limit,
        ))
    }
}

fn normalize_keyword_output(
    query: &str,
    endpoint: &str,
    response: KeywordSearchResponse,
    rate_limit: RateLimitInfo,
) -> SearchOutput {
    let results = response
        .data
        .skills
        .into_iter()
        .enumerate()
        .map(|(idx, skill)| SearchResultItem {
            rank: idx + 1,
            updated_ago: format_updated_ago(&skill.updated_at),
            id: skill.id,
            name: skill.name,
            author: skill.author,
            description: skill.description,
            github_url: skill.github_url,
            skill_url: skill.skill_url,
            stars: skill.stars,
            updated_at: skill.updated_at,
            semantic_score: None,
            source_file: None,
            source_file_id: None,
        })
        .collect::<Vec<_>>();

    SearchOutput {
        query: query.to_string(),
        semantic: false,
        endpoint: format!("/api/v1/skills/{endpoint}"),
        result_count: results.len(),
        results,
        rate_limit,
        pagination: Some(response.data.pagination),
        request: response.meta,
    }
}

fn normalize_semantic_output(
    query: &str,
    endpoint: &str,
    response: SemanticSearchResponse,
    rate_limit: RateLimitInfo,
) -> SearchOutput {
    let results = response
        .data
        .data
        .into_iter()
        .enumerate()
        .map(|(idx, matched)| SearchResultItem {
            rank: idx + 1,
            updated_ago: format_updated_ago(&matched.skill.updated_at),
            id: matched.skill.id,
            name: matched.skill.name,
            author: matched.skill.author,
            description: matched.skill.description,
            github_url: matched.skill.github_url,
            skill_url: matched.skill.skill_url,
            stars: matched.skill.stars,
            updated_at: matched.skill.updated_at,
            semantic_score: Some(matched.score),
            source_file: Some(matched.filename),
            source_file_id: Some(matched.file_id),
        })
        .collect::<Vec<_>>();

    SearchOutput {
        query: query.to_string(),
        semantic: true,
        endpoint: format!("/api/v1/skills/{endpoint}"),
        result_count: results.len(),
        results,
        rate_limit,
        pagination: None,
        request: response.meta,
    }
}

fn format_api_error(
    status: StatusCode,
    response_text: &str,
    semantic: bool,
    rate_limit: &RateLimitInfo,
) -> String {
    let base = match serde_json::from_str::<SearchApiErrorResponse>(response_text) {
        Ok(api_error) => format!(
            "SkillsMP API error ({status}) {}: {}",
            api_error.error.code, api_error.error.message,
        ),
        Err(_) => format!("SkillsMP API error ({status}): {response_text}"),
    };

    let auth_hint = if semantic && status == StatusCode::UNAUTHORIZED {
        format!(" Pass --api-key or set ${SKILLSMP_API_KEY_ENV}.")
    } else {
        String::new()
    };

    let rate_limit_hint = rate_limit.error_hint();
    format!("{base}{auth_hint}{rate_limit_hint}")
}

fn render_search_results(output: &SearchOutput, color: bool) {
    let mode = if output.semantic {
        "semantic"
    } else {
        "keyword"
    };
    let matches = match &output.pagination {
        Some(pagination) => format!(
            "{} shown of {} total",
            output.result_count, pagination.total
        ),
        None => output.result_count.to_string(),
    };

    print_field("Query", &output.query, color);
    print_field("Mode", mode, color);
    print_field("Matches", &matches, color);

    if let Some(quota) = output.rate_limit.display_summary() {
        print_field("Quota", &quota, color);
    }

    if let Some(request) = &output.request {
        let request_text = format!("{} ms ({})", request.response_time_ms, request.request_id);
        print_field("Request", &request_text, color);
    }

    println!();

    if output.results.is_empty() {
        println!("No skills found.");
        return;
    }

    print_section_header("Results", output.results.len(), color);
    println!();

    for (idx, result) in output.results.iter().enumerate() {
        if idx > 0 {
            println!();
        }
        render_result(result, output.semantic, color);
    }
}

fn render_result(result: &SearchResultItem, semantic: bool, color: bool) {
    println!("{}", format_heading(result, color));
    println!("   {}", format_meta(result, semantic));

    for line in wrap_description(&result.description, description_width()) {
        println!("   {line}");
    }

    if !result.skill_url.is_empty() {
        println!("   {}", format_link("SkillsMP", &result.skill_url, color));
    }
    if !result.github_url.is_empty() {
        println!("   {}", format_link("GitHub", &result.github_url, color));
    }
}

fn format_heading(result: &SearchResultItem, color: bool) -> String {
    if color {
        format!(
            "{ACCENT}{}.{RESET} {INFO}{}{RESET} {SECONDARY}by {}{RESET}",
            result.rank, result.name, result.author
        )
    } else {
        format!("{}. {} by {}", result.rank, result.name, result.author)
    }
}

fn format_meta(result: &SearchResultItem, semantic: bool) -> String {
    let mut parts = vec![format!("{}*", result.stars), result.updated_ago.clone()];

    if semantic {
        if let Some(score) = result.semantic_score {
            parts.push(format!("score {score:.3}"));
        }
        if let Some(source_file) = result.source_file.as_deref() {
            parts.push(truncate_width(source_file, 32));
        }
    }

    parts.join(" | ")
}

fn format_link(label: &str, value: &str, color: bool) -> String {
    if color {
        format!("{SECONDARY}{label}:{RESET} {value}")
    } else {
        format!("{label}: {value}")
    }
}

fn wrap_description(value: &str, width: usize) -> Vec<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() || width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for word in normalized.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);
        let spacer = usize::from(!current.is_empty());
        if current_width + spacer + word_width > width && !current.is_empty() {
            lines.push(current);
            if lines.len() == 2 {
                return truncate_lines(lines, width);
            }
            current = word.to_string();
            current_width = word_width;
        } else {
            if !current.is_empty() {
                current.push(' ');
                current_width += 1;
            }
            current.push_str(word);
            current_width += word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn truncate_lines(mut lines: Vec<String>, width: usize) -> Vec<String> {
    if let Some(last) = lines.last_mut() {
        if UnicodeWidthStr::width(last.as_str()) + 3 <= width {
            last.push_str("...");
        } else {
            *last = truncate_width(last, width);
        }
    }
    lines
}

fn truncate_width(text: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return ".".to_string();
    }
    if max_width == 2 {
        return "..".to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width + 3 > max_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out.push_str("...");
    out
}

fn description_width() -> usize {
    let terminal_width = if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        crossterm::terminal::size()
            .map(|(width, _)| usize::from(width))
            .ok()
    } else {
        None
    };

    terminal_width
        .unwrap_or(100)
        .saturating_sub(3)
        .clamp(48, 100)
}

fn normalize_query(query_terms: &[String]) -> String {
    query_terms
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_api_key(api_key_arg: Option<&str>, api_key_env: Option<String>) -> Option<String> {
    api_key_arg
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| api_key_env.map(|key| key.trim().to_string()))
        .filter(|key| !key.is_empty())
}

impl RateLimitInfo {
    fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            daily_limit: header_u64(headers, "x-ratelimit-daily-limit"),
            daily_remaining: header_u64(headers, "x-ratelimit-daily-remaining"),
            minute_limit: header_u64(headers, "x-ratelimit-minute-limit"),
            minute_remaining: header_u64(headers, "x-ratelimit-minute-remaining"),
            retry_after_seconds: headers
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok()),
        }
    }

    fn display_summary(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let (Some(remaining), Some(limit)) = (self.minute_remaining, self.minute_limit) {
            parts.push(format!("minute {remaining}/{limit} remaining"));
        }
        if let (Some(remaining), Some(limit)) = (self.daily_remaining, self.daily_limit) {
            parts.push(format!("daily {remaining}/{limit} remaining"));
        }
        if let Some(retry_after) = self.retry_after_seconds {
            parts.push(format!("retry after {retry_after}s"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }

    fn error_hint(&self) -> String {
        match self.display_summary() {
            Some(summary) => format!(" Rate limit: {summary}."),
            None => String::new(),
        }
    }
}

fn header_u64(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn normalize_query_joins_terms() {
        let query = normalize_query(&["  rust".into(), "cli  ".into(), "".into()]);
        assert_eq!(query, "rust cli");
    }

    #[test]
    fn resolve_api_key_prefers_cli_argument() {
        let key = resolve_api_key(Some("from-flag"), Some("from-env".to_string()));
        assert_eq!(key.as_deref(), Some("from-flag"));
    }

    #[test]
    fn rate_limit_info_reads_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-daily-limit", HeaderValue::from_static("500"));
        headers.insert(
            "x-ratelimit-daily-remaining",
            HeaderValue::from_static("499"),
        );
        headers.insert("x-ratelimit-minute-limit", HeaderValue::from_static("30"));
        headers.insert(
            "x-ratelimit-minute-remaining",
            HeaderValue::from_static("29"),
        );
        headers.insert(RETRY_AFTER, HeaderValue::from_static("12"));

        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.daily_limit, Some(500));
        assert_eq!(info.daily_remaining, Some(499));
        assert_eq!(info.minute_limit, Some(30));
        assert_eq!(info.minute_remaining, Some(29));
        assert_eq!(info.retry_after_seconds, Some(12));
    }

    #[test]
    fn keyword_search_response_normalizes_results() {
        let response: KeywordSearchResponse = serde_json::from_str(
            r#"{
                "data": {
                    "skills": [
                        {
                            "id": "skill-1",
                            "name": "rust-test",
                            "author": "alice",
                            "description": "A Rust skill",
                            "githubUrl": "https://github.com/example/rust-test",
                            "skillUrl": "https://skillsmp.com/skills/rust-test",
                            "stars": 42,
                            "updatedAt": "1700000000"
                        }
                    ],
                    "pagination": {
                        "page": 1,
                        "limit": 20,
                        "total": 1,
                        "totalPages": 1,
                        "hasNext": false,
                        "hasPrev": false,
                        "totalIsExact": true
                    }
                },
                "meta": {
                    "requestId": "req-1",
                    "responseTimeMs": 123
                }
            }"#,
        )
        .expect("parse keyword response");

        let output = normalize_keyword_output(
            "rust",
            "search",
            response,
            RateLimitInfo {
                daily_limit: Some(50),
                daily_remaining: Some(49),
                minute_limit: Some(10),
                minute_remaining: Some(9),
                retry_after_seconds: None,
            },
        );

        assert_eq!(output.result_count, 1);
        assert!(!output.semantic);
        assert_eq!(output.results[0].name, "rust-test");
        assert_eq!(output.pagination.as_ref().map(|p| p.total), Some(1));
    }
}
