//! Span model and position-independent anchor IDs.

/// The exact source span of one resolved symbol.
///
/// `start_line`/`end_line` are 1-based and describe where the symbol lives
/// in the given source; they are deliberately NOT part of the anchor ID,
/// which is stable across any line shifts above the symbol (REQ-S1/S3).
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// Position-independent anchor ID: `dsym-<lang>-<fnv1a64 hex>` over
    /// `"<lang>\n<symbol path>"`. Stable across edits elsewhere in the
    /// file and across line shifts.
    pub anchor_id: String,
    /// Language label (`"rust"` for [`Language::Rust`](crate::Language::Rust)).
    pub language: String,
    /// 1-based first line of the span in the given source.
    pub start_line: usize,
    /// 1-based last line of the span in the given source.
    pub end_line: usize,
    /// The exact source text of the span (byte-exact slice; no
    /// re-formatting, no trimming).
    pub code: String,
}

/// FNV-1a 64-bit over `lang \n path`; rendered as 16 hex characters.
///
/// Hand-rolled rather than `DefaultHasher` because the SipHash keys are
/// not stable across Rust versions and the anchor ID must be stable.
pub(crate) fn anchor_id(lang_label: &str, symbol_path: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for byte in lang_label.as_bytes() {
        mix(*byte);
    }
    mix(b'\n');
    for byte in symbol_path.as_bytes() {
        mix(*byte);
    }
    format!("dsym-{lang_label}-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::anchor_id;

    #[test]
    fn anchor_id_is_deterministic_and_position_free() {
        let a = anchor_id("rust", "OrderBook::match_limit");
        let b = anchor_id("rust", "OrderBook::match_limit");
        assert_eq!(a, b);
        assert!(a.starts_with("dsym-rust-"));
        assert_eq!(a.len(), "dsym-rust-".len() + 16);
        assert_ne!(a, anchor_id("rust", "OrderBook::match"));
        assert_ne!(a, anchor_id("python", "OrderBook::match_limit"));
    }
}
