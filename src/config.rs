use crate::error::{Result, AngelicAngelError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub twitter: TwitterConfig,
    pub registration: Option<Registration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterConfig {
    pub auth_token: String,
    pub ct0: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPushKeys {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub auth_secret: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPushSession {
    pub uaid: String,
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub endpoint: String,
    pub autopush: AutoPushSession,
    pub keys: WebPushKeys,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AngelicAngelError::Config(format!("failed to read config ({}): {}", path.display(), e))
        })?;
        toml::from_str(&content).map_err(Into::into)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| AngelicAngelError::Config(format!("failed to serialize config: {}", e)))?;
        write_private(path, content.as_bytes())?;
        Ok(())
    }
}

/// Writes `content` to `path`, readable and writable only by the owner (0600).
///
/// The config holds Twitter session cookies and the push private key.
/// Permissions are also tightened on an existing file before it is written.
#[cfg(unix)]
fn write_private(path: &Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(content)
}

#[cfg(not(unix))]
fn write_private(path: &Path, content: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, content)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn write_private_sets_0600_on_new_and_existing_files() {
        let path = std::env::temp_dir().join(format!("angelic-angel-{}.toml", uuid::Uuid::new_v4()));
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;

        write_private(&path, b"a").unwrap();
        assert_eq!(mode(&path), 0o600);

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&path, b"b").unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(std::fs::read(&path).unwrap(), b"b");

        std::fs::remove_file(&path).unwrap();
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn parses_webhook_format() {
        assert_eq!(parse_webhook_format("").unwrap(), WebhookFormat::Raw);
        assert_eq!(parse_webhook_format("raw").unwrap(), WebhookFormat::Raw);
        assert_eq!(parse_webhook_format(" Discord ").unwrap(), WebhookFormat::Discord);
        assert!(parse_webhook_format("slack").is_err());
    }
}

/// Shape of the JSON body POSTed to the webhook.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebhookFormat {
    /// The decrypted payload as received from X.
    Raw,
    /// A Discord webhook message (`content` with title, body and tweet link).
    Discord,
}

/// Reads the webhook body format from the WEBHOOK_FORMAT environment variable (default: raw).
pub fn get_webhook_format() -> Result<WebhookFormat> {
    match std::env::var("WEBHOOK_FORMAT") {
        Ok(value) => parse_webhook_format(&value),
        Err(_) => Ok(WebhookFormat::Raw),
    }
}

fn parse_webhook_format(value: &str) -> Result<WebhookFormat> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "raw" => Ok(WebhookFormat::Raw),
        "discord" => Ok(WebhookFormat::Discord),
        other => Err(AngelicAngelError::Config(format!(
            "unknown WEBHOOK_FORMAT '{}': expected 'raw' or 'discord'",
            other
        ))),
    }
}

/// Reads the webhook endpoint URL from the WEBHOOK_ENDPOINT environment variable.
pub fn get_webhook_endpoint() -> Result<String> {
    std::env::var("WEBHOOK_ENDPOINT").map_err(|_| {
        AngelicAngelError::Config("WEBHOOK_ENDPOINT environment variable is not set".to_string())
    })
}
