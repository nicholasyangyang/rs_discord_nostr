use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tempfile::TempDir;

use rs_discord_nostr::config::Config;
use rs_discord_nostr::discord::process_message;
use rs_discord_nostr::error::AppError;
use rs_discord_nostr::keys::KeyStore;
use rs_discord_nostr::state::{AppState, DiscordSender, NostrSender};

#[derive(Clone)]
struct MockNostr {
    calls: Arc<RwLock<Vec<(String, String)>>>,
}

impl MockNostr {
    fn new() -> Self {
        Self { calls: Arc::new(RwLock::new(vec![])) }
    }
    fn get_calls(&self) -> Vec<(String, String)> {
        self.calls.read().unwrap().clone()
    }
}

#[async_trait]
impl NostrSender for MockNostr {
    async fn send_dm(&self, to: &str, content: &str) -> Result<(), AppError> {
        self.calls.write().unwrap().push((to.to_string(), content.to_string()));
        Ok(())
    }
}

#[derive(Clone)]
struct MockDiscord {
    calls: Arc<RwLock<Vec<(u64, String)>>>,
}

impl MockDiscord {
    fn new() -> Self {
        Self { calls: Arc::new(RwLock::new(vec![])) }
    }
    fn get_calls(&self) -> Vec<(u64, String)> {
        self.calls.read().unwrap().clone()
    }
}

#[async_trait]
impl DiscordSender for MockDiscord {
    async fn send_message(&self, channel_id: u64, text: &str) -> Result<(), AppError> {
        self.calls.write().unwrap().push((channel_id, text.to_string()));
        Ok(())
    }
}

fn make_state(nostr: MockNostr, discord: MockDiscord) -> (Arc<AppState>, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("key.json");
    let keys = Arc::new(KeyStore::load_or_generate(&path).unwrap());
    let config = Arc::new(Config {
        discord_token: "tok".into(),
        channel_id: 100,
        allowed_users: vec![],
        msg_to: "npub1target".into(),
        nostr_relays: vec![],
    });
    let state = Arc::new(AppState::new(keys, Arc::new(nostr), Arc::new(discord), config));
    (state, dir)
}

#[tokio::test]
async fn test_discord_to_nostr_sends_dm() {
    let nostr = MockNostr::new();
    let discord = MockDiscord::new();
    let (state, _dir) = make_state(nostr.clone(), discord);

    process_message(&state, "hello nostr").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let calls = nostr.get_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "npub1target");
    assert_eq!(calls[0].1, "hello nostr");
}

#[tokio::test]
async fn test_nostr_to_discord_sends_to_channel() {
    let nostr = MockNostr::new();
    let discord = MockDiscord::new();
    let (state, _dir) = make_state(nostr, discord.clone());

    state.discord.send_message(state.config.channel_id, "hello discord").await.unwrap();

    let calls = discord.get_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (100u64, "hello discord".to_string()));
}

#[tokio::test]
async fn test_multiple_messages_all_sent() {
    let nostr = MockNostr::new();
    let discord = MockDiscord::new();
    let (state, _dir) = make_state(nostr.clone(), discord);

    process_message(&state, "msg1").await;
    process_message(&state, "msg2").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let calls = nostr.get_calls();
    assert_eq!(calls.len(), 2);
}
