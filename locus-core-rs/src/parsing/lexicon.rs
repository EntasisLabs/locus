pub const PROVENANCE_MARKER: &str = "⊕⟨";
pub const ENVELOPE_MARKER: &str = "⦿⟨";
pub const CONTENT_MARKER: &str = "◈⟨";
pub const METRICS_MARKER: &str = "⍉⟨";
pub const LAYER_STOP_MARKER: &str = "⟩";

pub const AVEC_USER_KEY: &str = "user_avec";
pub const AVEC_MODEL_KEY: &str = "model_avec";
pub const AVEC_COMPRESSION_KEY: &str = "compression_avec";

pub const AVEC_DIMENSION_KEYS: [&str; 4] = ["stability", "friction", "logic", "autonomy"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Provenance,
    Envelope,
    Content,
    Metrics,
}

impl LayerKind {
    pub fn marker(self) -> &'static str {
        match self {
            Self::Provenance => PROVENANCE_MARKER,
            Self::Envelope => ENVELOPE_MARKER,
            Self::Content => CONTENT_MARKER,
            Self::Metrics => METRICS_MARKER,
        }
    }
}

pub const LAYER_ORDER: [LayerKind; 4] = [
    LayerKind::Provenance,
    LayerKind::Envelope,
    LayerKind::Content,
    LayerKind::Metrics,
];
