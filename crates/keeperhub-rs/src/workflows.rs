//! Pre-built workflow templates for the KeeperHub marketplace.
//!
//! These builders return a `(name, description, nodes, edges)` tuple
//! ready to feed into
//! [`crate::mcp::McpClient::create_workflow`]. They encapsulate the
//! node/edge shape and template-reference conventions so callers
//! don't have to hand-author KeeperHub workflow graphs.
//!
//! # Template-reference syntax
//!
//! Workflows reference data from previous nodes via
//! `{{@nodeId:Label.field}}`. The label segment must match the
//! `data.label` of the source node. Trigger inputs are passed to
//! actions via the same syntax — the trigger's label is `"Manual Trigger"`
//! and its input fields are available as `{{@trigger-1:Manual Trigger.<field>}}`.
//!
//! If a template reference is wrong, the visual builder highlights it
//! in red and the workflow fails to validate. Use
//! [`crate::mcp::McpClient::validate_workflow`] (not yet implemented
//! in the Rust client) or click into the workflow in the UI to confirm.
//!
//! # Adding new templates
//!
//! Drop a new public function in this module that returns the same
//! `(name, description, Vec<Node>, Vec<Edge>)` tuple. Keep IDs
//! (`trigger-1`, `node-1`, ...) stable for the lifetime of the
//! workflow so updates via `update_workflow` work as full replaces.

use crate::types::{Edge, Node, NodeData, NodePosition};
use serde_json::{json, Value};

/// The (name, description, nodes, edges) tuple returned by every
/// template builder in this module.
///
/// Use this as the argument tuple for
/// [`crate::mcp::McpClient::create_workflow`].
pub type WorkflowBlueprint<'a> = (
    &'a str,        // name
    Option<&'a str>, // description
    Vec<Node>,
    Vec<Edge>,
);

/// Build the "Aave V3 Portfolio Risk Check" workflow.
///
/// # What it does
///
/// A single-action read-only workflow: given an input `wallet` address,
/// calls `aave-v3/get-user-account-data` on Ethereum mainnet and
/// returns the user's Aave V3 health factor, total collateral, total
/// debt, and LTV. No wallet required, no onchain transaction, no
/// payment for the action itself — but the marketplace workflow call
/// can be priced via `set_listing_price` to recover KeeperHub's
/// x402 cut.
///
/// # Inputs
///
/// - `wallet` (required, string) — EVM address to query
///
/// # Outputs
///
/// The full output of `aave-v3/get-user-account-data`:
/// `totalCollateralBase`, `totalDebtBase`, `availableBorrowsBase`,
/// `currentLiquidationThreshold`, `ltv`, `healthFactor`
/// (all in 8-decimal USD except `healthFactor` which is 18-decimal
/// WAD — divide by 1e18 to get the human factor).
///
/// # Graph
///
/// ```text
/// trigger-1 (Manual)
///   │
///   ▼
/// node-1 (aave-v3/get-user-account-data, network=1)
/// ```
pub fn aave_v3_risk_check() -> WorkflowBlueprint<'static> {
    let trigger = Node {
        id: "trigger-1".to_string(),
        node_type: Some("trigger".to_string()),
        position: Some(NodePosition { x: 0.0, y: 0.0 }),
        data: NodeData {
            data_type: Some("trigger".to_string()),
            label: Some("Manual Trigger".to_string()),
            config: json!({ "triggerType": "Manual" }),
            status: None,
            description: Some("Run manually with a wallet address".to_string()),
        },
    };

    let aave_action = Node {
        id: "node-1".to_string(),
        node_type: Some("action".to_string()),
        position: Some(NodePosition {
            x: 200.0,
            y: 0.0,
        }),
        data: NodeData {
            data_type: Some("action".to_string()),
            label: Some("Get Aave V3 user account data".to_string()),
            // The action takes `user` (address) and `network` (chain id
            // string). `user` is bound from the trigger's `wallet` input
            // via the template reference; `network` is hard-coded to
            // Ethereum mainnet ("1").
            config: json!({
                "actionType": "aave-v3/get-user-account-data",
                "network": "1",
                "user": "{{@trigger-1:Manual Trigger.wallet}}",
            }),
            status: None,
            description: Some(
                "Read-only: returns health factor, collateral, and debt for the input wallet."
                    .to_string(),
            ),
        },
    };

    let edge = Edge {
        id: "e1".to_string(),
        source: "trigger-1".to_string(),
        target: "node-1".to_string(),
        source_handle: None,
    };

    (
        "Aave V3 Portfolio Risk Check",
        Some(
            "Read-only Aave V3 portfolio risk check on Ethereum mainnet. \
             Pass a wallet address; get back the user's health factor, total \
             collateral, total debt, available borrow power, and LTV. \
             Powered by the Aave V3 plugin — no onchain transaction, no \
             KeeperHub wallet required.",
        ),
        vec![trigger, aave_action],
        vec![edge],
    )
}

/// The marketplace-facing input schema for [`aave_v3_risk_check`].
///
/// Pass this as `opts.input_schema` in
/// [`crate::types::ListWorkflowOptions`] when publishing.
pub fn aave_v3_risk_check_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "wallet": {
                "type": "string",
                "description": "EVM address to query the Aave V3 position for",
                "pattern": "^0x[a-fA-F0-9]{40}$"
            }
        },
        "required": ["wallet"],
        "additionalProperties": false
    })
}

