//! The IR between the parser and the renderer. This type *is* the trust
//! boundary.
//!
//! There is deliberately **no raw-HTML variant**. CommonMark allows inline HTML;
//! modelling it here and emitting it verbatim would make the security theorem
//! false and would need a verified sanitiser instead. Dropping the feature makes
//! the property fall out of the type definition: nothing in an `Ast` can carry
//! markup through to the output.
//!
//! Not extractable as written. The recursive variants recurse through `Vec`, and
//! Lean's kernel rejects the nested inductive that produces, so `make extract`
//! skips this module. Recursing through `Box` instead would lift that.

pub enum Inline {
    Text(Vec<u8>),
    Code(Vec<u8>),
    /// display, source. Held verbatim: nothing here typesets TeX, so the source
    /// is escaped and emitted for CSS to style. See notes/software-verification.
    Math(bool, Vec<u8>),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    /// url, body
    Link(Vec<u8>, Vec<Inline>),
    /// url, alt
    Image(Vec<u8>, Vec<u8>),
    SoftBreak,
    HardBreak,
}

pub enum Block {
    Paragraph(Vec<Inline>),
    /// Inline content with no block wrapper, as in a tight list item.
    Inlines(Vec<Inline>),
    /// level 1..=6, body
    Heading(u8, Vec<Inline>),
    /// language, text
    Code(Vec<u8>, Vec<u8>),
    Quote(Vec<Block>),
    /// ordered, items
    List(bool, Vec<Vec<Block>>),
    /// rows of cells, each cell a run of inlines. The first row is the header.
    /// Nested `Vec` rather than a struct so the shape survives extraction.
    Table(Vec<Vec<Vec<Inline>>>),
    Rule,
}
