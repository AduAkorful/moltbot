//! Agent configuration loaded from a TOML file with environment-variable
//! overrides.
//!
//! # Precedence
//!
//! For fields that have an environment-variable equivalent
//! (currently only `keeperhub_api_key`):
//!
//! 1. Explicit CLI argument (highest priority — not implemented yet)
//! 2. Environment variable (e.g. `KEEPERHUB_API_KEY`)
//! 3. TOML file value
//! 4. Built-in default (lowest priority)
//!
//! For all other fields, the TOML value wins. If the TOML field is
//! absent, the [`Default::default`] is used.
//!
//! # Example `moltbot.toml`
//!
//! ```toml
//! # 60-second loop tick.
//! tick_interval_seconds = 60
//!
//! # Target network for DeFi operations. Default: "1" (Ethereum mainnet).
//! network = "1"
//!
//! # USDC balance thresholds (in dollars, human-readable).
//! # On balance > park_threshold, the agent supplies to Aave.
//! # On balance < withdraw_threshold, the agent withdraws from Aave.
//! # On balance < safe_mode_threshold, the agent enters safe mode
//! # (no paid actions, no onchain txs).
//! park_threshold_usd = 50.0
//! withdraw_threshold_usd = 20.0
//! safe_mode_threshold_usd = 5.0
//! ```
//!
//! # API key
//!
//! The KeeperHub API key is *not* typically committed to a TOML file
//! (it would be a secret leak). Set it in the environment instead:
//!
//! ```sh
//! export KEEPERHUB_API_KEY=kh_...
//! cargo run -p moltbot
//! ```

use std::path::Path;
use std::time::Duration;
use serde::Deserialize;
use thiserror::Error;

/// Default tick interval (60 seconds).
fn default_tick_interval() -> u64 {
    60
}

/// Default network — Ethereum mainnet (`"1"`).
fn default_network() -> String {
    "1".to_string()
}

/// Default park threshold: $50 USDC.
fn default_park_threshold() -> f64 {
    50.0
}

/// Default withdraw threshold: $20 USDC.
fn default_withdraw_threshold() -> f64 {
    20.0
}

/// Default safe-mode threshold: $5 USDC.
fn default_safe_mode_threshold() -> f64 {
    5.0
}

/// The agent's runtime configuration.
///
/// Built by [`AgentConfig::from_env_and_file`]. All fields except
/// `tick_interval_seconds` and the threshold triples have sensible
/// defaults; the API key is sourced from the environment if not in the
/// TOML file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    /// Seconds between loop ticks. Default: 60.
    #[serde(default = "default_tick_interval")]
    pub tick_interval_seconds: u64,

    /// KeeperHub API key (Bearer token). Sourced from the TOML file
    /// or, if absent, from the `KEEPERHUB_API_KEY` environment variable.
    /// **Required.**
    pub keeperhub_api_key: Option<String>,

    /// Target chain ID as a string (e.g. `"1"` for Ethereum, `"8453"`
    /// for Base). Default: `"1"`.
    #[serde(default = "default_network")]
    pub network: String,

    /// USDC balance (USD) above which the agent supplies to Aave.
    /// Default: 50.0.
    #[serde(default = "default_park_threshold")]
    pub park_threshold_usd: f64,

    /// USDC balance (USD) below which the agent withdraws from Aave.
    /// Default: 20.0.
    #[serde(default = "default_withdraw_threshold")]
    pub withdraw_threshold_usd: f64,

    /// USDC balance (USD) below which the agent enters safe mode
    /// (skips all paid actions and onchain txs). Default: 5.0.
    #[serde(default = "default_safe_mode_threshold")]
    pub safe_mode_threshold_usd: f64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            tick_interval_seconds: default_tick_interval(),
            keeperhub_api_key: None,
            network: default_network(),
            park_threshold_usd: default_park_threshold(),
            withdraw_threshold_usd: default_withdraw_threshold(),
            safe_mode_threshold_usd: default_safe_mode_threshold(),
        }
    }
}

impl AgentConfig {
    /// The tick interval as a [`Duration`].
    pub fn tick_interval(&self) -> Duration {
        Duration::from_secs(self.tick_interval_seconds)
    }

