const REDACTED: &str = "[redacted]";

pub fn redact_review_text(value: &str) -> String {
    let mut redacted = String::with_capacity(value.len());
    let mut redact_next_tokens = 0_u8;

    for segment in value.split_inclusive(char::is_whitespace) {
        let (token, whitespace) = split_trailing_whitespace(segment);
        redacted.push_str(redact_token(token, &mut redact_next_tokens).as_str());
        redacted.push_str(whitespace);
    }

    redacted
}

fn redact_token(token: &str, redact_next_tokens: &mut u8) -> String {
    if token.is_empty() {
        return String::new();
    }

    if *redact_next_tokens > 0 {
        *redact_next_tokens = (*redact_next_tokens).saturating_sub(1);
        return REDACTED.to_owned();
    }

    if let Some((key, separator, value)) = split_key_value(token) {
        if key_is_secret(key) {
            if value.is_empty() {
                *redact_next_tokens = 2;
                return format!("{key}{separator}");
            }
            return format!("{key}{separator}{REDACTED}");
        }
    }

    if token_has_secret_prefix(token) {
        return REDACTED.to_owned();
    }

    token.to_owned()
}

fn split_trailing_whitespace(segment: &str) -> (&str, &str) {
    let token_len = segment.trim_end_matches(char::is_whitespace).len();
    segment.split_at(token_len)
}

fn split_key_value(token: &str) -> Option<(&str, char, &str)> {
    let equals = token.find('=');
    let colon = token.find(':');
    let index = match (equals, colon) {
        (Some(equals), Some(colon)) => equals.min(colon),
        (Some(equals), None) => equals,
        (None, Some(colon)) => colon,
        (None, None) => return None,
    };
    if index == 0 {
        return None;
    }
    let (key, rest) = token.split_at(index);
    let separator = rest.chars().next()?;
    let value = &rest[separator.len_utf8()..];
    Some((key, separator, value))
}

fn key_is_secret(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .map(|character| character.to_ascii_lowercase())
        .collect::<String>();
    normalized == "authorization"
        || normalized.contains("password")
        || normalized.contains("passwd")
        || normalized.contains("secret")
        || normalized.contains("token")
        || normalized.contains("apikey")
        || normalized.contains("api_key")
        || normalized.contains("accesskey")
        || normalized.contains("access_key")
        || normalized.contains("privatekey")
        || normalized.contains("private_key")
}

fn token_has_secret_prefix(token: &str) -> bool {
    let trimmed = token.trim_matches(|character: char| {
        matches!(
            character,
            '"' | '\'' | '`' | ',' | '.' | ';' | ')' | '(' | '[' | ']' | '{' | '}'
        )
    });
    let lowered = trimmed.to_ascii_lowercase();
    lowered.starts_with("ghp_")
        || lowered.starts_with("github_pat_")
        || lowered.starts_with("glpat-")
        || lowered.starts_with("glpat_")
        || lowered.starts_with("sk-") && lowered.len() >= 12
}
