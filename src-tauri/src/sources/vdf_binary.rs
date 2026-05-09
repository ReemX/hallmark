//! Hand-rolled binary VDF reader for Steam's UserGameStats_*.bin and
//! UserGameStatsSchema_*.bin files.
//!
//! # Type tags handled
//!
//! Per RESEARCH.md and empirical inspection on the dev machine:
//! - 0x00 Object  — recursive sub-object; key is C-string, body is more tag-prefixed entries until 0x08
//! - 0x01 String  — key + value are NUL-terminated UTF-8 C-strings
//! - 0x02 Int32   — key is C-string, value is 4 LE bytes
//! - 0x03 Float   — key is C-string, value is 4 LE bytes (IEEE 754)
//! - 0x04 Pointer — 4 LE bytes; rare in achievement files but documented; we skip it
//! - 0x05 WString — UTF-16LE NUL-terminated; rare in achievement files; we skip it
//! - 0x06 Color   — 4 RGBA bytes; rare; we skip it
//! - 0x07 UInt64  — key is C-string, value is 8 LE bytes
//! - 0x08 ObjectEnd — closes the current Object scope
//!
//! Unknown tags log a structured warning and abort parsing of that branch with an Err.
//!
//! # Recursion bound
//!
//! Achievement files in the wild have depth ≤ 4. To defend against pathological
//! input, recursion is bounded at depth 16. Beyond that, parser returns Err.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

const MAX_RECURSION_DEPTH: usize = 16;

#[derive(Debug, Clone)]
pub enum Value {
    Object(HashMap<String, Value>),
    String(String),
    Int32(i32),
    Float(f32),
    UInt64(u64),
}

impl Value {
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        if let Value::Object(m) = self { Some(m) } else { None }
    }
    pub fn as_int32(&self) -> Option<i32> {
        if let Value::Int32(v) = self { Some(*v) } else { None }
    }
    pub fn as_string(&self) -> Option<&str> {
        if let Value::String(s) = self { Some(s.as_str()) } else { None }
    }
}

#[derive(Debug, Clone)]
pub struct Vdf {
    pub root_key: String,
    pub root: Value,
}

pub fn parse_binary_vdf(bytes: &[u8]) -> anyhow::Result<Vdf> {
    let mut cursor = Cursor::new(bytes);
    // Per RESEARCH.md (empirical inspection of 166 real UserGameStats files on the
    // dev machine), Steam's UserGameStats_*.bin and UserGameStatsSchema_*.bin files
    // begin with the leading 0x00 Object tag followed by the C-string root key
    // ("cache" for state files, the appid for schema files). RESEARCH.md does NOT
    // document any "older variant" that omits the leading 0x00, so we do not probe;
    // we consume the leading 0x00 unconditionally. If a malformed file lacks it,
    // `read_cstr` returns an error which the caller logs as warn + skips.
    let mut tag = [0u8; 1];
    cursor.read_exact(&mut tag)?;
    if tag[0] != 0x00 {
        anyhow::bail!("vdf_binary: expected leading 0x00 root Object tag, got 0x{:02x}", tag[0]);
    }
    let root_key = read_cstr(&mut cursor)?;
    let root = read_object_body(&mut cursor, 0)?;
    Ok(Vdf { root_key, root })
}

fn read_object_body<R: Read>(r: &mut R, depth: usize) -> anyhow::Result<Value> {
    if depth >= MAX_RECURSION_DEPTH {
        anyhow::bail!("vdf_binary: max recursion depth ({}) exceeded", MAX_RECURSION_DEPTH);
    }
    let mut map = HashMap::new();
    loop {
        let mut tag = [0u8; 1];
        let n = r.read(&mut tag)?;
        if n == 0 {
            // EOF at top of the loop is benign — end of file as end of root object.
            break;
        }
        match tag[0] {
            0x00 => {
                let key = read_cstr(r)?;
                let val = read_object_body(r, depth + 1)?;
                map.insert(key, val);
            }
            0x01 => {
                let key = read_cstr(r)?;
                let val = read_cstr(r)?;
                map.insert(key, Value::String(val));
            }
            0x02 => {
                let key = read_cstr(r)?;
                let v = r.read_i32::<LittleEndian>()?;
                map.insert(key, Value::Int32(v));
            }
            0x03 => {
                let key = read_cstr(r)?;
                let v = r.read_f32::<LittleEndian>()?;
                map.insert(key, Value::Float(v));
            }
            0x04 => {
                // Pointer (4 bytes). Skip — not meaningful for achievements.
                let _key = read_cstr(r)?;
                let mut buf = [0u8; 4];
                r.read_exact(&mut buf)?;
            }
            0x05 => {
                // WString (UTF-16LE NUL-terminated). Skip — rare in achievement files.
                let _key = read_cstr(r)?;
                read_wstr_skip(r)?;
            }
            0x06 => {
                // Color (4 bytes). Skip.
                let _key = read_cstr(r)?;
                let mut buf = [0u8; 4];
                r.read_exact(&mut buf)?;
            }
            0x07 => {
                let key = read_cstr(r)?;
                let v = r.read_u64::<LittleEndian>()?;
                map.insert(key, Value::UInt64(v));
            }
            0x08 => return Ok(Value::Object(map)),
            other => {
                tracing::warn!(tag = format!("0x{:02x}", other), "vdf_binary: unknown type tag; aborting parse of this branch");
                anyhow::bail!("vdf_binary: unknown type tag 0x{:02x}", other);
            }
        }
    }
    Ok(Value::Object(map))
}

