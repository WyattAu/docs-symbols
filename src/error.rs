//! Typed, exhaustive errors for symbol extraction.

/// Everything that can go wrong resolving a symbol path.
#[derive(Debug)]
#[non_exhaustive]
pub enum SymbolError {
    /// The source does not parse cleanly (ERROR or MISSING nodes anywhere:
    /// docs repositories compile, so a hostile or broken source must not
    /// silently yield a wrong span).
    Parse {
        /// The first offending node kind and byte offset, for diagnosis.
        reason: String,
    },
    /// The symbol path could not be resolved against the AST.
    Unresolvable {
        /// The full symbol path as given.
        path: String,
        /// The segment that failed and what was visible at that scope.
        reason: String,
    },
    /// The symbol path is malformed (empty, or deeper than the resolver
    /// descends).
    BadPath {
        /// Why the path was rejected.
        reason: String,
    },
    /// The grammar failed to load (a build/packaging fault, not input).
    Grammar(String),
}

impl std::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { reason } => write!(f, "source does not parse cleanly: {reason}"),
            Self::Unresolvable { path, reason } => {
                write!(f, "symbol path `{path}` does not resolve: {reason}")
            }
            Self::BadPath { reason } => write!(f, "bad symbol path: {reason}"),
            Self::Grammar(e) => write!(f, "grammar load failed: {e}"),
        }
    }
}

impl std::error::Error for SymbolError {}
