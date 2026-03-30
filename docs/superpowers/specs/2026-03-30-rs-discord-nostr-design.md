# rs_discord_nostr 设计文档

**日期：** 2026-03-30
**状态：** 已批准
**目标：** 参考 rs_tg_nostr 架构，用 Rust 实现 Discord ↔ Nostr 消息桥接，单一二进制，TDD 驱动

---

## 1. 项目概述

`rs_discord_nostr` 是 `rs_tg_nostr` 的 Discord 版本。将 Telegram 的 webhook server 替换为 Discord Gateway（serenity），其余模块（keys、nostr、state、config、error）最大程度复用 TG 版的设计。

**核心功能：**
- 监听指定 Discord 频道的消息
- 通过 NIP-17 Gift Wrap（kind:1059）将 Discord 消息发送到 Nostr relay
- 订阅 Nostr relay 收取 DM，转发回指定 Discord 频道

---

## 2. 技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 异步运行时 | tokio | 生态标准，与 rs_tg_nostr 一致 |
| Nostr 库 | nostr-sdk 0.44 | 原生支持 NIP-17、NIP-44、relay pool |
| Discord 库 | serenity 0.12 | 封装 gateway 连接、heartbeat、reconnect，事件驱动 trait 风格 |
| HTTP 客户端 | serenity 内置（发送消息），reqwest 不再需要 | serenity 已包含 Discord REST client |
| 日志追踪 | tracing + tracing-subscriber | 与 rs_tg_nostr 一致 |
| 错误处理 | thiserror（库层）+ anyhow（app 层） | 与 rs_tg_nostr 一致 |
| 配置 | dotenvy | 读取 .env 文件 |

---

## 3. 项目结构

```
rs_discord_nostr/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── .gitignore
└── src/
    ├── main.rs       # 入口：解析 --cwd-dir，初始化 tracing，启动 App
    ├── config.rs     # 从 .env 读取配置
    ├── keys.rs       # KeyStore：读写 key.json（与 TG 版相同）
    ├── error.rs      # AppError（与 TG 版相同）
    ├── state.rs      # AppState + NostrSender + DiscordSender traits
    ├── nostr.rs      # NostrBridge（与 TG 版相同，零改动）
    ├── discord.rs    # serenity EventHandler + DiscordClient
    ├── app.rs        # App::run()：组装 AppState，启动序列
    └── lib.rs
tests/
    ├── keys_test.rs
    ├── nostr_test.rs
    ├── discord_test.rs
    └── bridge_test.rs
docs/
    └── superpowers/
        └── specs/
            └── 2026-03-30-rs-discord-nostr-design.md
```

---

## 4. 配置（.env）

```env
DISCORD_TOKEN=your_discord_bot_token
CHANNEL_ID=123456789012345678
ALLOWED_USERS=123456789,987654321
MSG_TO=npub1...
NOSTR_RELAYS=wss://relay.damus.io,wss://relay.0xchat.com
LOG_LEVEL=info
```

- `CHANNEL_ID`：监听并回复的 Discord 频道 ID（u64）
- `ALLOWED_USERS`：Discord user ID 白名单，留空则允许所有人
- `MSG_TO`：Nostr DM 目标 npub
- 无 `PORT` 字段：Discord 版无 HTTP webhook server

启动命令：
```bash
rs_discord_nostr --cwd-dir ~/bot-data/
```

---

## 5. AppState 设计

```rust
pub struct AppState {
    pub keys: Arc<KeyStore>,
    pub nostr: Arc<dyn NostrSender>,
    pub discord: Arc<dyn DiscordSender>,
    pub config: Arc<Config>,
    // 无 chat_id：Discord 用固定 CHANNEL_ID，不需运行时记录
}

#[async_trait]
pub trait NostrSender: Send + Sync {
    async fn send_dm(&self, to_npub: &str, content: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait DiscordSender: Send + Sync {
    async fn send_message(&self, channel_id: u64, text: &str) -> Result<(), AppError>;
}
```

与 TG 版相比，去掉了 `Arc<RwLock<Option<i64>>> chat_id`，因为目标频道由配置固定。

---

## 6. 启动序列（app.rs）

```
1. Config::from_env()
2. KeyStore::load_or_generate(cwd_dir/key.json)
3. NostrBridge::connect(relays, keys)
4. DiscordClient::new(discord_token)
5. 组装 AppState(Arc)
6. tokio::spawn(nostr_listener_task)   # 订阅 Nostr relay，收 DM 转发到 Discord
7. discord_client.start(state).await   # 连接 Discord Gateway（阻塞）
```

serenity 的 `client.start()` 是阻塞调用，因此 nostr_listener_task 必须在此之前 spawn。

---

## 7. 数据流

### Discord → Nostr

