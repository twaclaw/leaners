//! Austere module: the backend of the compiler, and the half meant to be
//! verified. Extracted to Lean via charon/aeneas by `make extract`, together
//! with the `Ast` it walks.
//!
//! Ladder steps 5 and 6:
//!
//!   5. every tag opened is closed, correctly nested
//!   6. every `<` in the output was emitted here, never derived from input
//!
//! Step 6 is what makes the unverified parser harmless: it quantifies over *all*
//! `Ast` values, so however wrong the parser is, it cannot produce an `Ast` whose
//! rendering carries markup through. The flat stream shape changes nothing
//! there: marker events emit fixed literals, and every input-derived byte still
//! leaves through `escape`.
//!
//! Step 5 is the property the flattening reprices. On the old tree it held for
//! every value; on a stream it holds for balanced streams, so its statement
//! gains a well-formedness hypothesis, discharged by construction for the
//! streams `adapt` emits.

use crate::ast::{Block, Inline};
use crate::escape::{escape, is_safe_url, push_all};
use crate::highlight::{highlight, lang_of};
use crate::slug::{assign, clone_bytes, slugify};

pub fn render(blocks: &Vec<Block>, out: &mut Vec<u8>) {
    let mut taken: Vec<Vec<u8>> = Vec::new();
    let mut i: usize = 0;
    while i < blocks.len() {
        render_block(&blocks[i], &mut taken, out);
        i += 1;
    }
}

fn render_block(block: &Block, taken: &mut Vec<Vec<u8>>, out: &mut Vec<u8>) {
    match block {
        Block::Paragraph(body) => {
            push_all(out, b"<p>");
            render_inlines(body, out);
            push_all(out, b"</p>\n");
        }
        Block::Inlines(body) => {
            render_inlines(body, out);
        }
        Block::Heading(level, body) => {
            let n = if *level < 1 {
                1
            } else if *level > 6 {
                6
            } else {
                *level
            };
            // Anchor ids are disambiguated against the ones already issued, so
            // the set of ids in a document has no duplicates.
            let text = inline_text(body);
            let id = assign(taken, &slugify(&text));
            taken.push(clone_bytes(&id));

            push_all(out, b"<h");
            out.push(b'0' + n);
            push_all(out, b" id=\"");
            escape(&id, out);
            push_all(out, b"\">");
            render_inlines(body, out);
            push_all(out, b"</h");
            out.push(b'0' + n);
            push_all(out, b">\n");
        }
        Block::Code(lang, text) => {
            push_all(out, b"<pre><code");
            if lang.len() > 0 {
                push_all(out, b" class=\"language-");
                escape(&slugify(lang), out);
                out.push(b'"');
            }
            out.push(b'>');
            // An unrecognised language yields LANG_NONE, whose output is
            // byte-for-byte what plain `escape` produced before.
            highlight(lang_of(lang), text, out);
            push_all(out, b"</code></pre>\n");
        }
        Block::QuoteOpen => {
            push_all(out, b"<blockquote>\n");
        }
        Block::QuoteClose => {
            push_all(out, b"</blockquote>\n");
        }
        Block::ListOpen(ordered) => {
            // Each branch pushes its own literal. `push_all(out, if c {a} else {b})`
            // reads better but extracts to an `ite` at slice type that aeneas
            // cannot elaborate, so the duplication is deliberate.
            if *ordered {
                push_all(out, b"<ol>\n");
            } else {
                push_all(out, b"<ul>\n");
            }
        }
        Block::ListClose(ordered) => {
            if *ordered {
                push_all(out, b"</ol>\n");
            } else {
                push_all(out, b"</ul>\n");
            }
        }
        Block::ItemOpen => {
            push_all(out, b"<li>");
        }
        Block::ItemClose => {
            push_all(out, b"</li>\n");
        }
        Block::Table(rows) => {
            push_all(out, b"<table>\n");
            let mut r: usize = 0;
            while r < rows.len() {
                if r == 0 {
                    push_all(out, b"<thead>\n");
                } else if r == 1 {
                    push_all(out, b"<tbody>\n");
                }
                push_all(out, b"<tr>");
                let mut c: usize = 0;
                while c < rows[r].len() {
                    if r == 0 {
                        push_all(out, b"<th>");
                        render_inlines(&rows[r][c], out);
                        push_all(out, b"</th>");
                    } else {
                        push_all(out, b"<td>");
                        render_inlines(&rows[r][c], out);
                        push_all(out, b"</td>");
                    }
                    c += 1;
                }
                push_all(out, b"</tr>\n");
                if r == 0 {
                    push_all(out, b"</thead>\n");
                }
                r += 1;
            }
            if rows.len() > 1 {
                push_all(out, b"</tbody>\n");
            }
            push_all(out, b"</table>\n");
        }
        Block::Rule => {
            push_all(out, b"<hr>\n");
        }
    }
}

