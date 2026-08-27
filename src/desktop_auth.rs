// Read a Claude Desktop profile's OAuth token.
//
// The desktop app stores credentials the way every Chromium app does on
// macOS: a random password in the Keychain under service "Claude Safe
// Storage", stretched with PBKDF2-HMAC-SHA1 (salt "saltysalt", 1003
// iterations, 16-byte key) into an AES-128-CBC key with a fixed IV of 16
// spaces. The ciphertext sits base64-encoded in the profile's config.json
// under `oauth:tokenCacheV2`, prefixed with the version marker `v10`.
//
// This is Chromium's scheme, not something Anthropic designed, but the
// contents are theirs and undocumented: a map from
// `<accountUuid>:<orgUuid>:<audience>:<scopes>` to a token record. It can
// change; every failure here is soft, and the caller falls back to the
// account uuid sitting in plaintext in the same config.json.
//
// Reading the Keychain entry prompts for the login keychain password the
// first time — the entry's ACL only lists the app itself. That is the user's
// call to make, so callers say what is about to happen before asking.

use std::path::Path;
use std::process::Command;

use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use hmac::Hmac;
use sha1::Sha1;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

const KEYCHAIN_SERVICE: &str = "Claude Safe Storage";
const KEYCHAIN_ACCOUNT: &str = "Claude Key";
const SALT: &[u8] = b"saltysalt";
const ITERATIONS: u32 = 1003;
const PREFIX: &[u8] = b"v10";
/// Chromium's macOS IV: sixteen spaces, not a random one.
const IV: [u8; 16] = [b' '; 16];

/// One entry of the profile's token cache.
#[derive(Debug, PartialEq)]
pub struct TokenEntry {
    pub token: String,
    /// First field of the cache key — the account this token belongs to.
    pub account_uuid: Option<String>,
    /// Whether the key's scope list mentions `user:profile`, which is what
    /// the identity endpoint wants.
    pub has_profile_scope: bool,
    /// Unix seconds, from `expiresAt` (milliseconds in the file).
    pub expires_at: Option<i64>,
}

