//! Ordered, deterministic diagnostics shared by the lock parser and verifier.

use std::fmt;

/// Severity order is also the report order: errors before warnings before info.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Severity::Error => 0,
            Severity::Warning => 1,
            Severity::Info => 2,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single verification finding with a stable code and a source location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Stable machine-readable code used by fixtures and downstream tooling.
    pub code: &'static str,
    /// Location such as `lock:12`, `asset:<path>`, `release.source-archive`, or `root`.
    pub location: String,
    pub message: String,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            location: location.into(),
            message: message.into(),
        }
    }

    pub fn error(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, code, location, message)
    }

    pub fn warning(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warning, code, location, message)
    }

    pub fn info(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Info, code, location, message)
    }

    fn sort_key(&self) -> (u8, &'static str, &str, &str) {
        (
            self.severity.rank(),
            self.code,
            self.location.as_str(),
            self.message.as_str(),
        )
    }

    /// Sort diagnostics into a stable order and drop exact duplicates.
    ///
    /// Hash-map iteration and filesystem traversal order must never leak into the
    /// report, so every consumer funnels through this deterministic ordering.
    pub fn sort(diagnostics: &mut Vec<Diagnostic>) {
        diagnostics.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        diagnostics.dedup();
    }
}

/// Escape control characters in report labels, paths, and messages so that a
/// hostile filename or command-line argument cannot forge additional diagnostic
/// lines in the rendered report.
pub fn escape_control(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Unicode line/paragraph separators are not `is_control()` but still
            // break report lines, so they are escaped explicitly.
            '\u{2028}' => out.push_str("\\u{2028}"),
            '\u{2029}' => out.push_str("\\u{2029}"),
            character if character.is_control() => {
                out.push_str(&format!("\\x{:02x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_control_characters() {
        assert_eq!(escape_control("a\nb\tc\u{7}d"), "a\\nb\\tc\\x07d");
        assert_eq!(escape_control("plain/path.c"), "plain/path.c");
    }

    #[test]
    fn escapes_unicode_line_separators() {
        assert_eq!(
            escape_control("a\u{2028}b\u{2029}c"),
            "a\\u{2028}b\\u{2029}c"
        );
    }
}
