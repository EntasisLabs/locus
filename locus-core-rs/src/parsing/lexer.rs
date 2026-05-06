use super::lexicon::{
    CONTENT_MARKER, ENVELOPE_MARKER, LAYER_STOP_MARKER, METRICS_MARKER, PROVENANCE_MARKER,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    ProvenanceStart,
    EnvelopeStart,
    ContentStart,
    MetricsStart,
    LayerEnd,
    LBrace,
    RBrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    let mut index = 0usize;
    let mut line = 1usize;
    let mut column = 1usize;

    while index < input.len() {
        let rest = &input[index..];

        if let Some((kind, marker)) = match_structural_marker(rest) {
            let len = marker.len();
            tokens.push(Token {
                kind,
                span: Span {
                    start: index,
                    end: index + len,
                    line,
                    column,
                },
            });

            advance_position(marker, &mut line, &mut column);
            index += len;
            continue;
        }

        let Some(ch) = rest.chars().next() else {
            break;
        };

        let ch_len = ch.len_utf8();
        match ch {
            '{' => tokens.push(Token {
                kind: TokenKind::LBrace,
                span: Span {
                    start: index,
                    end: index + ch_len,
                    line,
                    column,
                },
            }),
            '}' => tokens.push(Token {
                kind: TokenKind::RBrace,
                span: Span {
                    start: index,
                    end: index + ch_len,
                    line,
                    column,
                },
            }),
            _ => {}
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
        index += ch_len;
    }

    tokens
}

fn match_structural_marker(rest: &str) -> Option<(TokenKind, &'static str)> {
    if rest.starts_with(PROVENANCE_MARKER) {
        return Some((TokenKind::ProvenanceStart, PROVENANCE_MARKER));
    }
    if rest.starts_with(ENVELOPE_MARKER) {
        return Some((TokenKind::EnvelopeStart, ENVELOPE_MARKER));
    }
    if rest.starts_with(CONTENT_MARKER) {
        return Some((TokenKind::ContentStart, CONTENT_MARKER));
    }
    if rest.starts_with(METRICS_MARKER) {
        return Some((TokenKind::MetricsStart, METRICS_MARKER));
    }
    if rest.starts_with(LAYER_STOP_MARKER) {
        return Some((TokenKind::LayerEnd, LAYER_STOP_MARKER));
    }

    None
}

fn advance_position(text: &str, line: &mut usize, column: &mut usize) {
    for ch in text.chars() {
        if ch == '\n' {
            *line += 1;
            *column = 1;
        } else {
            *column += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_tokenize_structural_markers() {
        let raw = "⊕⟨ { a: 1 } ⟩\n⦿⟨ { b: 2 } ⟩\n◈⟨ { c(.9): x } ⟩\n⍉⟨ { rho: 1 } ⟩";
        let tokens = tokenize(raw);

        assert!(tokens.iter().any(|t| t.kind == TokenKind::ProvenanceStart));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::EnvelopeStart));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::ContentStart));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::MetricsStart));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::LayerEnd));
    }

    #[test]
    fn should_track_line_and_column() {
        let raw = "x\n⊕⟨ { a: 1 } ⟩";
        let tokens = tokenize(raw);
        let provenance = tokens
            .iter()
            .find(|t| t.kind == TokenKind::ProvenanceStart)
            .expect("provenance marker should exist");

        assert_eq!(provenance.span.line, 2);
        assert_eq!(provenance.span.column, 1);
    }
}
