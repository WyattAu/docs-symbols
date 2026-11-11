//! docs-symbols: symbol-anchored source extraction for docs sites
//! (docs-core extension program, `SPEC.md` REQ-S).
//!
//! A docs compiler resolves ```include-symbol fences against real source
//! files: this crate parses the file with tree-sitter, walks the AST by
//! ordered name matching (modules, items, impls keyed on type name), and
//! returns the symbol's exact byte span plus a position-independent
//! anchor ID derived from the symbol path itself (REQ-S1/S3). Unknown
//! paths fail with typed errors naming the offending segment (REQ-S2).
//!
//! ```
//! use docs_symbols::{extract, Language};
//!
//! let source = "mod engine {\n    fn process_order() -> u32 { 7 }\n}\n";
//! let span = extract(source, Language::Rust, "engine::process_order").unwrap();
//! assert_eq!(span.start_line, 2);
//! assert!(span.code.contains("fn process_order"));
//! assert!(span.anchor_id.starts_with("dsym-rust-"));
//! ```
//!
//! # Scope (0.1, honest)
//!
//! In: Rust only (`Language::Rust`); item kinds function/struct/enum/
//! trait/type/const/static/mod/macro; impl methods keyed on the impl's
//! type name (inherent and trait impls alike); document order as the
//! deterministic first-match tiebreak; strict parsing (any ERROR or
//! MISSING node rejects the source).
//! Out: other languages (grammars are added per language, each gated by
//! its own tests), generic-parameter-qualified paths (`Vec::<u8>::new`),
//! trait-name segments on trait impls, associated consts via type paths,
//! and doc-comment inclusion in spans (the span is the item node). Each
//! absence is a later increment, not silent.
//!
//! # Layer
//!
//! L0 - leaf. Zero estate-internal dependencies; the external floor is
//! `tree-sitter` and `tree-sitter-rust`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod resolve;
mod span;

pub use error::SymbolError;
pub use resolve::extract;
pub use span::Span;

/// Languages with a grammar compiled into this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// The Rust grammar (`tree-sitter-rust`).
    Rust,
}

impl Language {
    /// The language label used in anchor IDs and span metadata.
    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "rust",
        }
    }

    /// The compiled tree-sitter grammar.
    pub fn grammar(self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
        }
    }
}

/// The crate's version, for callers that report it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
