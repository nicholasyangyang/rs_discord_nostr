use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;
use crate::error::AppError;
use crate::keys::KeyStore;

#[async_trait]
pub trait NostrSender: Send + Sync {
    async fn send_dm(&self, to_npub: &str, content: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait DiscordSender: Send + Sync {
    async fn send_message(&self, channel_id: u64, text: &str) -> Result<(), AppError>;
}

pub struct AppState {
    pub keys: Arc<KeyStore>,
    pub nostr: Arc<dyn NostrSender>,
    pub discord: Arc<dyn DiscordSender>,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(
        keys: Arc<KeyStore>,
        nostr: Arc<dyn NostrSender>,
        discord: Arc<dyn DiscordSender>,
        config: Arc<Config>,
    ) -> Self {
        Self { keys, nostr, discord, config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyStore;
    use tempfile::TempDir;

    struct MockNostr;
    struct MockDiscord;

    #[async_trait]
    impl NostrSender for MockNostr {
        async fn send_dm(&self, _to: &str, _content: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[async_trait]
    impl DiscordSender for MockDiscord {
        async fn send_message(&self, _channel_id: u64, _text: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn make_config() -> Arc<Config> {
        Arc::new(Config {
            discord_token: "tok".into(),
            channel_id: 12345,
            allowed_users: vec![],
            msg_to: "npub1test".into(),
            nostr_relays: vec![],
        })
    }

    #[test]
    fn test_app_state_constructs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("key.json");
        let keys = Arc::new(KeyStore::load_or_generate(&path).unwrap());
        let state = AppState::new(keys, Arc::new(MockNostr), Arc::new(MockDiscord), make_config());
        assert_eq!(state.config.channel_id, 12345);
    }
}
