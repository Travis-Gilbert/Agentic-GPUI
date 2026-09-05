//! Credential shapes never reach a snapshot, an export, or a screen reader.
//!
//! Ported from `gpui-box`, `crates/gpui-kit-semantics/src/lib.rs`, at
//! `e993d0f4e2dbd4a9697db79c6428a623856444a4` (GPUI Box contributors,
//! MIT OR Apache-2.0). Unchanged apart from the module split.

/// Replaces text that looks like a credential with a fixed marker.
///
/// Applied at the probe and again on export. It is idempotent: the marker
/// itself matches nothing here.
#[must_use]
pub fn redact_sensitive_text(text: &str) -> String {
    let sensitive_prefixes = ["sk-", "xai-", "ogp_", "Bearer "];
    if sensitive_prefixes
        .iter()
        .any(|prefix| text.contains(prefix))
        || looks_like_jwt(text)
        || looks_like_secret_assignment(text)
    {
        "[REDACTED]".into()
    } else {
        text.into()
    }
}

fn looks_like_jwt(text: &str) -> bool {
    text.split('.').count() == 3 && text.len() >= 32
}

fn looks_like_secret_assignment(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["api_key=", "apikey=", "token=", "password=", "secret="]
        .iter()
        .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::redact_sensitive_text;

    #[test]
    fn exported_text_redacts_credential_shapes() {
        for secret in [
            "sk-secret-value",
            "Bearer credential",
            "api_key=hunter2",
            "eyJaaaaaaaaaa.bbbbbbbbbbbb.cccccccccccc",
        ] {
            assert_eq!(redact_sensitive_text(secret), "[REDACTED]");
        }
        assert_eq!(redact_sensitive_text("Token usage"), "Token usage");
    }
}
