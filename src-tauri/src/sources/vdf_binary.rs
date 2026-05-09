//! Hand-rolled binary VDF reader for Steam's UserGameStats_*.bin and
//! UserGameStatsSchema_*.bin files (Phase 3 stub — Plan 03-00).
//!
//! Plan 03-01 populates the body. The stub provides the public types
//! and `parse_binary_vdf` signature so dependent stubs in `steam_legit.rs`
//! can reference them, but always returns an empty Vdf in the stub.
//!
//! Type tags 0x00..0x08 are documented in the research; the stub's parser
//! must be replaced with the full implementation in Plan 03-01.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Object(HashMap<String, Value>),
    String(String),
    Int32(i32),
    Float(f32),
    UInt64(u64),
}

#[derive(Debug, Clone)]
pub struct Vdf {
    pub root_key: String,
    pub root: Value,
}

/// Parse a binary VDF byte buffer. Plan 03-01 populates the full implementation.
pub fn parse_binary_vdf(_bytes: &[u8]) -> anyhow::Result<Vdf> {
    Err(anyhow::anyhow!(
        "vdf_binary::parse_binary_vdf stub — Plan 03-01 will populate"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_returns_err() {
        assert!(parse_binary_vdf(&[]).is_err());
    }
}
