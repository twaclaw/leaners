//! The IR between the parser and the renderer. This type *is* the trust
//! boundary.
//!
//! There is deliberately **no raw-HTML variant**. CommonMark allows inline HTML;
//! modelling it here and emitting it verbatim would make the security theorem
//! false and would need a verified sanitiser instead. Dropping the feature makes
//! the property fall out of the type definition: nothing in an `Ast` can carry
//! markup through to the output.
//!
//! The shape is a flat event stream, not a tree. Emphasis, links, quotes and
//! lists are open/close marker pairs in the surrounding `Vec` rather than nodes
//! holding their children. An earlier tree-shaped `Ast` recursed through `Vec`,
//! and Lean's kernel rejects the nested inductive aeneas makes of that, so the
//! tree could not be extracted; the flat events can. The price is that "every
//! open marker has a matching close" is no longer true by construction. It is
//! an invariant of the streams `adapt` produces, and the tag-balance theorem is
//! stated under that hypothesis. The security property does not pay it: marker
//! events map to fixed literals whatever the stream looks like.

pub enum Inline {
    Text(Vec<u8>),
    Code(Vec<u8>),
    /// display, source. Held verbatim: nothing here typesets TeX, so the source
    /// is escaped and emitted for CSS to style. See notes/software-verification.
    Math(bool, Vec<u8>),
    EmphOpen,
    EmphClose,
    StrongOpen,
    StrongClose,
    /// url. The body is the events up to the matching `LinkClose`.
    LinkOpen(Vec<u8>),
    LinkClose,
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
    QuoteOpen,
    QuoteClose,
    /// ordered. The close repeats the flag so the renderer can pick the closing
    /// tag from the event alone instead of carrying a stack.
    ListOpen(bool),
    ListClose(bool),
    ItemOpen,
    ItemClose,
    /// rows of cells, each cell a run of inlines. The first row is the header.
    /// Nested `Vec` is harmless here: `Inline` does not recurse, so this is a
    /// plain type application rather than a nested inductive.
    Table(Vec<Vec<Vec<Inline>>>),
    Rule,
}
