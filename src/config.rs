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