```
serenity on_message() 回调（EventHandler::message）
  → 过滤：author.bot == true → 跳过（避免 bot 自身消息形成循环）
  → 过滤：channel_id != config.channel_id → 跳过
  → 过滤：author.id 不在 ALLOWED_USERS（若配置了白名单）→ 跳过
  → nostr.send_dm(config.msg_to, text)  # NIP-17 gift wrap → relay
```

### Nostr → Discord

```
nostr-sdk relay 事件循环
  → 收到 kind:1059 gift wrap
  → unwrap_gift_wrap() → plaintext
  → discord.send_message(config.channel_id, plaintext)
```

---

## 8. Discord 模块（discord.rs）

```rust
pub struct DiscordClient {
    token: String,
    http: Arc<serenity::http::Http>,
}

struct Handler {
    state: Arc<AppState>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot { return; }
        if msg.channel_id.get() != self.state.config.channel_id { return; }
        if !self.state.config.allowed_users.is_empty()
            && !self.state.config.allowed_users.contains(&(msg.author.id.get() as i64))
        {
            return;
        }
        let Some(text) = msg.content.as_str().to_string().into() else { return; };
        let nostr = self.state.nostr.clone();
        let msg_to = self.state.config.msg_to.clone();
        tokio::spawn(async move {
            if let Err(e) = nostr.send_dm(&msg_to, &text).await {
                tracing::warn!("Failed to send Nostr DM: {}", e);
            }
        });
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!("Discord bot connected: {}", ready.user.name);
    }
}

impl DiscordClient {
    pub fn new(token: String) -> Self {
        let http = Arc::new(serenity::http::Http::new(&token));
        Self { token, http }
    }

    pub async fn start(self: Arc<Self>, state: Arc<AppState>) -> Result<(), AppError> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;  // privileged intent

        let mut client = Client::builder(&self.token, intents)
            .event_handler(Handler { state })
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;

        client.start().await
            .map_err(|e| AppError::Discord(e.to_string()))
    }
}

#[async_trait]
impl DiscordSender for DiscordClient {
    async fn send_message(&self, channel_id: u64, text: &str) -> Result<(), AppError> {
        ChannelId::new(channel_id)
            .say(&self.http, text)
            .await
            .map_err(|e| AppError::Discord(e.to_string()))?;
        Ok(())
    }
}
```

**重要：`MESSAGE_CONTENT` 是 privileged intent**，需在 Discord 开发者后台 → Bot → Privileged Gateway Intents 手动开启，否则收到消息的 `content` 字段为空字符串。

---

## 9. 错误处理

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("key file error: {0}")]
    Keys(#[from] std::io::Error),
    #[error("nostr error: {0}")]
    Nostr(String),
    #[error("discord error: {0}")]
    Discord(String),           // 替换 TG 版的 Telegram(String)
    #[error("config missing: {0}")]
    Config(String),
}
```

- Discord gateway 断线 → serenity 内置自动重连
- Nostr relay 断线 → nostr-sdk 内置自动重连
- 启动失败 → `anyhow::bail!` 打印错误退出

---

## 10. TDD 策略

**原则：** 红→绿→重构，每个循环一次 git commit。

| 测试文件 | 覆盖内容 | mock 策略 |
|----------|----------|-----------|
| `keys_test.rs` | key.json 生成、读取、原子写入（直接复制 TG 版） | `tempfile::TempDir` |
| `nostr_test.rs` | NIP-17 wrap/unwrap 往返（直接复制 TG 版） | nostr-sdk 本地密钥对，`#[ignore]` 标注联网测试 |
| `discord_test.rs` | 消息过滤逻辑（bot 自身、channel_id、白名单） | 提取为纯函数，`MockDiscordSender` |
| `bridge_test.rs` | Discord→Nostr 调用链，Nostr→Discord 调用链 | `MockNostrSender` + `MockDiscordSender` |

serenity 的 `Message` 结构体字段为私有，消息过滤逻辑提取为接受基本类型（`bool`, `u64`, `&str`）的纯函数，便于单元测试。

---

## 11. 与 rs_tg_nostr 的对比

| 项目 | rs_tg_nostr | rs_discord_nostr |
|------|-------------|------------------|
| 平台接入 | axum webhook server | serenity Gateway |
| 平台 trait | `TgSender` | `DiscordSender` |
| 目标 chat 记录 | `Arc<RwLock<Option<i64>>>` 运行时记录 | 配置固定 `CHANNEL_ID` |
| key.json 格式 | `{"npub": "...", "nsec": "..."}` | 相同 |
| Nostr 协议 | NIP-17 kind:1059 | 相同 |
| nostr.rs | — | 零改动复用 |
| keys.rs | — | 零改动复用 |