/// The account uuid the app last signed in as, straight out of `config.json`
/// with no decryption at all. Weaker than the API answer — it is a uuid, not
/// an email, and it survives a sign-out — but it costs nothing and it is what
/// remains when Keychain access is refused.
pub fn last_known_account_uuid(profile: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(profile.join("config.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    json.get("lastKnownAccountUuid")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(String::from)
}

/// The Keychain password behind `Claude Safe Storage`. **Prompts** the first
/// time — see the module comment.
pub fn safe_storage_password() -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = Command::new("security")
        .args(["find-generic-password", "-s", KEYCHAIN_SERVICE])
        .args(["-a", KEYCHAIN_ACCOUNT, "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw.is_empty() { None } else { Some(raw) }
}

pub fn derive_key(password: &str) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2::<Hmac<Sha1>>(password.as_bytes(), SALT, ITERATIONS, &mut key)
        .expect("pbkdf2 into a fixed 16-byte buffer cannot fail");
    key
}

/// Decrypt a `v10`-prefixed blob. `None` for anything that isn't one, or that
/// doesn't decrypt to valid UTF-8 — a wrong key produces bytes, not an error,
/// so the UTF-8 check is what actually catches it.
pub fn decrypt(blob: &[u8], key: &[u8; 16]) -> Option<String> {
    let body = blob.strip_prefix(PREFIX)?;
    if body.is_empty() || body.len() % 16 != 0 {
        return None;
    }
    let mut buf = body.to_vec();
    let plain = Aes128CbcDec::new(key.into(), &IV.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .ok()?;
    String::from_utf8(plain.to_vec()).ok()
}

/// The `oauth:tokenCacheV2` blob, base64-decoded. Falls back to the pre-V2
/// key, which survives the migration as an empty placeholder and so is only
/// used when it actually holds something.
pub fn token_blob(profile: &Path) -> Option<Vec<u8>> {
    let raw = std::fs::read_to_string(profile.join("config.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    ["oauth:tokenCacheV2", "oauth:tokenCache"]
        .iter()
        .find_map(|key| {
            json.get(*key)
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
        })
        .and_then(base64_decode)
}

/// Every usable entry of a decrypted token cache, newest-usable-first is left
/// to `pick`. Entries without a token string are dropped.
pub fn parse_cache(plain: &str) -> Vec<TokenEntry> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(plain)
    else {
        return Vec::new();
    };
    map.iter()
        .filter_map(|(key, value)| {
            let token = value.get("token")?.as_str()?;
            if token.is_empty() {
                return None;
            }
            Some(TokenEntry {
                token: token.to_string(),
                account_uuid: key
                    .split(':')
                    .next()
                    .filter(|s| !s.is_empty())
                    .map(String::from),
                has_profile_scope: key.contains("user:profile"),
                // Stored in milliseconds; seconds is what the rest of the
                // codebase speaks.
                expires_at: value
                    .get("expiresAt")
                    .and_then(|v| v.as_i64())
                    .map(|ms| ms / 1000),
            })
        })
        .collect()
}

/// The entry to authenticate with: unexpired first, and among those the one
/// whose scopes include `user:profile`, since that is the endpoint we ask
/// first. An entry with no `expiresAt` is treated as usable rather than
/// discarded — the field is theirs to remove.
pub fn pick(entries: &[TokenEntry], now: i64) -> Option<&TokenEntry> {
    let fresh = |e: &&TokenEntry| e.expires_at.is_none_or(|exp| exp > now);
    entries
        .iter()
        .filter(fresh)
        .find(|e| e.has_profile_scope)
        .or_else(|| entries.iter().find(fresh))
}

pub enum TokenResult {
    Ok(String),
    /// No credential in the profile at all.
    NotSignedIn,
    /// The Keychain entry could not be read — most often because the user
    /// declined the prompt.
    NoKeychain,
    /// Decryption or parsing failed: the app changed the format, or the
    /// Keychain password is not the one this blob was encrypted with.
    Unreadable,
    /// Credentials are there but past their expiry, and refreshing them is
    /// the app's job, not ours.
    Expired,
}

/// The profile's live access token. Every step can fail softly, and each
/// failure is a different thing to tell the user, hence the enum.
pub fn profile_token(profile: &Path) -> TokenResult {
    let Some(blob) = token_blob(profile) else {
        return TokenResult::NotSignedIn;
    };
    let Some(password) = safe_storage_password() else {
        return TokenResult::NoKeychain;
    };
    let Some(plain) = decrypt(&blob, &derive_key(&password)) else {
        return TokenResult::Unreadable;
    };
    let entries = parse_cache(&plain);
    if entries.is_empty() {
        return TokenResult::Unreadable;
    }
    match pick(&entries, now_secs()) {
        Some(entry) => TokenResult::Ok(entry.token.clone()),
        None => TokenResult::Expired,
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Standard base64 with padding, decode only. Hand-rolled rather than adding
/// a dependency for thirty lines; whitespace is ignored, anything else
/// invalid is `None`.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in input.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(key: [u8; 16]) -> String {
        key.iter().map(|b| format!("{:02x}", b)).collect()
    }

    // Pinned against an independent implementation, so a wrong digest, salt
    // or iteration count fails here rather than as an unexplained 401:
    //   python3 -c "import hashlib,binascii; print(binascii.hexlify(
    //       hashlib.pbkdf2_hmac('sha1', b'peanuts', b'saltysalt', 1003, 16)))"
    #[test]
    fn key_derivation_matches_an_independent_pbkdf2() {
        assert_eq!(
            derive_key("peanuts"),
            derive_key("peanuts"),
            "deterministic"
        );
        assert_eq!(
            hex(derive_key("peanuts")),
            "d9a09d499b4e1b7461f28e67972c6dbd"
        );
        assert_eq!(hex(derive_key("")), "1cbee826d6938327ae9043f63bfc26d7");
    }

    #[test]
    fn decrypt_round_trips_what_the_same_key_encrypted() {
        use aes::cipher::{BlockEncryptMut, KeyIvInit};
        let key = derive_key("peanuts");
        let plain = br#"{"a:b:c:user:profile":{"token":"t"}}"#;
        let mut buf = vec![0u8; plain.len() + 16];
        buf[..plain.len()].copy_from_slice(plain);
        let ct = cbc::Encryptor::<aes::Aes128>::new(&key.into(), &IV.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buf, plain.len())
            .unwrap()
            .to_vec();

        let mut blob = PREFIX.to_vec();
        blob.extend_from_slice(&ct);
        assert_eq!(
            decrypt(&blob, &key).as_deref(),
            Some(r#"{"a:b:c:user:profile":{"token":"t"}}"#)
        );
    }

    #[test]
    fn decrypt_rejects_what_it_cannot_read() {
        let key = derive_key("peanuts");
        assert_eq!(decrypt(b"", &key), None, "empty");
        assert_eq!(decrypt(b"v11abcdefghijklmnop", &key), None, "wrong version");
        assert_eq!(decrypt(b"v10short", &key), None, "not a whole block");
        // A wrong key decrypts to bytes, not an error — the UTF-8 check is
        // the only thing standing between that and a garbage "token".
        let mut blob = PREFIX.to_vec();
        blob.extend_from_slice(&[0u8; 32]);
        assert_eq!(decrypt(&blob, &key), None, "garbage ciphertext");
    }

    const CACHE: &str = r#"{
        "acc-1:org:https://api.anthropic.com:user:inference user:sessions": {
            "token": "inference-token", "expiresAt": 4000000
        },
        "acc-1:org:https://api.anthropic.com:user:profile": {
            "token": "profile-token", "expiresAt": 4000000
        },
        "acc-1:org:https://api.anthropic.com:user:nothing": {
            "token": "", "expiresAt": 4000000
        }
    }"#;

    #[test]
    fn parsing_reads_the_uuid_out_of_the_key_and_drops_empty_tokens() {
        let entries = parse_cache(CACHE);
        assert_eq!(entries.len(), 2, "the empty-token entry is dropped");
        assert!(
            entries
                .iter()
                .all(|e| e.account_uuid.as_deref() == Some("acc-1"))
        );
        // Milliseconds in the file, seconds everywhere else.
        assert!(entries.iter().all(|e| e.expires_at == Some(4000)));
    }

    #[test]
    fn parsing_survives_anything_that_is_not_the_expected_shape() {
        assert!(parse_cache("not json").is_empty());
        assert!(parse_cache("[]").is_empty());
        assert!(parse_cache(r#"{"k":{"no-token":1}}"#).is_empty());
    }

    #[test]
    fn the_profile_scoped_token_is_preferred() {
        let entries = parse_cache(CACHE);
        assert_eq!(pick(&entries, 0).unwrap().token, "profile-token");
    }

    #[test]
    fn expired_entries_are_skipped() {
        let entries = parse_cache(CACHE);
        // Past every expiry: nothing is usable, rather than a stale token
        // that would only produce a confusing 401.
        assert!(pick(&entries, 5000).is_none());
    }

    #[test]
    fn a_non_profile_token_is_used_when_it_is_all_there_is() {
        let entries = parse_cache(
            r#"{"acc:org:aud:user:inference": {"token": "only-one", "expiresAt": 4000000}}"#,
        );
        assert_eq!(pick(&entries, 0).unwrap().token, "only-one");
    }

    #[test]
    fn an_entry_without_an_expiry_is_still_usable() {
        let entries = parse_cache(r#"{"acc:org:aud:user:profile": {"token": "t"}}"#);
        assert_eq!(entries[0].expires_at, None);
        assert_eq!(pick(&entries, i64::MAX).unwrap().token, "t");
    }

    #[test]
    fn picking_from_nothing_is_nothing() {
        assert!(pick(&[], 0).is_none());
    }

    #[test]
    fn base64_decodes_padded_and_unpadded_input() {
        assert_eq!(base64_decode("djEw").unwrap(), b"v10");
        assert_eq!(base64_decode("YQ==").unwrap(), b"a");
        assert_eq!(base64_decode("YWI=").unwrap(), b"ab");
        assert_eq!(base64_decode("YWJj").unwrap(), b"abc");
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("dj\nEw").unwrap(), b"v10", "newlines ignored");
    }

    #[test]
    fn base64_rejects_characters_it_does_not_know() {
        assert_eq!(base64_decode("dj!w"), None);
    }
}
