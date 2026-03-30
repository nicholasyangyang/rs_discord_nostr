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
