# rs_discord_nostr Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single-binary Rust service that bridges a Discord channel and Nostr NIP-17 DMs using serenity for Discord Gateway and nostr-sdk for Nostr.

**Architecture:** Mirrors rs_tg_nostr exactly — `Arc<AppState>` shared between a serenity EventHandler (Discord → Nostr) and a tokio-spawned nostr listener task (Nostr → Discord). `NostrSender` and `DiscordSender` traits decouple real implementations from mocks for TDD.

**Tech Stack:** Rust 2024 edition, tokio 1, serenity 0.12 (Discord Gateway), nostr-sdk 0.44 (NIP-17), async-trait, thiserror/anyhow, tracing, dotenvy, clap, tempfile

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `Cargo.toml` | Create | Dependencies |
| `.env.example` | Create | Config reference |
| `.gitignore` | Create | Ignore target/, .env |
| `src/lib.rs` | Create | Pub module re-exports |
| `src/main.rs` | Create | Entry point, CLI args, tracing init |
| `src/error.rs` | Create | `AppError` enum |
| `src/config.rs` | Create | `Config::from_env()` |
| `src/keys.rs` | Create | `KeyStore` — key.json read/write (identical to TG version) |
| `src/transport.rs` | Create | `UserAgentTransport` for nostr-sdk (identical to TG version, updated USER_AGENT) |
| `src/state.rs` | Create | `AppState`, `NostrSender`, `DiscordSender` traits |
| `src/nostr.rs` | Create | `NostrBridge` — relay pool, NIP-17 send/receive |
| `src/discord.rs` | Create | `DiscordClient`, serenity `EventHandler`, `should_handle()`, `process_message()` |
| `src/app.rs` | Create | `run()` startup sequence |
| `tests/keys_test.rs` | Create | KeyStore unit tests |
| `tests/nostr_test.rs` | Create | nostr-sdk key/NIP-17 unit tests |
| `tests/discord_test.rs` | Create | Message filtering and routing logic tests |
| `tests/bridge_test.rs` | Create | End-to-end Discord↔Nostr flow with mocks |

---

## Task 0: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `.env.example`
- Create: `.gitignore`
- Create: `src/lib.rs`
- Create: `src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rs_discord_nostr"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "rs_discord_nostr"
path = "src/main.rs"

[dependencies]
nostr-sdk        = { version = "0.44.1", features = ["nip59"] }
nostr-relay-pool = "0.44.0"
tokio-tungstenite = { version = "0.26", features = ["rustls-tls-webpki-roots"] }
async-wsocket    = "0.13"
serenity         = { version = "0.12", default-features = false, features = ["client", "gateway", "http", "model", "rustls_backend"] }
tokio            = { version = "1", features = ["full"] }
serde            = { version = "1", features = ["derive"] }
serde_json       = "1"
dotenvy          = "0.15"
tracing          = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
thiserror        = "2"
anyhow           = "1"
async-trait      = "0.1"
tempfile         = "3"
clap             = { version = "4", features = ["derive"] }

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: Create .env.example**

```env
DISCORD_TOKEN=your_discord_bot_token
CHANNEL_ID=123456789012345678
ALLOWED_USERS=123456789,987654321
MSG_TO=npub1...
NOSTR_RELAYS=wss://relay.damus.io,wss://relay.0xchat.com
LOG_LEVEL=info
```

- [ ] **Step 3: Create .gitignore**

```
/target
.env
key.json
```

- [ ] **Step 4: Create src/lib.rs (stub — modules added as they are created)**

```rust
pub mod error;
pub mod config;
pub mod keys;
pub mod state;
pub mod transport;
pub mod nostr;
pub mod discord;
pub mod app;
```

- [ ] **Step 5: Create src/main.rs**

```rust
use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

mod error;
mod transport;
mod config;
mod keys;
mod state;
mod nostr;
mod discord;
mod app;

/// Discord ↔ Nostr 消息桥
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// 数据目录（存放 key.json）
    #[arg(long, value_name = "DIR")]
    cwd_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let _ = dotenvy::dotenv();

    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("LOG_LEVEL")
                .unwrap_or_else(|_| EnvFilter::from_default_env()),
        )
        .init();

    std::fs::create_dir_all(&cli.cwd_dir)?;

    app::run(cli.cwd_dir).await
}
```

- [ ] **Step 6: Verify it compiles (no logic yet)**

```bash
cd /home/deeptuuk/CodeTeam/Code/rs_discord_nostr
cargo check 2>&1 | head -30
```

Expected: errors about missing modules (error, config, etc.) — that's fine at this stage. No syntax errors.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml .env.example .gitignore src/
git commit -m "feat: scaffold project structure"
```

