//! x402 payment support for the KeeperHub Rust client.
//!
//! When an agent calls a paid KeeperHub workflow, the server returns an
//! HTTP 402 with a payment challenge body. This module implements the
//! client side of the x402 protocol: parse the challenge, build an
//! EIP-3009 [`TransferWithAuthorization`](https://eips.ethereum.org/EIPS/eip-3009)
//! typed-data message, and (when paired with a signing backend) sign it
//! so the facilitator can settle the payment onchain.
//!
//! # Why this module exists
//!
//! The KeeperHub agentic wallet (npm package) handles 402 auto-pay via
//! a PreToolUse hook for Node-based agents. For a Rust binary, we need
//! our own implementation. This module provides the building blocks;
//! plugging in a signer (e.g. via `alloy-rs` or the KeeperHub server-side
//! signing proxy) is the next step.
//!
//! # Payment protocols supported
//!
//! - **x402 on Base (chain 8453):** USDC (`0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`)
//! - **MPP on Tempo mainnet (chain 4217):** USDC.e (`0x20C000000000000000000000B9537D11c60E8b50`)
//! - **x402 on Tempo testnet (chain 42431):** USDC.e (same contract)

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// The default Base USDC contract address (used by x402 on Base mainnet).
pub const BASE_USDC_ADDRESS: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

/// The default Tempo USDC.e contract address (used by MPP and x402 on Tempo).
pub const TEMPO_USDC_E_ADDRESS: &str = "0x20C000000000000000000000B9537D11c60E8b50";

/// The Base mainnet chain ID.
pub const CHAIN_ID_BASE: u64 = 8453;

/// The Tempo mainnet chain ID.
pub const CHAIN_ID_TEMPO: u64 = 4217;

/// The Tempo testnet chain ID.
pub const CHAIN_ID_TEMPO_TESTNET: u64 = 42431;

/// The payment protocol used by a 402 challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentProtocol {
    /// HTTP 402 + EIP-3009 `TransferWithAuthorization` on Base.
    X402,
    /// MPP on Tempo (Tempo's native payment protocol).
    Mpp,
}

/// A parsed x402 / MPP 402 challenge body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentChallenge {
    /// The payment protocol.
    pub protocol: PaymentProtocol,

    /// The amount to pay, in atomic units of the asset (e.g. `50000` = $0.05 USDC).
    pub amount: String,

    /// The asset contract address.
    pub asset: String,

    /// The chain ID where the payment should settle.
    pub chain_id: u64,

    /// The facilitator's address that will receive the funds.
    pub pay_to: String,

    /// A unique nonce for this payment.
    pub nonce: String,

    /// Unix timestamp after which the authorization expires.
    pub valid_after: u64,

    /// Unix timestamp before which the authorization is valid.
    pub valid_before: u64,

    /// Optional resource identifier (the workflow being paid for).
    #[serde(default)]
    pub resource: Option<String>,
}

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
/// **Status:** not yet fully implemented. The shape of the challenge
/// body may vary by protocol; the real parser will be added once we
/// have a sample challenge to test against.
pub fn parse_challenge(body: &str) -> Result<PaymentChallenge> {
    serde_json::from_str(body).map_err(Error::from)
}

/// Build the `X-PAYMENT` header value from a signed authorization.
///
/// The exact wire format depends on the protocol. For x402, it's a
/// base64-encoded JSON object containing the signature and authorization.
/// The full implementation lands in the next phase.
pub fn build_payment_header(_auth: &TransferWithAuthorization, _signature: &str) -> Result<String> {
    Err(Error::Internal(
        "build_payment_header not yet implemented".to_string(),
    ))
}
