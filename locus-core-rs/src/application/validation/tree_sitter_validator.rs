use crate::domain::contracts::NodeValidator;
use crate::domain::models::{AvecState, SttpNode, ValidationFailureReason, ValidationResult};

const REQUIRED_LAYERS: [&str; 4] = ["⊕⟨", "⦿⟨", "◈⟨", "⍉⟨"];
const VALID_TIERS: [&str; 6] = ["raw", "daily", "weekly", "monthly", "quarterly", "yearly"];

#[derive(Debug, Default, Clone, Copy)]
pub struct TreeSitterValidator;

impl TreeSitterValidator {
    /// Create a new validator instance.
    pub fn new() -> Self {
        Self
    }
}

impl NodeValidator for TreeSitterValidator {
    fn validate(&self, raw_node: &str) -> ValidationResult {
        if raw_node.trim().is_empty() {
            return ValidationResult::fail("Node is empty", ValidationFailureReason::ParseFailure);
        }

        for layer in REQUIRED_LAYERS {
            if !raw_node.contains(layer) {
                return ValidationResult::fail(
                    format!("Missing required layer: {layer}"),
                    ValidationFailureReason::MissingLayer,
                );
            }
        }

        let prov_idx = raw_node.find("⊕⟨").unwrap_or(usize::MAX);
        let env_idx = raw_node.find("⦿⟨").unwrap_or(usize::MAX);
        let con_idx = raw_node.find("◈⟨").unwrap_or(usize::MAX);
        let met_idx = raw_node.find("⍉⟨").unwrap_or(usize::MAX);

        if !(prov_idx < env_idx && env_idx < con_idx && con_idx < met_idx) {
            return ValidationResult::fail(
                "Layer order violation - required: ⊕ -> ⦿ -> ◈ -> ⍉",
                ValidationFailureReason::SchemaViolation,
            );
        }

        let lower = raw_node.to_ascii_lowercase();
        let tier_valid = VALID_TIERS
            .iter()
            .any(|t| lower.contains(&format!("tier: {t}")));
        if !tier_valid {
            return ValidationResult::fail(
                format!("Invalid tier - expected one of: {}", VALID_TIERS.join("|")),
                ValidationFailureReason::InvalidTier,
            );
        }

        if let Some(content_block) = extract_content_block(raw_node) {
            let depth = max_nesting_depth(content_block);
            if depth > 5 {
                return ValidationResult::fail(
                    format!("Content nesting depth {depth} exceeds maximum of 5"),
                    ValidationFailureReason::NestingDepth,
                );
            }
        }

        ValidationResult::ok()
    }

    fn verify_psi(&self, node: &SttpNode) -> bool {
        let computed = node.compression_avec.unwrap_or_else(AvecState::zero).psi();
        (computed - node.psi).abs() < 0.01
    }
}

fn extract_content_block(raw: &str) -> Option<&str> {
    let start = raw.find("◈⟨")?;
    let end = raw.find("⍉⟨")?;
    if end <= start {
        None
    } else {
        Some(&raw[start..end])
    }
}

fn max_nesting_depth(block: &str) -> usize {
    let mut max_depth = 0usize;
    let mut depth = 0usize;

    for c in block.chars() {
        if c == '{' {
            depth += 1;
            max_depth = max_depth.max(depth);
        } else if c == '}' {
            depth = depth.saturating_sub(1);
        }
    }

    max_depth
}