/// The marketplace-facing output mapping for [`aave_v3_risk_check`].
///
/// Exposes the action's full output as `result`, plus a convenience
/// `healthFactor` field for quick alerting use. Pass this as
/// `opts.output_mapping` in [`crate::types::ListWorkflowOptions`].
pub fn aave_v3_risk_check_output_mapping() -> Value {
    json!({
        "result": "{{@node-1:Get Aave V3 user account data.result}}",
        "healthFactor": "{{@node-1:Get Aave V3 user account data.healthFactor}}",
        "totalCollateralBase": "{{@node-1:Get Aave V3 user account data.totalCollateralBase}}",
        "totalDebtBase": "{{@node-1:Get Aave V3 user account data.totalDebtBase}}"
    })
}

/// The recommended marketplace slug for [`aave_v3_risk_check`].
///
/// Slugs are permanent once set. Pick something descriptive and
/// kebab-cased.
pub const AAVE_V3_RISK_CHECK_SLUG: &str = "aave-v3-risk-check";

/// The recommended per-call price (USDC) for [`aave_v3_risk_check`].
///
/// $0.01 is the marketplace minimum and matches the test workflow
/// (`sep-eth-balance-test`) we already use.
pub const AAVE_V3_RISK_CHECK_PRICE: &str = "0.01";

/// The category to set when publishing [`aave_v3_risk_check`].
pub const AAVE_V3_RISK_CHECK_CATEGORY: &str = "defi";

/// The chain ID to set when publishing [`aave_v3_risk_check`].
pub const AAVE_V3_RISK_CHECK_CHAIN: &str = "1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aave_v3_risk_check_has_expected_graph() {
        let (name, description, nodes, edges) = aave_v3_risk_check();
        assert_eq!(name, "Aave V3 Portfolio Risk Check");
        assert!(description.is_some());
        assert_eq!(nodes.len(), 2, "expected 2 nodes (trigger + action)");
        assert_eq!(edges.len(), 1, "expected 1 edge (trigger -> action)");

        let trigger = &nodes[0];
        assert_eq!(trigger.id, "trigger-1");
        assert_eq!(trigger.node_type.as_deref(), Some("trigger"));
        assert_eq!(
            trigger.data.config.get("triggerType").and_then(|v| v.as_str()),
            Some("Manual"),
            "expected Manual trigger"
        );

        let action = &nodes[1];
        assert_eq!(action.id, "node-1");
        assert_eq!(action.node_type.as_deref(), Some("action"));
        assert_eq!(
            action
                .data
                .config
                .get("actionType")
                .and_then(|v| v.as_str()),
            Some("aave-v3/get-user-account-data"),
            "expected aave-v3/get-user-account-data action"
        );
        assert_eq!(
            action.data.config.get("network").and_then(|v| v.as_str()),
            Some("1"),
            "expected network=1 (Ethereum mainnet)"
        );
        let user_ref = action
            .data
            .config
            .get("user")
            .and_then(|v| v.as_str())
            .expect("user template ref must be a string");
        assert!(
            user_ref.contains("trigger-1") && user_ref.contains("wallet"),
            "user ref must point at the trigger's wallet input, got {user_ref:?}"
        );

        assert_eq!(edges[0].source, "trigger-1");
        assert_eq!(edges[0].target, "node-1");
    }

    #[test]
    fn aave_v3_risk_check_input_schema_requires_wallet() {
        let schema = aave_v3_risk_check_input_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"][0], "wallet");
        assert_eq!(schema["properties"]["wallet"]["type"], "string");
    }

    #[test]
    fn aave_v3_risk_check_output_mapping_exposes_key_fields() {
        let mapping = aave_v3_risk_check_output_mapping();
        assert!(mapping.get("result").is_some());
        assert!(mapping.get("healthFactor").is_some());
        assert!(mapping.get("totalCollateralBase").is_some());
        assert!(mapping.get("totalDebtBase").is_some());
    }

    #[test]
    fn aave_v3_risk_check_constants_are_valid() {
        // Slug must be kebab-case, ASCII, and not empty.
        assert!(!AAVE_V3_RISK_CHECK_SLUG.is_empty());
        assert!(AAVE_V3_RISK_CHECK_SLUG
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()));
        assert_eq!(AAVE_V3_RISK_CHECK_CATEGORY, "defi");
        assert_eq!(AAVE_V3_RISK_CHECK_CHAIN, "1");
        // Price parses as a positive number.
        let price: f64 = AAVE_V3_RISK_CHECK_PRICE.parse().unwrap();
        assert!(price > 0.0);
    }

    #[test]
    fn aave_v3_risk_check_graph_round_trips_as_value() {
        // The graph must serialize to a JSON object that the
        // create_workflow MCP tool accepts (a list of node objects +
        // a list of edge objects).
        let (_name, _desc, nodes, edges) = aave_v3_risk_check();
        let nodes_v = serde_json::to_value(&nodes).unwrap();
        let edges_v = serde_json::to_value(&edges).unwrap();
        assert!(nodes_v.is_array());
        assert!(edges_v.is_array());
        assert_eq!(nodes_v.as_array().unwrap().len(), 2);
        assert_eq!(edges_v.as_array().unwrap().len(), 1);
    }
}
