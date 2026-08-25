//! Parsing stack for STTP lexical analysis and node parsing.
//!
//! Also owns the write-side document builder (canonical render counterpart).

pub mod ast;
pub mod document_builder;
pub mod lexer;
pub mod lexicon;
pub mod state_machine;
pub mod sttp_node_parser;

pub use document_builder::{
    SttpContentSlice, SttpDocument, SttpDocumentBuildError, SttpDocumentBuilder,
    SttpDocumentMetadata,
};
pub use sttp_node_parser::SttpNodeParser;