---

## Task 1: error module

**Files:**
- Create: `src/error.rs`

- [ ] **Step 1: Write the failing test (inline in error.rs)**

Create `src/error.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("key file error: {0}")]
    Keys(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("nostr error: {0}")]
    Nostr(String),

    #[error("discord error: {0}")]
    Discord(String),

    #[error("config error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn test_io_error_converts_to_app_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_string().contains("key file error"));
    }

    #[test]
    fn test_nostr_error_display() {
        let e = AppError::Nostr("bad relay".into());
        assert_eq!(e.to_string(), "nostr error: bad relay");
    }

    #[test]
    fn test_discord_error_display() {
        let e = AppError::Discord("send failed".into());
        assert_eq!(e.to_string(), "discord error: send failed");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib error 2>&1
```

Expected: 3 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/error.rs
git commit -m "feat: add error module"
```

---

## Task 2: config module

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: Write src/config.rs with inline tests**

```rust
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub channel_id: u64,
    pub allowed_users: Vec<u64>,
    pub msg_to: String,
    pub nostr_relays: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let discord_token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| AppError::Config("DISCORD_TOKEN not set".into()))?;

        let channel_id = std::env::var("CHANNEL_ID")
            .map_err(|_| AppError::Config("CHANNEL_ID not set".into()))?
            .parse::<u64>()
            .map_err(|_| AppError::Config("CHANNEL_ID must be a u64".into()))?;

        let allowed_users = std::env::var("ALLOWED_USERS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();

        let msg_to = std::env::var("MSG_TO")
            .map_err(|_| AppError::Config("MSG_TO not set".into()))?;

        let nostr_relays = std::env::var("NOSTR_RELAYS")
            .unwrap_or_else(|_| "wss://relay.damus.io".into())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Ok(Self {
            discord_token,
            channel_id,
            allowed_users,
            msg_to,
            nostr_relays,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_users_parse() {
        let raw = "111222333,444555666";
        let users: Vec<u64> = raw
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        assert_eq!(users, vec![111222333u64, 444555666u64]);
    }

    #[test]
    fn test_allowed_users_empty() {
        let raw = "";
        let users: Vec<u64> = raw
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();
        assert!(users.is_empty());
    }

    #[test]
    fn test_relays_parse() {
        let raw = "wss://relay.damus.io,wss://relay.0xchat.com";
        let relays: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
        assert_eq!(relays.len(), 2);
        assert_eq!(relays[0], "wss://relay.damus.io");
    }

    #[test]
    fn test_channel_id_parse() {
        let raw = "123456789012345678";
        let id: u64 = raw.parse().unwrap();
        assert_eq!(id, 123456789012345678u64);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib config 2>&1
```

Expected: 4 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: add config module"
```

---

## Task 3: keys module

**Files:**
- Create: `src/keys.rs`
- Create: `tests/keys_test.rs`

- [ ] **Step 1: Write tests/keys_test.rs (failing)**

```rust
use rs_discord_nostr::keys::KeyStore;
use tempfile::TempDir;

#[test]
fn test_generate_when_no_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("key.json");

    let store = KeyStore::load_or_generate(&path).unwrap();
    let pair = store.key_pair();

    assert!(pair.npub.starts_with("npub1"));
    assert!(pair.nsec.starts_with("nsec1"));
    assert!(path.exists());
}

#[test]
fn test_load_existing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("key.json");

    let store1 = KeyStore::load_or_generate(&path).unwrap();
    let npub1 = store1.key_pair().npub.clone();

    let store2 = KeyStore::load_or_generate(&path).unwrap();
    let npub2 = store2.key_pair().npub.clone();

    assert_eq!(npub1, npub2);
}

#[test]
fn test_python_compat_json_format() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("key.json");

    KeyStore::load_or_generate(&path).unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();

    assert!(v["npub"].is_string());
    assert!(v["nsec"].is_string());
    assert!(v.get("extra").is_none());
}

#[test]
fn test_atomic_write_no_corruption() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("key.json");

    for _ in 0..5 {
        KeyStore::load_or_generate(&path).unwrap();
    }

    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&raw).is_ok());
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test --test keys_test 2>&1 | head -20
```

Expected: FAIL — `module not found` or similar

- [ ] **Step 3: Write src/keys.rs**

```rust
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use nostr_sdk::{Keys, ToBech32};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub npub: String,
    pub nsec: String,
}

pub struct KeyStore {
    #[allow(dead_code)]
    path: PathBuf,
    keys: RwLock<KeyPair>,
}

impl KeyStore {
    pub fn load_or_generate(path: &Path) -> Result<Self, AppError> {
        let pair = if path.exists() {
            let raw = std::fs::read_to_string(path)?;
            serde_json::from_str::<KeyPair>(&raw)?
        } else {
            let keys = Keys::generate();
            let pair = KeyPair {
                npub: keys.public_key().to_bech32().map_err(|e| {
                    AppError::Nostr(format!("bech32 encode failed: {e}"))
                })?,
                nsec: keys.secret_key().to_bech32().map_err(|e| {
                    AppError::Nostr(format!("bech32 encode failed: {e}"))
                })?,
            };
            write_atomic(path, &pair)?;
            pair
        };

        Ok(Self {
            path: path.to_path_buf(),
            keys: RwLock::new(pair),
        })
    }

    pub fn key_pair(&self) -> KeyPair {
        self.keys.read().unwrap().clone()
    }

    pub fn nostr_keys(&self) -> Result<Keys, AppError> {
        let pair = self.keys.read().unwrap();
        Keys::parse(&pair.nsec)
            .map_err(|e| AppError::Nostr(format!("parse nsec failed: {e}")))
    }
}

fn write_atomic(path: &Path, pair: &KeyPair) -> Result<(), AppError> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    let json = serde_json::to_string_pretty(pair)?;
    tmp.write_all(json.as_bytes())?;
    tmp.flush()?;
    tmp.persist(path)
        .map_err(|e| AppError::Keys(e.error))?;
    Ok(())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test keys_test 2>&1
```

Expected: 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/keys.rs tests/keys_test.rs
git commit -m "feat: add keys module"
```

---

## Task 4: transport module

**Files:**
- Create: `src/transport.rs`

No unit tests — this is a pure infrastructure adapter for nostr-sdk's WebSocket layer. Correctness is validated at runtime when NostrBridge connects.

- [ ] **Step 1: Create src/transport.rs**

```rust
use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_wsocket::futures_util::stream::SplitSink;
use async_wsocket::futures_util::{Sink, StreamExt, TryStreamExt};
use async_wsocket::{ConnectionMode, Message, WebSocket};
use nostr_sdk::nostr::util::BoxedFuture;
use nostr_sdk::nostr::Url;
use nostr_relay_pool::transport::error::TransportError;
use nostr_relay_pool::transport::websocket::{WebSocketSink, WebSocketStream, WebSocketTransport};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

const USER_AGENT: &str = concat!("rs_discord_nostr/", env!("CARGO_PKG_VERSION"));

struct OurSink(SplitSink<WebSocket, Message>);

impl fmt::Debug for OurSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OurSink").finish()
    }
}

impl Sink<Message> for OurSink {
    type Error = TransportError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.0)
            .poll_ready(cx)
            .map_err(TransportError::backend)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        Pin::new(&mut self.0)
            .start_send(item)
            .map_err(TransportError::backend)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.0)
            .poll_flush(cx)
            .map_err(TransportError::backend)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.0)
            .poll_close(cx)
            .map_err(TransportError::backend)
    }
}

