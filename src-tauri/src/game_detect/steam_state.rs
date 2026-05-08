//! Parse Steam's loginusers.vdf to identify the currently signed-in Steam user.
//! Used as context for game-detection logging and (eventually) Phase 3 Steam-legit
//! adapter. Plan 02's authoritative game-running signal remains process scanning.
//!
//! Mirrors the keyvalues-parser pattern from paths.rs::parse_libraryfolders_text.

use std::path::Path;

/// One row from loginusers.vdf — represents a Steam account that has signed in
/// on this machine. `most_recent` is true for the account currently active.
#[derive(Debug, Clone, PartialEq)]
pub struct LoginUser {
    pub steam_id: String,       // 64-bit ID as string (the VDF key)
    pub persona_name: String,
    pub most_recent: bool,
}

/// Parse a loginusers.vdf text blob. Returns empty Vec on parse failure
/// (logged at warn). Each top-level child of "users" is one account.
pub fn parse_loginusers(text: &str) -> Vec<LoginUser> {
    use keyvalues_parser::Vdf;
    let vdf = match Vdf::parse(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "loginusers.vdf parse failed");
            return Vec::new();
        }
    };
    if !vdf.key.eq_ignore_ascii_case("users") {
        tracing::warn!(top_key = %vdf.key, "loginusers.vdf has unexpected top-level key");
        return Vec::new();
    }
    let Some(obj) = vdf.value.get_obj() else {
        tracing::warn!("loginusers.vdf top-level value is not an object");
        return Vec::new();
    };
    let mut out = Vec::new();
    for (steam_id, values) in obj.iter() {
        for value in values.iter() {
            let Some(sub_obj) = value.get_obj() else { continue };
            let mut persona = String::new();
            let mut most_recent = false;
            if let Some(vs) = sub_obj.get("PersonaName") {
                if let Some(v) = vs.first() {
                    if let Some(s) = v.get_str() { persona = s.to_string(); }
                }
            }
            if let Some(vs) = sub_obj.get("MostRecent") {
                if let Some(v) = vs.first() {
                    if let Some(s) = v.get_str() { most_recent = s == "1"; }
                }
            }
            out.push(LoginUser {
                steam_id: steam_id.to_string(),
                persona_name: persona,
                most_recent,
            });
            break; // only first object per key
        }
    }
    out
}

/// Return the currently-signed-in Steam user from `<steam_root>/config/loginusers.vdf`.
/// None if file missing or no MostRecent user.
pub fn current_steam_user(steam_root: &Path) -> Option<LoginUser> {
    let path = steam_root.join("config").join("loginusers.vdf");
    let text = std::fs::read_to_string(&path).ok()?;
    parse_loginusers(&text).into_iter().find(|u| u.most_recent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loginusers_with_two_accounts_one_most_recent() {
        let vdf = r#"
"users"
{
    "76561198012345678"
    {
        "PersonaName"   "AlphaPlayer"
        "MostRecent"    "1"
    }
    "76561198098765432"
    {
        "PersonaName"   "BetaPlayer"
        "MostRecent"    "0"
    }
}
"#;
        let users = parse_loginusers(vdf);
        assert_eq!(users.len(), 2);
        let alpha = users.iter().find(|u| u.steam_id == "76561198012345678").unwrap();
        assert_eq!(alpha.persona_name, "AlphaPlayer");
        assert!(alpha.most_recent);
        let beta = users.iter().find(|u| u.steam_id == "76561198098765432").unwrap();
        assert!(!beta.most_recent);
    }

    #[test]
    fn malformed_vdf_returns_empty() {
        assert!(parse_loginusers("not a vdf").is_empty());
    }

    #[test]
    fn empty_vdf_returns_empty() {
        assert!(parse_loginusers(r#""users" { }"#).is_empty());
    }
}
