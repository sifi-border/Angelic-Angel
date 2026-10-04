//! Formats decrypted X push payloads as Discord webhook messages.
//!
//! Discord rejects arbitrary JSON: a webhook body needs `content` or `embeds`.

use serde_json::{Value, json};

/// Discord's limit for `content`, in characters.
const MAX_CONTENT_CHARS: usize = 2000;

const X_BASE_URL: &str = "https://x.com";

/// Builds the Discord webhook body for a notification payload.
///
/// The message is just the tweet link; Discord's link preview shows the tweet.
/// Falls back to the payload itself when it has no link. Mentions are disabled so
/// text in that fallback (e.g. `@everyone`) cannot ping the channel.
pub fn message(payload: &Value) -> Value {
    let content = field(payload, "uri")
        .and_then(tweet_url)
        .unwrap_or_else(|| fallback(payload));
    json!({
        "content": content,
        "allowed_mentions": { "parse": [] },
    })
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
    fn posts_only_the_tweet_link() {
        let payload = json!({
            "registration_ids": ["https://updates.push.services.mozilla.com/wpush/v2/secret"],
            "title": "Alice",
            "body": "hello @everyone",
            "data": { "type": "tweet", "uri": "/alice/status/123" }
        });
        let message = message(&payload);

        assert_eq!(content_of(&message), "https://x.com/alice/status/123");
        assert_eq!(message["allowed_mentions"]["parse"], json!([]));
    }

    #[test]
    fn prefers_uri_under_data() {
        let payload = json!({ "uri": "/outer/status/1", "data": { "uri": "/inner/status/2" } });
        assert_eq!(content_of(&message(&payload)), "https://x.com/inner/status/2");
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
    fn uri_that_is_not_a_path_or_https_url_falls_back() {
        let payload = json!({ "data": { "uri": "javascript:alert(1)" } });
        assert_eq!(
            content_of(&message(&payload)),
            "```json\n{\"data\":{\"uri\":\"javascript:alert(1)\"}}\n```"
        );
    }
}
