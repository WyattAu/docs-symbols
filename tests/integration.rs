//! Golden-path tests: extraction, line-shift invariance, anchor stability.

// Test-only lifts (docs-core decoder-pattern precedent): expectations
// carry context on the setup half of each test and match-arm panics are
// the failure voice. Library paths keep the denies.
#![allow(clippy::expect_used, clippy::panic)]

use docs_symbols::{extract, Language, SymbolError};

/// A small module with items at several depths: a top-level fn, a nested
/// mod, an inherent impl, and a trait impl.
fn fixture() -> String {
    r#"
use std::collections::BTreeMap;

pub struct OrderBook {
    bids: BTreeMap<u64, u64>,
}

pub const MAX_DEPTH: usize = 10;

impl OrderBook {
    pub fn new() -> Self {
        Self { bids: BTreeMap::new() }
    }

    pub fn match_limit(&self, qty: u64) -> u64 {
        qty.min(self.bids.len() as u64)
    }
}

impl Clone for OrderBook {
    fn clone(&self) -> Self {
        Self { bids: self.bids.clone() }
    }
}

pub mod engine {
    pub fn spin_up() -> bool {
        true
    }
}

pub fn process_order(book: &mut OrderBook) {
    let _ = book.match_limit(1);
}
"#
    .to_string()
}

#[test]
fn extracts_top_level_function() {
    let span = extract(&fixture(), Language::Rust, "process_order").expect("resolves");
    assert!(span.code.contains("pub fn process_order"));
    assert!(span.code.contains("book.match_limit(1)"));
    assert_eq!(span.language, "rust");
    assert_eq!(span.start_line, 32, "1-based line in the fixture");
}

#[test]
fn line_shift_is_invariant_for_code_and_anchor() {
    let base = extract(&fixture(), Language::Rust, "process_order").expect("resolves");
    let padding = "// padding line\n".repeat(12);
    let shifted = format!("{padding}{}", fixture());
    let moved = extract(&shifted, Language::Rust, "process_order").expect("resolves");
    assert_eq!(base.code, moved.code, "exact span survives the shift");
    assert_eq!(base.anchor_id, moved.anchor_id, "REQ-S1/S3: ID is stable");
    assert_eq!(base.end_line + 12, moved.end_line, "line metadata shifts");
    assert_eq!(moved.start_line, base.start_line + 12);
}

#[test]
fn resolves_inherent_impl_method() {
    let span = extract(&fixture(), Language::Rust, "OrderBook::match_limit").expect("resolves");
    assert!(span.code.contains("pub fn match_limit"));
    assert!(span.code.contains("qty.min"));
}

#[test]
fn resolves_trait_impl_method() {
    let span = extract(&fixture(), Language::Rust, "OrderBook::clone").expect("resolves");
    assert!(span.code.contains("fn clone(&self)"));
}

#[test]
fn resolves_through_nested_module() {
    let span = extract(&fixture(), Language::Rust, "engine::spin_up").expect("resolves");
    assert!(span.code.contains("pub fn spin_up"));
}

#[test]
fn unknown_segments_fail_with_the_path_named() {
    let fixture = fixture();
    let e = extract(&fixture, Language::Rust, "OrderBook::nonexistent")
        .expect_err("unknown segment must not resolve");
    match &e {
        SymbolError::Unresolvable { path, reason } => {
            assert_eq!(path, "OrderBook::nonexistent");
            assert!(reason.contains("nonexistent"), "reason={reason}");
            assert!(e.to_string().contains("OrderBook::nonexistent"));
        }
        other => panic!("expected Unresolvable, got {other:?}"),
    }
}

#[test]
fn bad_paths_are_rejected() {
    for bad in ["", "::", "a::::b", "1st::item", "a::b::c::d::e::f::g::h::i"] {
        match extract(&fixture(), Language::Rust, bad) {
            Err(SymbolError::BadPath { .. }) => {}
            other => panic!("path {bad:?}: expected BadPath, got {other:?}"),
        }
    }
}

#[test]
fn anchor_ids_differ_across_paths_and_positions() {
    let fixture = fixture();
    let a = extract(&fixture, Language::Rust, "process_order").expect("resolves");
    let b = extract(&fixture, Language::Rust, "OrderBook::match_limit").expect("resolves");
    assert_ne!(a.anchor_id, b.anchor_id);
    // Same symbol name in a different file at a different position: the
    // anchor depends on path alone, not on position (REQ-S3).
    let other = format!("\n\n{fixture}\n");
    let c = extract(&other, Language::Rust, "process_order").expect("resolves");
    assert_eq!(a.anchor_id, c.anchor_id);
}

#[test]
fn broken_source_is_a_parse_error_not_a_wrong_span() {
    let broken = "fn broken( { struct stru ;;\nfn ok() {}\n";
    match extract(broken, Language::Rust, "ok") {
        Err(SymbolError::Parse { reason }) => assert!(!reason.is_empty()),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn extract_matches_doctest_contract() {
    // The doctest source, reused here so the public example stays honest.
    let source = "mod engine {\n    fn process_order() -> u32 { 7 }\n}\n";
    let span = extract(source, Language::Rust, "engine::process_order").expect("resolves");
    assert_eq!(span.start_line, 2);
    assert_eq!(span.end_line, 2);
    assert!(span.code.contains("fn process_order"));
}