fn read_cstr<R: Read>(r: &mut R) -> anyhow::Result<String> {
    let mut buf = Vec::with_capacity(32);
    let mut byte = [0u8; 1];
    loop {
        r.read_exact(&mut byte)?;
        if byte[0] == 0 { break; }
        buf.push(byte[0]);
        if buf.len() > 1024 {
            anyhow::bail!("vdf_binary: C-string exceeds 1024 bytes (likely corrupt)");
        }
    }
    Ok(String::from_utf8(buf)?)
}

fn read_wstr_skip<R: Read>(r: &mut R) -> anyhow::Result<()> {
    let mut pair = [0u8; 2];
    loop {
        r.read_exact(&mut pair)?;
        if pair == [0, 0] { return Ok(()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests");
        p.push("fixtures");
        p.push("steam_legit");
        p
    }

    #[test]
    fn parse_real_user_game_stats_fixture() {
        // Plan 03-01 step 1a copied at least one real UserGameStats_*.bin here.
        let dir = fixtures_dir();
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("fixtures dir must exist after Plan 03-01 step 1a")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("UserGameStats_")
                    && e.path().extension().map(|s| s == "bin").unwrap_or(false)
            })
            .collect();
        assert!(!entries.is_empty(), "expected at least one UserGameStats fixture in {:?}", dir);

        for entry in &entries {
            let bytes = std::fs::read(entry.path()).expect("fixture readable");
            let result = parse_binary_vdf(&bytes);
            assert!(
                result.is_ok(),
                "parse failed for fixture {:?}: {:?}",
                entry.path(),
                result.err()
            );
            let vdf = result.unwrap();
            // Real UserGameStats files have root_key "cache" or similar (per RESEARCH.md).
            // We accept any non-empty root_key here; the SteamLegit adapter's tests assert structural correctness.
            assert!(!vdf.root_key.is_empty(), "root_key should be non-empty for {:?}", entry.path());
            assert!(matches!(vdf.root, Value::Object(_)), "root must be Object for {:?}", entry.path());
            // W-2 acceptance: prove parsed tree contains a real top-level key (not just shape).
            // State files have root_key "cache"; the root Object contains either "crc" (Int32),
            // a numeric stat_slot child, or both. Assert at least one of these expected keys.
            let root_obj = vdf.root.as_object().unwrap();
            let has_known_key = root_obj.contains_key("crc")
                || root_obj.keys().any(|k| k.parse::<u32>().is_ok())
                || root_obj.contains_key("PendingChanges");
            assert!(has_known_key, "expected real key (crc/numeric stat_slot/PendingChanges) in fixture {:?}; got keys: {:?}", entry.path(), root_obj.keys().collect::<Vec<_>>());
        }
    }

    #[test]
    fn unknown_type_tag_returns_err() {
        // synthesise: NUL-terminated key + unknown tag 0xFF
        let bytes = [0x00, b'k', b'e', b'y', 0x00, 0xFF, 0x00];
        let result = parse_binary_vdf(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn non_zero_leading_byte_returns_err() {
        // Per W-2 acceptance: input not beginning with the 0x00 root Object tag
        // must return Err (RESEARCH.md does not document any "older variant").
        let bytes = [0xFF];
        assert!(parse_binary_vdf(&bytes).is_err());
        let bytes2 = [0x42, b'k', 0x00];
        assert!(parse_binary_vdf(&bytes2).is_err());
    }

    #[test]
    fn recursion_depth_bounded() {
        // Synthesise deeply-nested object: 20 levels of 0x00 key 0x00 ...
        let mut bytes = Vec::new();
        for i in 0..20 {
            bytes.push(0x00);
            bytes.extend_from_slice(format!("k{i}").as_bytes());
            bytes.push(0x00);
        }
        let result = parse_binary_vdf(&bytes);
        assert!(result.is_err(), "expected depth-bound failure, got Ok");
    }

    #[test]
    fn cstr_overlong_returns_err() {
        // 1500-byte C-string without terminator
        let mut bytes = vec![0x00];
        bytes.extend(std::iter::repeat(b'x').take(1500));
        let result = parse_binary_vdf(&bytes);
        assert!(result.is_err());
    }
}
