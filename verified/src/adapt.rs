//! Ordinary Rust, and the half that is **never** verified. This is the
//! compiler frontend: it turns Markdown into the `Ast`, using a mature
//! off-the-shelf parser rather than one we would have to prove things about.
//!
//! Raw HTML events are dropped here. That is the design decision that keeps
//! the renderer's security theorem true: no markup from the input can reach
//! the `Ast`, so none can reach the output.
//!
//! The `Ast` is a flat event stream, so this module also owns its balance
//! invariant: every open marker pushed below is followed by its matching close
//! on the same path, unconditionally, which is the well-formedness hypothesis
//! the tag-balance theorem is stated under.

use crate::ast::{Block, Inline};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn parse(src: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_MATH);
    let events: Vec<Event> = Parser::new_ext(src, options).collect();
    let mut i = 0usize;
    let mut out = Vec::new();
    blocks(&events, &mut i, None, &mut out);
    out
}

fn blocks(ev: &[Event], i: &mut usize, stop: Option<TagEnd>, out: &mut Vec<Block>) {
    while *i < ev.len() {
        if let Event::End(end) = &ev[*i] {
            if Some(*end) == stop {
                *i += 1;
                return;
            }
            // An end tag we are not waiting for: skip it.
            *i += 1;
            continue;
        }
        if let Event::Start(tag) = &ev[*i]
            && !is_inline_start(tag)
        {
            let tag = tag.clone();
            *i += 1;
            push_block(ev, i, tag, out);
            continue;
        }
        if let Event::Rule = &ev[*i] {
            *i += 1;
            out.push(Block::Rule);
            continue;
        }

        // A tight list item holds inline content directly, with no paragraph
        // around it. Gather the run rather than dropping it.
        let before = *i;
        let mut run = Vec::new();
        while *i < ev.len() && push_inline(ev, i, &mut run) {}
        if !run.is_empty() {
            out.push(Block::Inlines(run));
        }
        if *i == before {
            *i += 1; // nothing consumed: guarantee progress
        }
    }
}

