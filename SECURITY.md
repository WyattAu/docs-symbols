# Security Policy

## Reporting a vulnerability

Email wyatt_au@protonmail.com with the repo name in the subject. Please
include reproduction steps or a failing input; do not open a public issue
for an unfixed vulnerability.

## Content discipline

Source text is repository content, still untrusted input (the estate
decoder pattern): resolution is total. Sources with any parse error are
rejected (`SymbolError::Parse`) rather than silently yielding a wrong
span; unresolvable paths and malformed paths are typed errors naming the
failing segment. The tree-sitter C runtime is exercised as published
upstream; this crate contains no unsafe code. If you find an input that
panics, hangs, or mis-extracts a span on a clean-parsing source, that is
a security bug.

## Supported versions

Only the latest released `0.1.x` is supported with fixes.
