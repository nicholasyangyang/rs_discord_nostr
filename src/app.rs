use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

use crate::config::Config;
use crate::discord::DiscordClient;
use crate::keys::KeyStore;
use crate::nostr::NostrBridge;
use crate::state::{AppState, DiscordSender};

pub async fn run(cwd_dir: PathBuf) -> Result<()> {
    // 1. Load config
    let config = Arc::new(Config::from_env().context("failed to load config")?);

    // 2. Load or generate keys
    let key_path = cwd_dir.join("key.json");
    let keys = Arc::new(
        KeyStore::load_or_generate(&key_path).context("failed to load keys")?,
    );
    info!("Loaded keys: npub={}", keys.key_pair().npub);

    // 3. Connect to Nostr relay pool
    let nostr = Arc::new(
        NostrBridge::connect(&keys, &config.nostr_relays)
            .await
            .context("failed to connect to Nostr relays")?,
    );

    // 4. Create Discord client
    let discord_client = Arc::new(DiscordClient::new(config.discord_token.clone()));

    // 5. Assemble AppState
    let state = Arc::new(AppState::new(
        keys,
        nostr.clone(),
        discord_client.clone() as Arc<dyn DiscordSender>,
        config.clone(),
    ));

    // 6. Start Nostr listener task
    let nostr_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = nostr.listen(nostr_state).await {
            tracing::error!("Nostr listener exited: {}", e);
        }
    });

    // 7. Start Discord Gateway (blocking)
    info!("Connecting to Discord Gateway...");
    discord_client
        .start(state)
        .await
        .context("Discord gateway error")?;

    Ok(())
}