fn is_inline_start(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

fn push_block(ev: &[Event], i: &mut usize, tag: Tag, out: &mut Vec<Block>) {
    match tag {
        Tag::Paragraph => out.push(Block::Paragraph(inlines(ev, i, TagEnd::Paragraph))),
        Tag::Heading { level, .. } => {
            let n = level as u8;
            out.push(Block::Heading(n, inlines(ev, i, TagEnd::Heading(level))));
        }
        Tag::CodeBlock(kind) => {
            let lang = match &kind {
                CodeBlockKind::Fenced(l) => l.as_bytes().to_vec(),
                CodeBlockKind::Indented => Vec::new(),
            };
            let mut text = Vec::new();
            while *i < ev.len() {
                match &ev[*i] {
                    Event::End(TagEnd::CodeBlock) => {
                        *i += 1;
                        break;
                    }
                    Event::Text(t) => {
                        text.extend_from_slice(t.as_bytes());
                        *i += 1;
                    }
                    _ => *i += 1,
                }
            }
            out.push(Block::Code(lang, text));
        }
        Tag::BlockQuote(_) => {
            out.push(Block::QuoteOpen);
            blocks(ev, i, Some(TagEnd::BlockQuote(None)), out);
            out.push(Block::QuoteClose);
        }
        Tag::List(start) => {
            let ordered = start.is_some();
            out.push(Block::ListOpen(ordered));
            while *i < ev.len() {
                match &ev[*i] {
                    Event::End(TagEnd::List(_)) => {
                        *i += 1;
                        break;
                    }
                    Event::Start(Tag::Item) => {
                        *i += 1;
                        out.push(Block::ItemOpen);
                        blocks(ev, i, Some(TagEnd::Item), out);
                        out.push(Block::ItemClose);
                    }
                    _ => *i += 1,
                }
            }
            out.push(Block::ListClose(ordered));
        }
        Tag::Table(_) => {
            let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
            while *i < ev.len() {
                match &ev[*i] {
                    Event::End(TagEnd::Table) => {
                        *i += 1;
                        break;
                    }
                    Event::Start(Tag::TableHead) => {
                        *i += 1;
                        rows.push(cells(ev, i, TagEnd::TableHead));
                    }
                    Event::Start(Tag::TableRow) => {
                        *i += 1;
                        rows.push(cells(ev, i, TagEnd::TableRow));
                    }
                    _ => *i += 1,
                }
            }
            out.push(Block::Table(rows));
        }
        // Footnotes and anything else we do not model are skipped
        // wholesale rather than half-rendered.
        _ => {}
    }
}

fn inlines(ev: &[Event], i: &mut usize, stop: TagEnd) -> Vec<Inline> {
    let mut out = Vec::new();
    gather(ev, i, stop, &mut out);
    out
}

/// Appends inline events until `stop`, consuming the stop tag.
fn gather(ev: &[Event], i: &mut usize, stop: TagEnd, out: &mut Vec<Inline>) {
    while *i < ev.len() {
        if let Event::End(end) = &ev[*i] {
            if *end == stop {
                *i += 1;
                return;
            }
            *i += 1;
            continue;
        }
        // Event::Html and Event::InlineHtml are not inline events here, so
        // they fall through and are dropped.
        if !push_inline(ev, i, out) {
            *i += 1;
        }
    }
}

/// Consumes one inline event. Returns false, consuming nothing, if the event
/// at `i` is not inline content.
fn push_inline(ev: &[Event], i: &mut usize, out: &mut Vec<Inline>) -> bool {
    match &ev[*i] {
        Event::Text(t) => {
            out.push(Inline::Text(t.as_bytes().to_vec()));
            *i += 1;
            true
        }
        Event::Code(t) => {
            out.push(Inline::Code(t.as_bytes().to_vec()));
            *i += 1;
            true
        }
        Event::InlineMath(t) => {
            out.push(Inline::Math(false, t.as_bytes().to_vec()));
            *i += 1;
            true
        }
        Event::DisplayMath(t) => {
            out.push(Inline::Math(true, t.as_bytes().to_vec()));
            *i += 1;
            true
        }
        Event::SoftBreak => {
            out.push(Inline::SoftBreak);
            *i += 1;
            true
        }
        Event::HardBreak => {
            out.push(Inline::HardBreak);
            *i += 1;
            true
        }
        Event::Start(tag) if is_inline_start(tag) => {
            let tag = tag.clone();
            *i += 1;
            match tag {
                Tag::Emphasis => {
                    out.push(Inline::EmphOpen);
                    gather(ev, i, TagEnd::Emphasis, out);
                    out.push(Inline::EmphClose);
                }
                Tag::Strong => {
                    out.push(Inline::StrongOpen);
                    gather(ev, i, TagEnd::Strong, out);
                    out.push(Inline::StrongClose);
                }
                Tag::Strikethrough => {
                    // Not modelled as its own markers; keep the body events.
                    gather(ev, i, TagEnd::Strikethrough, out);
                }
                Tag::Link { dest_url, .. } => {
                    out.push(Inline::LinkOpen(dest_url.as_bytes().to_vec()));
                    gather(ev, i, TagEnd::Link, out);
                    out.push(Inline::LinkClose);
                }
                Tag::Image { dest_url, .. } => {
                    let mut alt = Vec::new();
                    gather(ev, i, TagEnd::Image, &mut alt);
                    out.push(Inline::Image(dest_url.as_bytes().to_vec(), flatten(&alt)));
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}

/// The cells of one table row.
fn cells(ev: &[Event], i: &mut usize, stop: TagEnd) -> Vec<Vec<Inline>> {
    let mut out = Vec::new();
    while *i < ev.len() {
        match &ev[*i] {
            Event::End(end) if *end == stop => {
                *i += 1;
                return out;
            }
            Event::Start(Tag::TableCell) => {
                *i += 1;
                out.push(inlines(ev, i, TagEnd::TableCell));
            }
            _ => *i += 1,
        }
    }
    out
}

/// Plain text of an event run, for image alt text. Markers contribute nothing;
/// the text between them is already flat in the stream.
fn flatten(inlines: &[Inline]) -> Vec<u8> {
    let mut out = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) | Inline::Code(t) => out.extend_from_slice(t),
            Inline::Math(_, t) => out.extend_from_slice(t),
            Inline::Image(_, a) => out.extend_from_slice(a),
            Inline::SoftBreak | Inline::HardBreak => out.push(b' '),
            Inline::EmphOpen
            | Inline::EmphClose
            | Inline::StrongOpen
            | Inline::StrongClose
            | Inline::LinkOpen(_)
            | Inline::LinkClose => {}
        }
    }
    out
}
