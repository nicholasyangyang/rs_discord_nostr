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
