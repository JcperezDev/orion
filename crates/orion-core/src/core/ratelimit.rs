//! Classify provider errors so the agent loop can react like opencode: a
//! transient rate-limit is retried with backoff, while a hard usage limit
//! checkpoints the work and surfaces a "resume later" signal instead of just
//! failing the run.

use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    /// Hit a rate / usage limit. `retry_after` (seconds) is parsed from the
    /// message when the provider tells us when it resets.
    RateLimited { retry_after: Option<u64> },
    /// Provider is temporarily overloaded (e.g. Anthropic 529, HTTP 503).
    Overloaded,
    /// Network blip / 5xx — safe to retry quickly.
    Transient,
    /// Anything else — not worth retrying.
    Fatal,
}

impl ErrorClass {
    /// Should the agent loop retry this error at all?
    pub fn is_retryable(&self) -> bool {
        !matches!(self, ErrorClass::Fatal)
    }

    /// Is this a usage-limit the user must wait out (vs a quick blip)?
    pub fn is_limit(&self) -> bool {
        matches!(self, ErrorClass::RateLimited { .. })
    }
}

/// Parse a retry/reset hint (in seconds) out of a free-form error message.
/// Handles "retry-after: 30", "try again in 45 seconds", "resets in 2 minutes",
/// "available in 1h", etc.
pub fn parse_retry_after(msg: &str) -> Option<u64> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:retry[- ]?after|try again in|again in|resets? in|available in|wait)\D{0,8}(\d+)\s*(seconds?|secs?|s|minutes?|mins?|m|hours?|hrs?|h)?",
        )
        .unwrap()
    });
    let caps = re.captures(msg)?;
    let n: u64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
    let secs = match unit.chars().next() {
        Some('m') if unit.starts_with("min") || unit == "m" => n * 60,
        Some('h') => n * 3600,
        _ => n, // seconds (or unitless)
    };
    Some(secs)
}

/// Classify a provider error message.
pub fn classify_error(msg: &str) -> ErrorClass {
    let m = msg.to_lowercase();

    let rate_limited = m.contains("rate limit")
        || m.contains("rate_limit")
        || m.contains("ratelimit")
        || m.contains("too many requests")
        || m.contains("status 429")
        || m.contains("429")
        || m.contains("quota")
        || m.contains("usage limit")
        || m.contains("usage_limit")
        || m.contains("limit reached")
        || m.contains("insufficient_quota");
    if rate_limited {
        return ErrorClass::RateLimited { retry_after: parse_retry_after(&m) };
    }

    let overloaded = m.contains("overloaded")
        || m.contains("529")
        || m.contains("503")
        || m.contains("service unavailable")
        || m.contains("server is busy")
        || m.contains("capacity");
    if overloaded {
        return ErrorClass::Overloaded;
    }

    let transient = m.contains("timed out")
        || m.contains("timeout")
        || m.contains("connection reset")
        || m.contains("connection closed")
        || m.contains("broken pipe")
        || m.contains("502")
        || m.contains("504")
        || m.contains("bad gateway")
        || m.contains("gateway timeout")
        || m.contains("temporarily");
    if transient {
        return ErrorClass::Transient;
    }

    ErrorClass::Fatal
}

/// Exponential backoff for retry `attempt` (0-based), honoring a provider's
/// `retry_after` hint, capped at `max`.
pub fn backoff_delay(attempt: u32, retry_after: Option<u64>, max: Duration) -> Duration {
    if let Some(secs) = retry_after {
        return Duration::from_secs(secs).min(max);
    }
    // 1s, 2s, 4s, 8s, ... capped.
    let base = 1u64 << attempt.min(6); // up to 64s before cap
    Duration::from_secs(base).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_rate_limits() {
        assert_eq!(
            classify_error("HTTP status 429: rate limit exceeded"),
            ErrorClass::RateLimited { retry_after: None }
        );
        assert!(matches!(
            classify_error("You have hit your usage limit, resets in 5 minutes"),
            ErrorClass::RateLimited { retry_after: Some(300) }
        ));
        assert!(matches!(
            classify_error("Too Many Requests"),
            ErrorClass::RateLimited { .. }
        ));
    }

    #[test]
    fn classifies_overloaded_and_transient() {
        assert_eq!(classify_error("Error 529: overloaded"), ErrorClass::Overloaded);
        assert_eq!(classify_error("503 Service Unavailable"), ErrorClass::Overloaded);
        assert_eq!(classify_error("connection reset by peer"), ErrorClass::Transient);
        assert_eq!(classify_error("request timed out"), ErrorClass::Transient);
    }

    #[test]
    fn classifies_fatal() {
        assert_eq!(classify_error("invalid api key"), ErrorClass::Fatal);
        assert_eq!(classify_error("model not found"), ErrorClass::Fatal);
        assert!(!ErrorClass::Fatal.is_retryable());
        assert!(ErrorClass::Transient.is_retryable());
    }

    #[test]
    fn parses_retry_after_units() {
        assert_eq!(parse_retry_after("retry-after: 30"), Some(30));
        assert_eq!(parse_retry_after("try again in 45 seconds"), Some(45));
        assert_eq!(parse_retry_after("resets in 2 minutes"), Some(120));
        assert_eq!(parse_retry_after("available in 1 hour"), Some(3600));
        assert_eq!(parse_retry_after("no hint here"), None);
    }

    #[test]
    fn backoff_respects_hint_and_cap() {
        let max = Duration::from_secs(30);
        assert_eq!(backoff_delay(0, Some(10), max), Duration::from_secs(10));
        // hint larger than cap is clamped
        assert_eq!(backoff_delay(0, Some(600), max), Duration::from_secs(30));
        // exponential without hint
        assert_eq!(backoff_delay(0, None, max), Duration::from_secs(1));
        assert_eq!(backoff_delay(2, None, max), Duration::from_secs(4));
        // capped
        assert_eq!(backoff_delay(10, None, max), Duration::from_secs(30));
    }

    #[test]
    fn is_limit_only_for_rate_limited() {
        assert!(ErrorClass::RateLimited { retry_after: None }.is_limit());
        assert!(!ErrorClass::Overloaded.is_limit());
        assert!(!ErrorClass::Transient.is_limit());
    }
}
