use super::lexicon::LayerKind;

#[derive(Debug, Clone)]
pub struct AstLayer {
    pub kind: LayerKind,
    pub source: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SttpAst {
    pub provenance: Option<AstLayer>,
    pub envelope: Option<AstLayer>,
    pub content: Option<AstLayer>,
    pub metrics: Option<AstLayer>,
    pub strict_spine: bool,
}

impl SttpAst {
    pub fn with_strict_spine(mut self, strict_spine: bool) -> Self {
        self.strict_spine = strict_spine;
        self
    }
}
