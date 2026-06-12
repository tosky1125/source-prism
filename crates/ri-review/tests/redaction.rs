#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_review::redact_review_text;

#[test]
fn redacts_secret_key_values_and_bearer_tokens() {
    let text = "title token=ghp_live_secret\nAuthorization: Bearer ghp_live_secret";

    let redacted = redact_review_text(text);

    assert!(!redacted.contains("ghp_live_secret"));
    assert!(redacted.contains("token=[redacted]"));
    assert!(redacted.contains("Authorization: [redacted] [redacted]"));
}

#[test]
fn preserves_non_secret_review_text() {
    let text = "src/invoice.rs:12 line calls Money::round";

    let redacted = redact_review_text(text);

    assert_eq!(redacted, text);
}
