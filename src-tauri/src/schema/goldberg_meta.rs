//! Parse Goldberg's `achievements.json` for METADATA ONLY (display_name,
//! description, icon URL/path, hidden flag). The "earned" state is parsed
//! by Phase 1's `goldberg.rs` adapter; this module is read-only on metadata.
//!
//! Goldberg / gbe_fork forks have inconsistent field names: some builds use
//! `displayName`, others `display_name`; some `description`, others `desc`.
//! Phase 1's empirical-goldberg-schema-NOTES.md confirms the variations.
//! This parser tries each variant in order via `serde_json::Value`.

use serde_json::Value;

/// Metadata for one achievement, sourced from Goldberg's achievements.json.
/// All fields are Option because partial data is acceptable — popup falls
/// back to api_name when display_name is None (D-26).
#[derive(Debug, Clone, PartialEq)]
pub struct GoldbergAchievementMeta {
    pub api_name: String,               // canonical "name" field — REQUIRED for PK
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,       // URL or filename relative to Goldberg dir
    pub hidden: bool,
}

/// Parse a Goldberg achievements.json text blob and return one entry per
/// achievement. Empty list on parse failure — caller logs + continues.
/// Tolerates two top-level shapes: array `[{...}, {...}]` (most common)
/// and object `{"<api_name>": {...}}` (older Goldberg builds).
pub fn parse_goldberg_metadata(json: &str) -> anyhow::Result<Vec<GoldbergAchievementMeta>> {
    let v: Value = serde_json::from_str(json)?;
    let mut out = Vec::new();

    if let Value::Array(arr) = &v {
        for item in arr {
            if let Some(meta) = extract(item, None) {
                out.push(meta);
            }
        }
    } else if let Value::Object(map) = &v {
        for (k, item) in map {
            if let Some(meta) = extract(item, Some(k.as_str())) {
                out.push(meta);
            }
        }
    } else {
        anyhow::bail!("unexpected top-level JSON shape; expected array or object");
    }
    Ok(out)
}

/// Extract one achievement's metadata, tolerating field-name variants.
/// `fallback_api_name` is the object map key when the JSON is `{"<name>": {...}}`
/// shape and the inner object lacks a `name` field.
fn extract(item: &Value, fallback_api_name: Option<&str>) -> Option<GoldbergAchievementMeta> {
    let obj = item.as_object()?;
    // api_name: try "name", else fallback_api_name.
    let api_name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| fallback_api_name.map(String::from))?;
    // display_name: try "display_name", "displayName", "displayname"
    let display_name = obj
        .get("display_name")
        .or_else(|| obj.get("displayName"))
        .or_else(|| obj.get("displayname"))
        .and_then(|v| v.as_str())
        .map(String::from);
    // description: try "description", "desc"
    let description = obj
        .get("description")
        .or_else(|| obj.get("desc"))
        .and_then(|v| v.as_str())
        .map(String::from);
    // icon: try "icon", "icon_url", "iconUrl"
    let icon_url = obj
        .get("icon")
        .or_else(|| obj.get("icon_url"))
        .or_else(|| obj.get("iconUrl"))
        .and_then(|v| v.as_str())
        .map(String::from);
    // hidden: try "hidden" as bool/int; default false
    let hidden = obj
        .get("hidden")
        .map(|v| match v {
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_i64().map(|i| i != 0).unwrap_or(false),
            Value::String(s) => s == "1" || s.eq_ignore_ascii_case("true"),
            _ => false,
        })
        .unwrap_or(false);
    Some(GoldbergAchievementMeta {
        api_name,
        display_name,
        description,
        icon_url,
        hidden,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_array_with_snake_case_fields() {
        let json = r#"[
            {"name":"ACH_A","display_name":"Got A","description":"Win once","icon":"a.jpg","hidden":false},
            {"name":"ACH_B","display_name":"Got B","description":"Win twice","icon":"b.jpg","hidden":true}
        ]"#;
        let metas = parse_goldberg_metadata(json).unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].api_name, "ACH_A");
        assert_eq!(metas[0].display_name.as_deref(), Some("Got A"));
        assert_eq!(metas[1].hidden, true);
    }

    #[test]
    fn parses_array_with_camel_case_variants() {
        let json = r#"[
            {"name":"ACH_A","displayName":"Got A","desc":"Win once","iconUrl":"https://a.png"}
        ]"#;
        let metas = parse_goldberg_metadata(json).unwrap();
        assert_eq!(metas[0].display_name.as_deref(), Some("Got A"));
        assert_eq!(metas[0].description.as_deref(), Some("Win once"));
        assert_eq!(metas[0].icon_url.as_deref(), Some("https://a.png"));
    }

    #[test]
    fn parses_object_shape_with_key_as_api_name() {
        let json = r#"{
            "ACH_X":{"display_name":"X","description":"do x"},
            "ACH_Y":{"display_name":"Y","description":"do y"}
        }"#;
        let metas = parse_goldberg_metadata(json).unwrap();
        assert_eq!(metas.len(), 2);
        let names: Vec<_> = metas.iter().map(|m| m.api_name.as_str()).collect();
        assert!(names.contains(&"ACH_X"));
        assert!(names.contains(&"ACH_Y"));
    }

    #[test]
    fn missing_optional_fields_become_none() {
        let json = r#"[{"name":"ACH_A"}]"#;
        let metas = parse_goldberg_metadata(json).unwrap();
        assert_eq!(metas[0].display_name, None);
        assert_eq!(metas[0].description, None);
        assert_eq!(metas[0].icon_url, None);
        assert_eq!(metas[0].hidden, false);
    }

    #[test]
    fn malformed_json_returns_err() {
        assert!(parse_goldberg_metadata("not json").is_err());
    }

    #[test]
    fn hidden_int_variant_parses() {
        let json = r#"[{"name":"A","hidden":1},{"name":"B","hidden":0}]"#;
        let metas = parse_goldberg_metadata(json).unwrap();
        assert!(metas[0].hidden);
        assert!(!metas[1].hidden);
    }
}
