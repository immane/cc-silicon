//! Lexical validation of corpus-relative asset paths.
//!
//! Every lock path is corpus-relative POSIX text. Validation rejects anything
//! that could escape the supplied corpus root or be ambiguous across platforms;
//! physical symlink escapes are checked separately during verification.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathProblem {
    Empty,
    Absolute,
    Traversal,
    Backslash,
    Colon,
    ControlChar,
    DotComponent,
    EmptyComponent,
    TrailingSeparator,
}

impl PathProblem {
    /// Stable diagnostic code.
    pub fn code(self) -> &'static str {
        match self {
            PathProblem::Empty => "path.empty",
            PathProblem::Absolute => "path.absolute",
            PathProblem::Traversal => "path.traversal",
            PathProblem::Backslash => "path.backslash",
            PathProblem::Colon => "path.colon",
            PathProblem::ControlChar => "path.control-char",
            PathProblem::DotComponent => "path.dot-component",
            PathProblem::EmptyComponent => "path.empty-component",
            PathProblem::TrailingSeparator => "path.trailing-separator",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            PathProblem::Empty => "path must not be empty",
            PathProblem::Absolute => "path must be relative to the corpus root",
            PathProblem::Traversal => "path must not contain '..' components",
            PathProblem::Backslash => "path must use '/' separators, not backslashes",
            PathProblem::Colon => "path must not contain ':' (drive or scheme prefix)",
            PathProblem::ControlChar => "path must not contain control characters",
            PathProblem::DotComponent => "path must not contain '.' components",
            PathProblem::EmptyComponent => "path must not contain empty components",
            PathProblem::TrailingSeparator => "path must not end with '/'",
        }
    }
}

impl fmt::Display for PathProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.describe())
    }
}

pub fn validate_relative_path(raw: &str) -> Result<(), PathProblem> {
    if raw.is_empty() {
        return Err(PathProblem::Empty);
    }
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(PathProblem::Absolute);
    }
    if raw.contains('\\') {
        return Err(PathProblem::Backslash);
    }
    if raw.contains(':') {
        return Err(PathProblem::Colon);
    }
    if raw.chars().any(char::is_control) {
        return Err(PathProblem::ControlChar);
    }
    if raw.ends_with('/') {
        return Err(PathProblem::TrailingSeparator);
    }
    for component in raw.split('/') {
        match component {
            "" => return Err(PathProblem::EmptyComponent),
            "." => return Err(PathProblem::DotComponent),
            ".." => return Err(PathProblem::Traversal),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_relative_paths() {
        assert!(validate_relative_path("gcc/testsuite/lib/c-torture.exp").is_ok());
        assert!(validate_relative_path("gcc.c-torture/execute/20000101-1.c").is_ok());
    }

    #[test]
    fn rejects_escape_and_ambiguity() {
        assert_eq!(validate_relative_path(""), Err(PathProblem::Empty));
        assert_eq!(
            validate_relative_path("/etc/passwd"),
            Err(PathProblem::Absolute)
        );
        assert_eq!(
            validate_relative_path("../x.c"),
            Err(PathProblem::Traversal)
        );
        assert_eq!(
            validate_relative_path("a/../../b.c"),
            Err(PathProblem::Traversal)
        );
        assert_eq!(
            validate_relative_path("a\\b.c"),
            Err(PathProblem::Backslash)
        );
        assert_eq!(validate_relative_path("C:/x.c"), Err(PathProblem::Colon));
        assert_eq!(
            validate_relative_path("a/./b.c"),
            Err(PathProblem::DotComponent)
        );
        assert_eq!(
            validate_relative_path("a//b.c"),
            Err(PathProblem::EmptyComponent)
        );
        assert_eq!(
            validate_relative_path("a/b/"),
            Err(PathProblem::TrailingSeparator)
        );
        assert_eq!(
            validate_relative_path("a/\u{7}b.c"),
            Err(PathProblem::ControlChar)
        );
    }
}