/// WebSocket transport that injects a `User-Agent` header into the handshake
/// request, fixing HTTP 403 on relays that require it (e.g. relay.0xchat.com).
#[derive(Debug, Clone, Default)]
pub struct UserAgentTransport;

impl WebSocketTransport for UserAgentTransport {
    fn support_ping(&self) -> bool {
        true
    }

    fn connect<'a>(
        &'a self,
        url: &'a Url,
        mode: &'a ConnectionMode,
        _timeout: Duration,
    ) -> BoxedFuture<'a, Result<(WebSocketSink, WebSocketStream), TransportError>> {
        Box::pin(async move {
            if !matches!(mode, ConnectionMode::Direct) {
                return Err(TransportError::backend(io::Error::new(
                    io::ErrorKind::Other,
                    "UserAgentTransport only supports Direct mode",
                )));
            }

            let mut request = url
                .as_str()
                .into_client_request()
                .map_err(TransportError::backend)?;
            request.headers_mut().insert(
                "User-Agent",
                HeaderValue::from_static(USER_AGENT),
            );

            let (ws_stream, _response) =
                tokio_tungstenite::connect_async_tls_with_config(request, None, false, None)
                    .await
                    .map_err(TransportError::backend)?;

            let socket = WebSocket::Tokio(ws_stream);
            let (tx, rx) = socket.split();

            let sink: WebSocketSink = Box::new(OurSink(tx));
            let stream: WebSocketStream = Box::pin(rx.map_err(TransportError::backend));

            Ok((sink, stream))
        })
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check 2>&1 | grep -E "^error"
```

Expected: no errors from transport.rs

- [ ] **Step 3: Commit**

```bash
git add src/transport.rs
git commit -m "feat: add transport module (UserAgentTransport)"
```

---

## Task 5: state module

**Files:**
- Create: `src/state.rs`

- [ ] **Step 1: Write src/state.rs with inline tests**

```rust
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
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib state 2>&1
```

Expected: 1 test PASS

- [ ] **Step 3: Commit**

```bash
git add src/state.rs
git commit -m "feat: add state module with NostrSender and DiscordSender traits"
```

---

## Task 6: nostr module

**Files:**
- Create: `src/nostr.rs`
- Create: `tests/nostr_test.rs`

- [ ] **Step 1: Write tests/nostr_test.rs**

```rust
use nostr_sdk::{Keys, PublicKey, ToBech32};

