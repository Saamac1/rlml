//! Load diagnostics. Loading reports every problem it can find at once,
//! with the file and line where it was found (M 2.6): load errors stop the
//! game from starting, load warnings are shown and the game starts.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl Diagnostic {
    pub fn error(file: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Diagnostic {
            level: Level::Error,
            file: file.into(),
            line,
            message: message.into(),
        }
    }

    pub fn warning(file: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Diagnostic {
            level: Level::Warning,
            file: file.into(),
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.level {
            Level::Error => "error",
            Level::Warning => "warning",
        };
        if self.line == 0 {
            write!(f, "{}: {level}: {}", self.file, self.message)
        } else {
            write!(f, "{}:{}: {level}: {}", self.file, self.line, self.message)
        }
    }
}

/// Collected diagnostics in the order they were found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report(pub Vec<Diagnostic>);

impl Report {
    pub fn error(&mut self, file: &str, line: usize, message: impl Into<String>) {
        self.0.push(Diagnostic::error(file, line, message));
    }

    pub fn warning(&mut self, file: &str, line: usize, message: impl Into<String>) {
        self.0.push(Diagnostic::warning(file, line, message));
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.0.push(d);
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter(|d| d.level == Level::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter(|d| d.level == Level::Warning)
    }

    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_file_line_and_level() {
        assert_eq!(
            Diagnostic::error("maps/caves.rlml", 12, "unknown character").to_string(),
            "maps/caves.rlml:12: error: unknown character"
        );
        assert_eq!(
            Diagnostic::warning("game.rlml", 0, "x").to_string(),
            "game.rlml: warning: x"
        );
    }

    #[test]
    fn report_separates_levels() {
        let mut r = Report::default();
        r.warning("a", 1, "w");
        assert!(!r.has_errors());
        r.error("b", 2, "e");
        assert!(r.has_errors());
        assert_eq!(r.errors().count(), 1);
        assert_eq!(r.warnings().count(), 1);
    }
}