fn render_inlines(inlines: &Vec<Inline>, out: &mut Vec<u8>) {
    let mut i: usize = 0;
    while i < inlines.len() {
        render_inline(&inlines[i], out);
        i += 1;
    }
}

fn render_inline(inline: &Inline, out: &mut Vec<u8>) {
    match inline {
        Inline::Text(t) => escape(t, out),
        Inline::Code(t) => {
            push_all(out, b"<code>");
            escape(t, out);
            push_all(out, b"</code>");
        }
        // No TeX typesetter exists in this pipeline, so the source is escaped
        // and handed to CSS. The tag set stays closed and the escape property
        // holds exactly as it does for code.
        Inline::Math(display, t) => {
            if *display {
                push_all(out, b"<span class=\"math math-display\">");
            } else {
                push_all(out, b"<span class=\"math\">");
            }
            escape(t, out);
            push_all(out, b"</span>");
        }
        Inline::EmphOpen => {
            push_all(out, b"<em>");
        }
        Inline::EmphClose => {
            push_all(out, b"</em>");
        }
        Inline::StrongOpen => {
            push_all(out, b"<strong>");
        }
        Inline::StrongClose => {
            push_all(out, b"</strong>");
        }
        Inline::LinkOpen(url) => {
            push_all(out, b"<a href=\"");
            // A rejected scheme yields an empty href rather than being emitted.
            if is_safe_url(url) {
                escape(url, out);
            }
            push_all(out, b"\">");
        }
        Inline::LinkClose => {
            push_all(out, b"</a>");
        }
        Inline::Image(url, alt) => {
            push_all(out, b"<img src=\"");
            if is_safe_url(url) {
                escape(url, out);
            }
            push_all(out, b"\" alt=\"");
            escape(alt, out);
            push_all(out, b"\">");
        }
        Inline::SoftBreak => out.push(b'\n'),
        Inline::HardBreak => push_all(out, b"<br>\n"),
    }
}

/// Plain text of an inline run, used to derive heading anchors. Marker events
/// contribute nothing; on the old tree this recursed into their children, and
/// on the stream those children are simply the events that follow.
fn inline_text(inlines: &Vec<Inline>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < inlines.len() {
        match &inlines[i] {
            Inline::Text(t) => push_bytes(&mut out, t),
            Inline::Code(t) => push_bytes(&mut out, t),
            Inline::Math(_, t) => push_bytes(&mut out, t),
            Inline::EmphOpen => {}
            Inline::EmphClose => {}
            Inline::StrongOpen => {}
            Inline::StrongClose => {}
            Inline::LinkOpen(_) => {}
            Inline::LinkClose => {}
            Inline::Image(_, alt) => push_bytes(&mut out, alt),
            Inline::SoftBreak => out.push(b' '),
            Inline::HardBreak => out.push(b' '),
        }
        i += 1;
    }
    out
}

fn push_bytes(out: &mut Vec<u8>, v: &Vec<u8>) {
    let mut i: usize = 0;
    while i < v.len() {
        out.push(v[i]);
        i += 1;
    }
}