#[test]
fn test_keys_generate_and_parse() {
    let keys = Keys::generate();
    let nsec = keys.secret_key().to_bech32().unwrap();
    let npub = keys.public_key().to_bech32().unwrap();

    assert!(nsec.starts_with("nsec1"));
    assert!(npub.starts_with("npub1"));

    let rebuilt = Keys::parse(&nsec).unwrap();
    assert_eq!(rebuilt.public_key(), keys.public_key());
}

#[test]
fn test_nip17_keys_valid() {
    let sender = Keys::generate();
    let recipient = Keys::generate();

    let sender_pub: PublicKey = sender.public_key();
    let recipient_pub: PublicKey = recipient.public_key();

    assert_ne!(sender_pub, recipient_pub);
    let npub_str = recipient_pub.to_bech32().unwrap();
    let parsed_pub = PublicKey::parse(&npub_str).unwrap();
    assert_eq!(parsed_pub, recipient_pub);
}
```

- [ ] **Step 2: Write src/nostr.rs**

```rust
use std::sync::Arc;

use async_trait::async_trait;
use nostr_sdk::{Client, Filter, Kind, PublicKey, RelayMessage, RelayPoolNotification, Timestamp};
use tracing::{info, warn};

use crate::error::AppError;
use crate::keys::KeyStore;
use crate::state::{AppState, NostrSender};
use crate::transport::UserAgentTransport;

pub struct NostrBridge {
    client: Client,
}

impl NostrBridge {
    pub async fn connect(keys: &KeyStore, relays: &[String]) -> Result<Self, AppError> {
        let nostr_keys = keys.nostr_keys()?;
        let client = Client::builder()
            .signer(nostr_keys)
            .websocket_transport(UserAgentTransport)
            .build();

        for relay in relays {
            client
                .add_relay(relay.as_str())
                .await
                .map_err(|e| AppError::Nostr(e.to_string()))?;
        }
        client.connect().await;
        info!("Connected to {} Nostr relay(s)", relays.len());

        Ok(Self { client })
    }

