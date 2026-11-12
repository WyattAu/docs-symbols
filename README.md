# docs-symbols

Symbol-anchored source extraction for documentation sites: resolve a
symbol path (`engine::OrderBook::match_limit`) against real source text
via tree-sitter and return the exact byte span plus a position-independent
anchor ID derived from the path itself. Unknown paths fail with typed
errors naming the offending segment. Part of the docs-core extension
program (`docs-core/SPEC.md` REQ-S).

## Module map

| Module | Contents |
|---|---|
| `extract` | parse + resolve one symbol path against source text |
| `Span` | `anchor_id`, language label, 1-based lines, byte-exact code |
| `Language` | grammars compiled into the crate (Rust in 0.1) |
| `SymbolError` | parse / unresolvable (with path + segment) / bad path / grammar |

## Guarantees (0.1, honest)

- **Line-shift invariant** (REQ-S1): resolution walks the AST by name in
  document order, so shifting lines above the symbol changes the reported
  line numbers but not the extracted code or the anchor ID. Oracle: the
  line-shift fixture test.
- **Typed failures** (REQ-S2): unknown segments carry the full path and
  the failing segment; malformed paths (empty or doubled `::` segments,
  > 8 segments) are `BadPath`; sources with any ERROR/MISSING node are
  rejected as `Parse` rather than silently yielding a wrong span.
- **Stable anchors** (REQ-S3): `dsym-<lang>-<fnv1a64 hex>` over
  `"<lang>\n<path>"`; hand-rolled FNV because SipHash keys are not
  stable across Rust versions.
- **Deterministic first-match**: document order is the tiebreak; a
  non-final segment prefers impl scopes over same-named items, and every
  impl of a type (inherent or trait) is tried in order, so
  `OrderBook::clone` resolves even though `impl Clone for OrderBook`
  follows `impl OrderBook`.

## Scope (0.1, honest)

Rust only. No generic-qualified paths (`Vec::<u8>::new`), no trait-name
segments, no doc-comment inclusion in spans (the span is the item node).
The C grammar builds for native targets; the crate is native-only by
design because extraction is a build-time concern (the browser client
never extracts symbols). Each absence is a later increment, not silent.

## Where the truth lives

| Concern | Truth | Path |
|---|---|---|
| Symbol resolution + spans | source of record | `src/resolve.rs`, `src/span.rs` |
| Typed failures | source of record | `src/error.rs` |
| REQ-S anchors (symbol-anchored includes) | SPEC | `docs-core/SPEC.md` |
| Layer declaration + tree-sitter pin (0.25: one native lib per graph) | manifest metadata | `Cargo.toml` |

## Layer

L0 - leaf. Zero estate-internal dependencies; the external floor is
`tree-sitter` and `tree-sitter-rust`.

## Tests

`cargo test` under clippy `-D warnings` with the estate decoder-pattern
denies (`unwrap`/`expect`/`panic`/`indexing_slicing` lifted in test
targets only): extraction at three depths (top-level, mod, inherent +
trait impl), line-shift invariance, anchor stability and divergence,
typed failures for unknown segments and malformed paths, parse-error
rejection, and the doctest contract.

Licensed under MIT OR Apache-2.0.
