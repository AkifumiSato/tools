//! A generic library for lossless syntax trees.
//! See `examples/s_expressions.rs` for a tutorial.
#![forbid(
    // missing_debug_implementations,
    unconditional_recursion,
    future_incompatible,
    // missing_docs,
)]
#![deny(unsafe_code)]
#![deny(rustdoc::broken_intra_doc_links)]

#[doc(hidden)]
pub mod macros;

#[allow(unsafe_code)]
pub mod cursor;
#[allow(unsafe_code)]
mod green;

pub mod syntax;
mod syntax_node_text;
mod utility_types;

#[allow(unsafe_code)]
mod arc;
mod ast;
mod cow_mut;
pub mod raw_language;
#[cfg(feature = "serde")]
mod serde_impls;
mod syntax_factory;
mod syntax_rewriter;
mod syntax_token_text;
mod tree_builder;

pub use rome_text_size::{TextLen, TextRange, TextSize};
#[cfg(feature = "serde")]
pub use serde_impls::TextRangeSchema;

pub use crate::{
    ast::*,
    green::RawSyntaxKind,
    syntax::{
        Language, SendNode, SyntaxElement, SyntaxElementChildren, SyntaxKind, SyntaxList,
        SyntaxNode, SyntaxNodeChildren, SyntaxSlot, SyntaxToken, SyntaxTriviaPiece,
        SyntaxTriviaPieceComments, TriviaPiece, TriviaPieceKind,
    },
    syntax_factory::*,
    syntax_node_text::SyntaxNodeText,
    syntax_rewriter::{SyntaxRewriter, VisitNodeSignal},
    syntax_token_text::SyntaxTokenText,
    tree_builder::{Checkpoint, TreeBuilder},
    utility_types::{Direction, NodeOrToken, TokenAtOffset, WalkEvent},
};

pub(crate) use crate::green::{GreenNode, GreenNodeData, GreenToken, GreenTokenData};

pub fn check_live() -> Option<String> {
    if cursor::has_live() || green::has_live() {
        Some(countme::get_all().to_string())
    } else {
        None
    }
}