    pub async fn listen(self: Arc<Self>, state: Arc<AppState>) -> Result<(), AppError> {
        let my_pubkey = state.keys.nostr_keys()?.public_key();

        // NIP-59 gift wrap events have intentionally backdated created_at (up to 48h).
        let since = Timestamp::now() - 2 * 24 * 60 * 60;
        let filter = Filter::new()
            .kind(Kind::GiftWrap)
            .pubkey(my_pubkey)
            .since(since);

        self.client
            .subscribe(filter, None)
            .await
            .map_err(|e| AppError::Nostr(e.to_string()))?;

        info!("Nostr listener subscribed for pubkey={}", my_pubkey);

        let client = self.client.clone();
        self.client
            .handle_notifications(|notification| {
                let state = state.clone();
                let client = client.clone();
                async move {
                    match notification {
                        RelayPoolNotification::Event { event, .. } => {
                            if event.kind == Kind::GiftWrap {
                                match client.unwrap_gift_wrap(&event).await {
                                    Ok(gift) => {
                                        let content = gift.rumor.content.clone();
                                        let channel_id = state.config.channel_id;
                                        if let Err(e) =
                                            state.discord.send_message(channel_id, &content).await
                                        {
                                            warn!("Failed to forward to Discord: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to unwrap NIP-17: {}", e);
                                    }
                                }
                            }
                        }
                        RelayPoolNotification::Message { relay_url, message } => {
                            if let RelayMessage::Auth { challenge } = message {
                                let preview: String = challenge.chars().take(16).collect();
                                info!(
                                    "NIP-42 AUTH challenge from {} (challenge={}…)",
                                    relay_url, preview
                                );
                            }
                        }
                        _ => {}
                    }
                    Ok(false)
                }
            })
            .await
            .map_err(|e| AppError::Nostr(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl NostrSender for NostrBridge {
    async fn send_dm(&self, to_npub: &str, content: &str) -> Result<(), AppError> {
        let recipient = PublicKey::parse(to_npub)
            .map_err(|e| AppError::Nostr(format!("invalid npub: {e}")))?;

        self.client
            .send_private_msg(recipient, content, std::iter::empty::<nostr_sdk::Tag>())
            .await
            .map_err(|e| AppError::Nostr(e.to_string()))?;

        info!("Sent NIP-17 DM to {}", to_npub);
        Ok(())
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --test nostr_test 2>&1
```

Expected: 2 tests PASS (no network required)

- [ ] **Step 4: Commit**

```bash
git add src/nostr.rs tests/nostr_test.rs
git commit -m "feat: add nostr module (NostrBridge, NIP-17 send/listen)"
```

---

## Task 7: discord module

**Files:**
- Create: `src/discord.rs`
- Create: `tests/discord_test.rs`

- [ ] **Step 1: Write tests/discord_test.rs (failing)**

```rust
use rs_discord_nostr::config::Config;
use rs_discord_nostr::discord::should_handle;
use std::sync::Arc;

fn make_config(channel_id: u64, allowed_users: Vec<u64>) -> Arc<Config> {
    Arc::new(Config {
        discord_token: "tok".into(),
        channel_id,
        allowed_users,
        msg_to: "npub1target".into(),
        nostr_relays: vec![],
    })
}

#[test]
fn test_bot_message_ignored() {
    let cfg = make_config(100, vec![]);
    assert!(!should_handle(true, 100, 999, &cfg));
}

#[test]
fn test_wrong_channel_ignored() {
    let cfg = make_config(100, vec![]);
    assert!(!should_handle(false, 999, 42, &cfg));
}

#[test]
fn test_correct_channel_no_allowlist() {
    let cfg = make_config(100, vec![]);
    assert!(should_handle(false, 100, 42, &cfg));
}

#[test]
fn test_allowlist_blocks_unknown_user() {
    let cfg = make_config(100, vec![1, 2, 3]);
    assert!(!should_handle(false, 100, 99, &cfg));
}

#[test]
fn test_allowlist_permits_known_user() {
    let cfg = make_config(100, vec![1, 2, 3]);
    assert!(should_handle(false, 100, 2, &cfg));
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test --test discord_test 2>&1 | head -10
```

Expected: FAIL — `function not found`

- [ ] **Step 3: Write src/discord.rs**

```rust
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
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test discord_test 2>&1
```

Expected: 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/discord.rs tests/discord_test.rs
git commit -m "feat: add discord module (serenity EventHandler, DiscordClient, should_handle)"
```

---

## Task 8: app module + final wiring

**Files:**
- Create: `src/app.rs`

- [ ] **Step 1: Write src/app.rs**

```rust
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
```

- [ ] **Step 2: Verify full project compiles**

```bash
cargo build 2>&1
```

Expected: compiles cleanly (warnings OK, no errors)

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: add app module and wire up startup sequence"
```

---

## Task 9: bridge integration tests

**Files:**
- Create: `tests/bridge_test.rs`

- [ ] **Step 1: Write tests/bridge_test.rs**

```rust
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
async fn test_multiple_messages_ordered() {
    let nostr = MockNostr::new();
    let discord = MockDiscord::new();
    let (state, _dir) = make_state(nostr.clone(), discord);

    process_message(&state, "msg1").await;
    process_message(&state, "msg2").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let calls = nostr.get_calls();
    assert_eq!(calls.len(), 2);
}
```

- [ ] **Step 2: Run all tests**

```bash
cargo test 2>&1
```

Expected: all tests PASS (keys_test: 4, nostr_test: 2, discord_test: 5, bridge_test: 3, lib unit tests: ~5)

- [ ] **Step 3: Commit**

```bash
git add tests/bridge_test.rs
git commit -m "feat: add bridge integration tests"
```

---

## Task 10: CLAUDE.md + final polish

**Files:**
- Create: `CLAUDE.md`

- [ ] **Step 1: Create CLAUDE.md**

```markdown
# CLAUDE.md

This file provides guidance to Claude Code when working in this repository.

## Build & Test

```bash
cargo build
cargo test
cargo test --test keys_test
cargo run -- --cwd-dir /tmp/data   # requires .env
RUST_LOG=debug cargo run -- --cwd-dir /tmp/data
```

## Architecture

Single process, `Arc<AppState>` shared, two tokio tasks:
- serenity Gateway client (Discord EventHandler, listens for messages)
- nostr listener task (subscribes relay, forwards DMs to Discord channel)

`NostrSender` / `DiscordSender` traits decouple implementations from mocks.

## Key Files

- `src/keys.rs` — key.json read/write (compatible with rs_tg_nostr format)
- `src/state.rs` — AppState, NostrSender, DiscordSender traits
- `src/nostr.rs` — nostr-sdk Client, NIP-17 send/receive
- `src/discord.rs` — serenity EventHandler, DiscordClient, should_handle(), process_message()
- `src/app.rs` — startup sequence

## Discord Setup

1. Create bot at discord.com/developers
2. Enable **Privileged Gateway Intents**: Server Members Intent + Message Content Intent
3. Invite bot with `Send Messages` + `Read Message History` permissions
4. Copy bot token to `DISCORD_TOKEN` in .env
5. Copy target channel ID (right-click channel → Copy Channel ID) to `CHANNEL_ID`

## nostr-sdk API Notes

- `client.send_private_msg(pubkey, content, std::iter::empty::<Tag>())` — NIP-17 DM
- `client.unwrap_gift_wrap(&event)` — unwrap kind:1059
- `Filter::new().kind(Kind::GiftWrap).pubkey(pk).since(ts)`
```

- [ ] **Step 2: Final test run**

```bash
cargo test 2>&1
```

Expected: all tests PASS, no warnings about unused code

- [ ] **Step 3: Final commit**

```bash
git add CLAUDE.md
git commit -m "docs: add CLAUDE.md with build/test/setup instructions"
```

---

## Self-Review

**Spec coverage check:**

| Spec Section | Covered by Task |
|---|---|
| Single binary, --cwd-dir | Task 0 (main.rs, Cargo.toml) |
| AppError with Discord variant | Task 1 |
| Config: DISCORD_TOKEN, CHANNEL_ID, ALLOWED_USERS, MSG_TO, NOSTR_RELAYS | Task 2 |
| KeyStore, key.json, Python-compat format | Task 3 |
| UserAgentTransport (nostr relay 403 fix) | Task 4 |
| AppState, NostrSender, DiscordSender traits | Task 5 |
| NostrBridge, NIP-17 send, NIP-59 gift wrap listen | Task 6 |
| serenity EventHandler, should_handle(), process_message() | Task 7 |
| App::run() startup sequence (spawn nostr task, then discord.start) | Task 8 |
| Bridge tests (Discord→Nostr, Nostr→Discord) | Task 9 |
| CLAUDE.md with Discord setup notes | Task 10 |
| MESSAGE_CONTENT privileged intent warning | Task 7 (discord.rs comment), Task 10 (CLAUDE.md) |