    /// Load a config from `path` (or defaults if `None`) and overlay
    /// the `KEEPERHUB_API_KEY` environment variable when the TOML
    /// doesn't set it.
    pub fn from_env_and_file(path: Option<&Path>) -> Result<Self, ConfigError> {
        let mut config = if let Some(p) = path {
            let text = std::fs::read_to_string(p).map_err(|e| {
                ConfigError::Io(format!("reading {}: {e}", p.display()))
            })?;
            toml::from_str(&text).map_err(|e| {
                ConfigError::Parse(format!("parsing {}: {e}", p.display()))
            })?
        } else {
            Self::default()
        };

        if config.keeperhub_api_key.is_none() {
            config.keeperhub_api_key = std::env::var("KEEPERHUB_API_KEY").ok();
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate the config. Currently checks: tick interval is non-zero,
    /// API key is set, thresholds are non-negative, and
    /// `safe_mode < withdraw < park`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.tick_interval_seconds == 0 {
            return Err(ConfigError::Invalid(
                "tick_interval_seconds must be > 0".to_string(),
            ));
        }
        if self.keeperhub_api_key.as_deref().unwrap_or("").is_empty() {
            return Err(ConfigError::Missing(
                "KEEPERHUB_API_KEY (set in moltbot.toml or as env var)".to_string(),
            ));
        }
        if self.park_threshold_usd < 0.0
            || self.withdraw_threshold_usd < 0.0
            || self.safe_mode_threshold_usd < 0.0
        {
            return Err(ConfigError::Invalid(
                "thresholds must be non-negative".to_string(),
            ));
        }
        if !(self.safe_mode_threshold_usd <= self.withdraw_threshold_usd
            && self.withdraw_threshold_usd <= self.park_threshold_usd)
        {
            return Err(ConfigError::Invalid(format!(
                "threshold order must satisfy safe_mode ({}) <= withdraw ({}) <= park ({})",
                self.safe_mode_threshold_usd,
                self.withdraw_threshold_usd,
                self.park_threshold_usd
            )));
        }
        Ok(())
    }
}

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// I/O error reading the config file.
    #[error("config I/O error: {0}")]
    Io(String),

    /// TOML parse error.
    #[error("config parse error: {0}")]
    Parse(String),

    /// A required field is missing.
    #[error("missing required config: {0}")]
    Missing(String),

    /// A field has an invalid value (out of range, wrong order, etc.).
    #[error("invalid config: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_validates() {
        let c = AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            ..AgentConfig::default()
        };
        c.validate().expect("default config should validate");
    }

    #[test]
    fn empty_api_key_fails_validation() {
        let c = AgentConfig {
            keeperhub_api_key: Some(String::new()),
            ..AgentConfig::default()
        };
        let err = c.validate().unwrap_err();
        assert!(matches!(err, ConfigError::Missing(_)), "got {err:?}");
    }

    #[test]
    fn missing_api_key_fails_validation() {
        let c = AgentConfig::default();
        let err = c.validate().unwrap_err();
        assert!(matches!(err, ConfigError::Missing(_)), "got {err:?}");
    }

    #[test]
    fn zero_tick_interval_fails_validation() {
        let c = AgentConfig {
            tick_interval_seconds: 0,
            keeperhub_api_key: Some("kh_test".to_string()),
            ..AgentConfig::default()
        };
        let err = c.validate().unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
    }

    #[test]
    fn thresholds_in_wrong_order_fail_validation() {
        // safe > withdraw — inverted
        let c = AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            safe_mode_threshold_usd: 50.0,
            withdraw_threshold_usd: 20.0,
            park_threshold_usd: 100.0,
            ..AgentConfig::default()
        };
        let err = c.validate().unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
    }

    #[test]
    fn thresholds_equal_is_allowed() {
        // safe == withdraw == park is a degenerate but valid case.
        let c = AgentConfig {
            keeperhub_api_key: Some("kh_test".to_string()),
            safe_mode_threshold_usd: 10.0,
            withdraw_threshold_usd: 10.0,
            park_threshold_usd: 10.0,
            ..AgentConfig::default()
        };
        c.validate().expect("equal thresholds should be allowed");
    }

    #[test]
    fn tick_interval_returns_duration() {
        let c = AgentConfig {
            tick_interval_seconds: 30,
            ..AgentConfig::default()
        };
        assert_eq!(c.tick_interval(), Duration::from_secs(30));
    }

    #[test]
    fn parse_minimal_toml() {
        let text = r#"
            keeperhub_api_key = "kh_test"
        "#;
        let c: AgentConfig = toml::from_str(text).unwrap();
        assert_eq!(c.tick_interval_seconds, 60); // default
        assert_eq!(c.keeperhub_api_key.as_deref(), Some("kh_test"));
        assert_eq!(c.network, "1"); // default
    }

    #[test]
    fn parse_full_toml() {
        let text = r#"
            tick_interval_seconds = 30
            keeperhub_api_key = "kh_test"
            network = "8453"
            park_threshold_usd = 100.0
            withdraw_threshold_usd = 40.0
            safe_mode_threshold_usd = 10.0
        "#;
        let c: AgentConfig = toml::from_str(text).unwrap();
        assert_eq!(c.tick_interval_seconds, 30);
        assert_eq!(c.network, "8453");
        assert_eq!(c.park_threshold_usd, 100.0);
        assert_eq!(c.withdraw_threshold_usd, 40.0);
        assert_eq!(c.safe_mode_threshold_usd, 10.0);
    }

    #[test]
    fn parse_rejects_unknown_fields() {
        let text = r#"
            keeperhub_api_key = "kh_test"
            bogus_field = true
        "#;
        let err = toml::from_str::<AgentConfig>(text).unwrap_err();
        assert!(err.to_string().contains("bogus_field"));
    }
}
