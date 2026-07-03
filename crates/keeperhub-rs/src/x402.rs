//! x402 payment support for the KeeperHub Rust client.
//!
//! When an agent calls a paid KeeperHub workflow, the server returns a
//! 402 response with a payment challenge body. This module provides the
//! types and parsers for the x402 / MPP protocols.
//!
//! # Two-layer design
//!
//! For most use cases, you should NOT call `build_payment_header`
//! directly. Instead, use the KeeperHub agentic wallet's MCP server
//! (`mcp__plugin_keeperhub_wallet__call_workflow`), which handles 402
//! auto-pay transparently. This module exists for two cases:
//!
//! 1. **Parsing the 402 challenge** so the caller can log / display
//!    the price, asset, and recipient — surfaced via
//!    [`crate::error::Error::X402Unpaid`].
//! 2. **Self-custody signing** (post-hackathon) where the Rust binary
//!    holds the key material itself. This requires `alloy-rs` and is
//!    out of scope for the hackathon build.
//!
//! # Payment protocols supported
//!
//! - **x402 on Base (chain 8453):** USDC (`0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`)
//! - **MPP on Tempo mainnet (chain 4217):** USDC.e (`0x20c000000000000000000000b9537d11c60e8b50`)
//! - **x402 on Tempo testnet (chain 42431):** USDC.e (same contract)

use crate::error::{Error, Result};
pub use crate::types::{PaymentChallenge, PaymentProtocol};
use serde::{Deserialize, Serialize};

/// The default Base USDC contract address (used by x402 on Base mainnet).
pub const BASE_USDC_ADDRESS: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

/// The default Tempo USDC.e contract address (used by MPP and x402 on Tempo).
pub const TEMPO_USDC_E_ADDRESS: &str = "0x20c000000000000000000000b9537d11c60e8b50";

/// The Base mainnet chain ID.
pub const CHAIN_ID_BASE: u64 = 8453;

/// The Tempo mainnet chain ID.
pub const CHAIN_ID_TEMPO: u64 = 4217;

/// The Tempo testnet chain ID.
pub const CHAIN_ID_TEMPO_TESTNET: u64 = 42431;

// Re-export the shared types for convenience.
// (PaymentChallenge and PaymentProtocol are already re-exported above via `pub use`.)

/// An EIP-3009 `TransferWithAuthorization` typed-data payload, ready to be
/// signed by a wallet.
///
/// Signing this with the user's EIP-712 signer and POSTing it back to the
/// facilitator in the `X-PAYMENT` header settles the payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferWithAuthorization {
    /// The token contract address.
    pub token: String,

    /// The amount in atomic units.
    pub value: String,

    /// The address paying (the user's wallet).
    pub from: String,

    /// The address receiving (the facilitator).
    pub to: String,

    /// The authorization's validity window start (unix seconds).
    pub valid_after: String,

    /// The authorization's validity window end (unix seconds).
    pub valid_before: String,

    /// Unique nonce for this authorization.
    pub nonce: String,
}

impl PaymentChallenge {
    /// Build a `TransferWithAuthorization` payload from this challenge,
    /// for the given payer address.
    ///
    /// The result still needs to be EIP-712 signed. Once signed, the
    /// signature is sent in the `X-PAYMENT` header on retry.
    pub fn to_transfer_with_authorization(&self, payer: impl AsRef<str>) -> TransferWithAuthorization {
        TransferWithAuthorization {
            token: self.asset.clone(),
            value: self.amount.clone(),
            from: payer.as_ref().to_string(),
            to: self.pay_to.clone(),
            valid_after: self.valid_after.to_string(),
            valid_before: self.valid_before.to_string(),
            nonce: self.nonce.clone(),
        }
    }
}

/// Parse a 402 challenge body from raw JSON.
///
/// `body` should be the JSON portion of a 402 response — e.g. the part
/// after `"API call failed: 402 Payment Required - "` in the MCP tool
/// error text. Returns the structured [`PaymentChallenge`].
pub fn parse_challenge(body: &str) -> Result<PaymentChallenge> {
    serde_json::from_str(body).map_err(|e| {
        Error::Internal(format!("parse_challenge: invalid JSON: {e}; body: {body}"))
    })
}

/// Build the `X-PAYMENT` header value from a signed authorization.
///
/// The exact wire format depends on the protocol. For x402, it's a
/// base64-encoded JSON object containing the signature and authorization.
///
/// **Status:** not yet implemented. Per the [project plan](https://github.com/AduAkorful/moltbot/blob/main/plans/next-steps.md#5),
/// the hackathon build delegates auto-pay to the KeeperHub agentic wallet's
/// MCP server instead of reimplementing EIP-3009 here. A real implementation
/// is out of scope for the hackathon; this is a future-only stub.
pub fn build_payment_header(_auth: &TransferWithAuthorization, _signature: &str) -> Result<String> {
    Err(Error::Internal(
        "build_payment_header not yet implemented — use the agentic wallet MCP \
         (mcp__plugin_keeperhub_wallet__call_workflow) for auto-pay."
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PaymentChallenge, PaymentProtocol};

    #[test]
    fn parse_challenge_round_trips() {
        let body = r#"{
            "protocol": "x402",
            "amount": "10000",
            "asset": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "chainId": 8453,
            "payTo": "0xAbC0000000000000000000000000000000000ff",
            "nonce": "0x1234",
            "validAfter": 0,
            "validBefore": 9999999999,
            "resource": "my-workflow"
        }"#;
        let c: PaymentChallenge = parse_challenge(body).unwrap();
        assert_eq!(c.protocol, PaymentProtocol::X402);
        assert_eq!(c.amount, "10000");
        assert_eq!(c.chain_id, 8453);
        assert_eq!(c.resource.as_deref(), Some("my-workflow"));
    }

    #[test]
    fn to_transfer_with_authorization_copies_fields() {
        let c = PaymentChallenge {
            protocol: PaymentProtocol::Mpp,
            amount: "50000".into(),
            asset: TEMPO_USDC_E_ADDRESS.into(),
            chain_id: CHAIN_ID_TEMPO,
            pay_to: "0xRecipient".into(),
            nonce: "0xn".into(),
            valid_after: 0,
            valid_before: 100,
            resource: None,
        };
        let t = c.to_transfer_with_authorization("0xPayer");
        assert_eq!(t.token, TEMPO_USDC_E_ADDRESS);
        assert_eq!(t.value, "50000");
        assert_eq!(t.from, "0xPayer");
        assert_eq!(t.to, "0xRecipient");
    }
}
