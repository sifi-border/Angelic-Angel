//! Formats decrypted X push payloads as Discord webhook messages.
//!
//! Discord rejects arbitrary JSON: a webhook body needs `content` or `embeds`.

use serde_json::{Value, json};

/// Discord's limit for `content`, in characters.
const MAX_CONTENT_CHARS: usize = 2000;

const X_BASE_URL: &str = "https://x.com";

/// Builds the Discord webhook body for a notification payload.
///
/// Mentions are disabled so tweet text containing `@everyone` cannot ping the channel.
pub fn message(payload: &Value) -> Value {
    json!({
        "content": content(payload),
        "allowed_mentions": { "parse": [] },
    })
}

/// `**title**`, body and tweet URL on separate lines, whichever of them are present.
///
/// The body is the part that gets shortened, so the link always survives.
/// Falls back to the payload itself when none of the known fields exist.
fn content(payload: &Value) -> String {
    let title = field(payload, "title").map(|t| format!("**{}**", escape_markdown(t)));
    let body = field(payload, "body").map(escape_markdown);
    let url = field(payload, "uri").and_then(tweet_url);

    if title.is_none() && body.is_none() && url.is_none() {
        return fallback(payload);
    }

    let fixed: usize = [&title, &url]
        .into_iter()
        .flatten()
        .map(|part| part.chars().count() + 1)
        .sum();
    let body = body.map(|b| truncate(&b, MAX_CONTENT_CHARS.saturating_sub(fixed)));

    let parts: Vec<String> = [title, body, url].into_iter().flatten().collect();
    truncate(&parts.join("\n"), MAX_CONTENT_CHARS)
}

/// Escapes Discord markdown so text from X is shown literally.
///
/// Stops tweet text from rendering a masked link (`[text](url)`) that hides its
/// real destination, or from breaking the bold title. In words that start with
/// `http://` or `https://` only brackets are escaped, so links stay clickable.
fn escape_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for word in text.split_inclusive(char::is_whitespace) {
        let is_url = word.starts_with("http://") || word.starts_with("https://");
        for c in word.chars() {
            let special = matches!(c, '[' | ']')
                || (!is_url && matches!(c, '\\' | '*' | '_' | '~' | '`' | '|' | '>' | '#'));
            if special {
                escaped.push('\\');
            }
            escaped.push(c);
        }
    }
    escaped
}

/// Looks up a non-empty string under `data` first, then at the top level.
fn field<'a>(payload: &'a Value, name: &str) -> Option<&'a str> {
    [payload.get("data"), Some(payload)]
        .into_iter()
        .flatten()
        .filter_map(|obj| obj.get(name)?.as_str())
        .find(|s| !s.trim().is_empty())
}

/// Turns the payload's `uri` (a path such as `/user/status/1`) into a full URL.
fn tweet_url(uri: &str) -> Option<String> {
    if uri.starts_with("https://") {
        Some(uri.to_string())
    } else if uri.starts_with('/') {
        Some(format!("{}{}", X_BASE_URL, uri))
    } else {
        None
    }
}

/// Shows an unrecognized payload as JSON, without the push endpoint URLs.
fn fallback(payload: &Value) -> String {
    const FENCE_CHARS: usize = "```json\n\n```".len();

    let mut payload = payload.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("registration_ids");
    }
    let text = payload.to_string().replace("```", "'''");
    format!("```json\n{}\n```", truncate(&text, MAX_CONTENT_CHARS - FENCE_CHARS))
}

/// Shortens `text` to at most `max` characters, marking the cut with an ellipsis.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_of(message: &Value) -> &str {
        message["content"].as_str().unwrap()
    }

    #[test]
    fn formats_title_body_and_link() {
        let payload = json!({
            "registration_ids": ["https://updates.push.services.mozilla.com/wpush/v2/secret"],
            "title": "Alice",
            "body": "hello",
            "data": { "type": "tweet", "uri": "/alice/status/123" }
        });
        let message = message(&payload);

        assert_eq!(content_of(&message), "**Alice**\nhello\nhttps://x.com/alice/status/123");
        assert_eq!(message["allowed_mentions"]["parse"], json!([]));
    }

    #[test]
    fn prefers_fields_under_data() {
        let payload = json!({ "title": "outer", "data": { "title": "inner", "body": "b" } });
        assert_eq!(content_of(&message(&payload)), "**inner**\nb");
    }

    #[test]
    fn long_body_is_cut_but_link_is_kept() {
        let payload = json!({
            "title": "Alice",
            "body": "あ".repeat(5000),
            "data": { "uri": "/alice/status/123" }
        });
        let message = message(&payload);
        let content = content_of(&message);

        assert_eq!(content.chars().count(), MAX_CONTENT_CHARS);
        assert!(content.ends_with("…\nhttps://x.com/alice/status/123"));
    }

    #[test]
    fn unknown_payload_falls_back_to_json_without_endpoint() {
        let payload = json!({ "registration_ids": ["https://secret"], "foo": "bar" });
        let message = message(&payload);
        let content = content_of(&message);

        assert_eq!(content, "```json\n{\"foo\":\"bar\"}\n```");
        assert!(!content.contains("secret"));
    }

    #[test]
    fn huge_unknown_payload_fits_the_limit() {
        let payload = json!({ "raw": "x".repeat(5000) });
        let message = message(&payload);
        let content = content_of(&message);

        assert_eq!(content.chars().count(), MAX_CONTENT_CHARS);
        assert!(content.ends_with("\n```"));
    }

    #[test]
    fn escapes_markdown_but_keeps_links() {
        let payload = json!({
            "title": "**M_a**",
            "body": "[click](https://evil.example) see https://t.co/a_b https://x[y](https://evil)",
            "data": { "uri": "/m/status/1" }
        });

        assert_eq!(
            content_of(&message(&payload)),
            "**\\*\\*M\\_a\\*\\***\n\\[click\\](https://evil.example) see https://t.co/a_b https://x\\[y\\](https://evil)\nhttps://x.com/m/status/1"
        );
    }

    #[test]
    fn ignores_uri_that_is_not_a_path_or_https_url() {
        let payload = json!({ "body": "b", "data": { "uri": "javascript:alert(1)" } });
        assert_eq!(content_of(&message(&payload)), "b");
    }
}
