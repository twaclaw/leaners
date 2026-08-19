//! Ordinary Rust, and the half that is **never** verified. This is the
//! compiler frontend: it turns Markdown into the `Ast`, using a mature
//! off-the-shelf parser rather than one we would have to prove things about.
//!
//! Raw HTML events are dropped here. That is the design decision that keeps
//! the renderer's security theorem true: no markup from the input can reach
//! the `Ast`, so none can reach the output.

use crate::ast::{Block, Inline};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn parse(src: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_MATH);
    let events: Vec<Event> = Parser::new_ext(src, options).collect();
    let mut i = 0usize;
    blocks(&events, &mut i, None)
}

fn blocks(ev: &[Event], i: &mut usize, stop: Option<TagEnd>) -> Vec<Block> {
    let mut out = Vec::new();
    while *i < ev.len() {
        if let Event::End(end) = &ev[*i] {
            if Some(*end) == stop {
                *i += 1;
                return out;
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
            if let Some(block) = block_for(ev, i, tag) {
                out.push(block);
            }
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
    out
}

fn is_inline_start(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

fn block_for(ev: &[Event], i: &mut usize, tag: Tag) -> Option<Block> {
    match tag {
        Tag::Paragraph => Some(Block::Paragraph(inlines(ev, i, TagEnd::Paragraph))),
        Tag::Heading { level, .. } => {
            let n = level as u8;
            Some(Block::Heading(n, inlines(ev, i, TagEnd::Heading(level))))
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
            Some(Block::Code(lang, text))
        }
        Tag::BlockQuote(_) => Some(Block::Quote(blocks(ev, i, Some(TagEnd::BlockQuote(None))))),
        Tag::List(start) => {
            let ordered = start.is_some();
            let mut items = Vec::new();
            while *i < ev.len() {
                match &ev[*i] {
                    Event::End(TagEnd::List(_)) => {
                        *i += 1;
                        break;
                    }
                    Event::Start(Tag::Item) => {
                        *i += 1;
                        items.push(blocks(ev, i, Some(TagEnd::Item)));
                    }
                    _ => *i += 1,
                }
            }
            Some(Block::List(ordered, items))
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
            Some(Block::Table(rows))
        }
        // Footnotes and anything else we do not model are skipped
        // wholesale rather than half-rendered.
        _ => None,
    }
}

fn inlines(ev: &[Event], i: &mut usize, stop: TagEnd) -> Vec<Inline> {
    let mut out = Vec::new();
    while *i < ev.len() {
        if let Event::End(end) = &ev[*i] {
            if *end == stop {
                *i += 1;
                return out;
            }
            *i += 1;
            continue;
        }
        // Event::Html and Event::InlineHtml are not inline events here, so
        // they fall through and are dropped.
        if !push_inline(ev, i, &mut out) {
            *i += 1;
        }
    }
    out
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
                Tag::Emphasis => out.push(Inline::Emph(inlines(ev, i, TagEnd::Emphasis))),
                Tag::Strong => out.push(Inline::Strong(inlines(ev, i, TagEnd::Strong))),
                Tag::Strikethrough => {
                    // Not modelled as its own node; keep the text.
                    let body = inlines(ev, i, TagEnd::Strikethrough);
                    let mut j = 0;
                    while j < body.len() {
                        out.push(clone_inline(&body[j]));
                        j += 1;
                    }
                }
                Tag::Link { dest_url, .. } => out.push(Inline::Link(
                    dest_url.as_bytes().to_vec(),
                    inlines(ev, i, TagEnd::Link),
                )),
                Tag::Image { dest_url, .. } => {
                    let alt = inlines(ev, i, TagEnd::Image);
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

fn clone_inline(inline: &Inline) -> Inline {
    match inline {
        Inline::Text(t) => Inline::Text(t.clone()),
        Inline::Code(t) => Inline::Code(t.clone()),
        Inline::Math(d, t) => Inline::Math(*d, t.clone()),
        Inline::Emph(b) => Inline::Emph(b.iter().map(clone_inline).collect()),
        Inline::Strong(b) => Inline::Strong(b.iter().map(clone_inline).collect()),
        Inline::Link(u, b) => Inline::Link(u.clone(), b.iter().map(clone_inline).collect()),
        Inline::Image(u, a) => Inline::Image(u.clone(), a.clone()),
        Inline::SoftBreak => Inline::SoftBreak,
        Inline::HardBreak => Inline::HardBreak,
    }
}

fn flatten(inlines: &[Inline]) -> Vec<u8> {
    let mut out = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) | Inline::Code(t) => out.extend_from_slice(t),
            Inline::Math(_, t) => out.extend_from_slice(t),
            Inline::Emph(b) | Inline::Strong(b) => out.extend_from_slice(&flatten(b)),
            Inline::Link(_, b) => out.extend_from_slice(&flatten(b)),
            Inline::Image(_, a) => out.extend_from_slice(a),
            Inline::SoftBreak | Inline::HardBreak => out.push(b' '),
        }
    }
    out
}
