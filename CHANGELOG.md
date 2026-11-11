# Changelog

All notable changes to this project are documented in this file.

## [0.1.0] - 2026-11-11

### Added

- `extract`: tree-sitter (Rust grammar) resolution of `a::b::c` symbol
  paths through modules and impls, returning the exact byte span.
- `Span` with position-independent anchor IDs (`dsym-<lang>-<fnv1a64>`
  over lang + path; hand-rolled FNV for cross-version stability).
- Typed errors: `Parse` (any ERROR/MISSING node rejects the source),
  `Unresolvable` (full path + failing segment), `BadPath` (empty or
  doubled `::` segments, > 8 segments), `Grammar`.
- Impl resolution: inherent and trait impls of a type tried in document
  order, so `Type::clone` resolves across a trailing `impl Clone for`.
- Tests: extraction at three depths, line-shift invariance (REQ-S1
  oracle), anchor stability/divergence (REQ-S3 oracle), typed-failure
  negative tests (REQ-S2 oracle), parse rejection.
