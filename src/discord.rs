use std::sync::Arc;

use async_trait::async_trait;
use serenity::all::{ChannelId, Context, EventHandler, GatewayIntents, Message, Ready};
use serenity::http::Http;
use serenity::Client;
use tracing::{info, warn};

use crate::config::Config;
use crate::error::AppError;
use crate::state::{AppState, DiscordSender};

/// Returns true if this message should be bridged to Nostr.
/// Extracted as a pure function for testability (serenity's Message fields are private).
pub fn should_handle(is_bot: bool, channel_id: u64, author_id: u64, config: &Config) -> bool {
    if is_bot {
        return false;
    }
    if channel_id != config.channel_id {
        return false;
    }
    if !config.allowed_users.is_empty() && !config.allowed_users.contains(&author_id) {
        return false;
    }
    true
}

/// Core bridge logic for a single incoming Discord message.
/// Separated from the EventHandler to allow unit testing.
pub async fn process_message(state: &Arc<AppState>, content: &str) {
    let nostr = state.nostr.clone();
    let msg_to = state.config.msg_to.clone();
    let content = content.to_string();
    tokio::spawn(async move {
        if let Err(e) = nostr.send_dm(&msg_to, &content).await {
            warn!("Failed to send Nostr DM: {}", e);
        }
    });
}

struct Handler {
    state: Arc<AppState>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if !should_handle(
            msg.author.bot,
            msg.channel_id.get(),
            msg.author.id.get(),
            &self.state.config,
        ) {
            return;
        }
        if msg.content.is_empty() {
            return;
        }
        process_message(&self.state, &msg.content).await;
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Discord bot connected: {}", ready.user.name);
    }
}

pub struct DiscordClient {
    token: String,
    http: Arc<Http>,
}

impl DiscordClient {
    pub fn new(token: String) -> Self {
        let http = Arc::new(Http::new(&token));
        Self { token, http }
    }

    pub async fn start(&self, state: Arc<AppState>) -> Result<(), AppError> {
        let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

        let mut client = Client::builder(&self.token, intents)
            .event_handler(Handler { state })
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;

        client.start().await.map_err(|e| AppError::Discord(e.to_string()))
    }
}

#[async_trait]
impl DiscordSender for DiscordClient {
    async fn send_message(&self, channel_id: u64, text: &str) -> Result<(), AppError> {
        ChannelId::new(channel_id)
            .say(self.http.as_ref(), text)
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;
        Ok(())
    }
}
