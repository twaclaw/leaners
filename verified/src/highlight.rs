//! Austere module: extracted to Lean via charon/aeneas by `make extract`.
//!
//! Highlighting happens here rather than in a JS library in the page, and that is
//! the whole point. A JS highlighter would re-parse and rewrite the HTML `render`
//! just produced, putting unverified code inside the trust boundary and voiding
//! ladder step 6. Done here it inherits step 6 instead:
//!
//!   - every byte of `text` reaches the output through `escape_byte`, so no `<`
//!     in a code block can ever be markup;
//!   - the only markup this module can emit is one of the fixed literals in
//!     `open` and `close` below, which is a closed set of five span classes;
//!   - `open` and `close` are called in pairs on every path, so spans nest.
//!
//! The lexers themselves are allowed to be wrong. A mislabelled keyword is a
//! cosmetic bug, not a safety one, and no property above depends on them.

use crate::escape::{escape_byte, push_all};

pub const LANG_NONE: u8 = 0;
pub const LANG_RUST: u8 = 1;
pub const LANG_BASH: u8 = 2;
pub const LANG_LEAN: u8 = 3;

const TOK_KW: u8 = 1;
const TOK_STR: u8 = 2;
const TOK_COM: u8 = 3;
const TOK_NUM: u8 = 4;
const TOK_VAR: u8 = 5;

/// The closed set of tags this module can emit. Nothing here depends on input.
fn open(tok: u8, out: &mut Vec<u8>) {
    if tok == TOK_KW {
        push_all(out, b"<span class=\"tok-kw\">");
    } else if tok == TOK_STR {
        push_all(out, b"<span class=\"tok-str\">");
    } else if tok == TOK_COM {
        push_all(out, b"<span class=\"tok-com\">");
    } else if tok == TOK_NUM {
        push_all(out, b"<span class=\"tok-num\">");
    } else if tok == TOK_VAR {
        push_all(out, b"<span class=\"tok-var\">");
    }
}

fn close(out: &mut Vec<u8>) {
    push_all(out, b"</span>");
}

/// Copies `text[from..to]` to the output, escaped, wrapped in one span.
fn emit(tok: u8, text: &Vec<u8>, from: usize, to: usize, out: &mut Vec<u8>) {
    open(tok, out);
    let mut i: usize = from;
    while i < to && i < text.len() {
        escape_byte(text[i], out);
        i += 1;
    }
    close(out);
}

/// Maps a fence's info string to a lexer. Unknown languages get `LANG_NONE`,
/// which is byte-for-byte the old behaviour of plain `escape`.
pub fn lang_of(lang: &Vec<u8>) -> u8 {
    if eq_ci(lang, b"rust") || eq_ci(lang, b"rs") {
        return LANG_RUST;
    }
    if eq_ci(lang, b"bash") || eq_ci(lang, b"sh") || eq_ci(lang, b"shell") || eq_ci(lang, b"zsh") {
        return LANG_BASH;
    }
    if eq_ci(lang, b"lean") || eq_ci(lang, b"lean4") {
        return LANG_LEAN;
    }
    LANG_NONE
}

pub fn highlight(kind: u8, text: &Vec<u8>, out: &mut Vec<u8>) {
    if kind == LANG_RUST {
        highlight_rust(text, out);
    } else if kind == LANG_BASH {
        highlight_bash(text, out);
    } else if kind == LANG_LEAN {
        highlight_lean(text, out);
    } else {
        let mut i: usize = 0;
        while i < text.len() {
            escape_byte(text[i], out);
            i += 1;
        }
    }
}

// ---------------------------------------------------------------- byte classes

fn is_digit(b: u8) -> bool {
    b >= b'0' && b <= b'9'
}

/// Bytes >= 0x80 count as identifier bytes so that a multi-byte UTF-8 sequence
/// is never split across a span boundary. Lean needs this for `α` and `₁`.
fn is_ident(b: u8) -> bool {
    (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z') || is_digit(b) || b == b'_' || b >= 0x80
}

fn is_space(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn at(text: &Vec<u8>, i: usize) -> u8 {
    if i < text.len() { text[i] } else { 0 }
}

fn eq_ci(a: &Vec<u8>, b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < b.len() {
        let mut c = a[i];
        if c >= b'A' && c <= b'Z' {
            c += 32;
        }
        if c != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

// ------------------------------------------------------------- shared scanners

/// A run of identifier bytes starting at `i`.
fn scan_ident(text: &Vec<u8>, i: usize) -> usize {
    let mut j: usize = i;
    while j < text.len() && is_ident(text[j]) {
        j += 1;
    }
    j
}

/// A number. Deliberately loose: digits, letters and `_` so that `0xFF`, `1_000`
/// and `3u8` come out as one token, plus one `.` if a digit follows it.
fn scan_number(text: &Vec<u8>, i: usize) -> usize {
    let mut j: usize = i;
    let mut stop: bool = false;
    while j < text.len() && !stop {
        let c = text[j];
        if is_digit(c)
            || c == b'_'
            || (c >= b'a' && c <= b'z')
            || (c >= b'A' && c <= b'Z')
            || (c == b'.' && is_digit(at(text, j + 1)))
        {
            j += 1;
        } else {
            stop = true;
        }
    }
    j
}

/// A quoted string ending at the matching `quote`. `escapes` controls whether a
/// backslash hides the next byte, which single-quoted shell strings do not do.
/// Returns the index one past the closing quote, or the end of input.
fn scan_quoted(text: &Vec<u8>, i: usize, quote: u8, escapes: bool) -> usize {
    let mut j: usize = i + 1;
    while j < text.len() {
        let c = text[j];
        if escapes && c == b'\\' {
            j += 2;
        } else if c == quote {
            return j + 1;
        } else {
            j += 1;
        }
    }
    text.len()
}

fn scan_to_eol(text: &Vec<u8>, i: usize) -> usize {
    let mut j: usize = i;
    while j < text.len() && text[j] != b'\n' {
        j += 1;
    }
    j
}

/// A nesting block comment delimited by `o1 o2` and `c1 c2`. Both Rust's
/// `/* */` and Lean's `/- -/` nest, so a depth counter is required.
fn scan_block_comment(text: &Vec<u8>, i: usize, o2: u8, c1: u8, c2: u8) -> usize {
    let mut j: usize = i + 2;
    let mut depth: usize = 1;
    while j < text.len() {
        if at(text, j) == c1 && at(text, j + 1) == c2 {
            depth -= 1;
            j += 2;
            if depth == 0 {
                return j;
            }
        } else if at(text, j) == text[i] && at(text, j + 1) == o2 {
            depth += 1;
            j += 2;
        } else {
            j += 1;
        }
    }
    text.len()
}

/// Compares `text[from..to]` against `table[ts..te]`.
fn span_eq(text: &Vec<u8>, from: usize, to: usize, table: &[u8], ts: usize, te: usize) -> bool {
    if to < from || te < ts {
        return false;
    }
    if to - from != te - ts {
        return false;
    }
    let mut k: usize = 0;
    while k < to - from {
        if text[from + k] != table[ts + k] {
            return false;
        }
        k += 1;
    }
    true
}

/// Is `text[from..to]` one of the newline-separated words in `table`?
///
/// A table plus one indexed loop, rather than a chain of forty `word_is(..) ||`
/// terms. The chain is more readable but extracts into `do` blocks nested deeply
/// enough to break aeneas's pretty-printer, which cost an afternoon to diagnose.
fn kw_match(text: &Vec<u8>, from: usize, to: usize, table: &[u8]) -> bool {
    let mut i: usize = 0;
    while i < table.len() {
        let start = i;
        while i < table.len() && table[i] != b'\n' {
            i += 1;
        }
        if span_eq(text, from, to, table, start, i) {
            return true;
        }
        i += 1;
    }
    false
}

// ----------------------------------------------------------------------- rust

fn highlight_rust(text: &Vec<u8>, out: &mut Vec<u8>) {
    let mut i: usize = 0;
    while i < text.len() {
        let c = text[i];

        if c == b'/' && at(text, i + 1) == b'/' {
            let j = scan_to_eol(text, i);
            emit(TOK_COM, text, i, j, out);
            i = j;
        } else if c == b'/' && at(text, i + 1) == b'*' {
            let j = scan_block_comment(text, i, b'*', b'*', b'/');
            emit(TOK_COM, text, i, j, out);
            i = j;
        } else if c == b'"' {
            let j = scan_quoted(text, i, b'"', true);
            emit(TOK_STR, text, i, j, out);
            i = j;
        } else if (c == b'r' || c == b'b') && is_raw_string(text, i) {
            let j = scan_raw_string(text, i);
            emit(TOK_STR, text, i, j, out);
            i = j;
        } else if c == b'\'' && is_char_literal(text, i) {
            let j = scan_quoted(text, i, b'\'', true);
            emit(TOK_STR, text, i, j, out);
            i = j;
        } else if is_digit(c) {
            let j = scan_number(text, i);
            emit(TOK_NUM, text, i, j, out);
            i = j;
        } else if is_ident(c) {
            let j = scan_ident(text, i);
            if rust_kw(text, i, j) {
                emit(TOK_KW, text, i, j, out);
            } else {
                emit_plain(text, i, j, out);
            }
            i = j;
        } else {
            escape_byte(c, out);
            i += 1;
        }
    }
}

fn emit_plain(text: &Vec<u8>, from: usize, to: usize, out: &mut Vec<u8>) {
    let mut i: usize = from;
    while i < to && i < text.len() {
        escape_byte(text[i], out);
        i += 1;
    }
}

/// `r"..."`, `r#"..."#`, `b"..."`. Needs a lookahead so that `read` is not
/// mistaken for the start of a raw string.
fn is_raw_string(text: &Vec<u8>, i: usize) -> bool {
    let mut j: usize = i + 1;
    if text[i] == b'b' && at(text, j) == b'r' {
        j += 1;
    }
    if at(text, j) == b'"' {
        return true;
    }
    let mut hashes: usize = 0;
    while at(text, j) == b'#' {
        hashes += 1;
        j += 1;
    }
    hashes > 0 && at(text, j) == b'"'
}

fn scan_raw_string(text: &Vec<u8>, i: usize) -> usize {
    let mut j: usize = i + 1;
    if text[i] == b'b' && at(text, j) == b'r' {
        j += 1;
    }
    let mut hashes: usize = 0;
    while at(text, j) == b'#' {
        hashes += 1;
        j += 1;
    }
    if at(text, j) != b'"' {
        return j;
    }
    j += 1;
    while j < text.len() {
        if text[j] == b'"' {
            let mut k: usize = 0;
            while k < hashes && at(text, j + 1 + k) == b'#' {
                k += 1;
            }
            if k == hashes {
                return j + 1 + hashes;
            }
        }
        j += 1;
    }
    text.len()
}

/// Distinguishes `'a'` and `'\n'` from the lifetime `'a`, which is not a string.
fn is_char_literal(text: &Vec<u8>, i: usize) -> bool {
    if at(text, i + 1) == b'\\' {
        return true;
    }
    // One byte, or one multi-byte codepoint, then a closing quote.
    if at(text, i + 1) >= 0x80 {
        let mut j: usize = i + 1;
        while j < text.len() && text[j] >= 0x80 {
            j += 1;
        }
        return at(text, j) == b'\'';
    }
    at(text, i + 1) != 0 && at(text, i + 2) == b'\''
}

fn rust_kw(text: &Vec<u8>, s: usize, e: usize) -> bool {
    kw_match(text, s, e, b"as\nasync\nawait\nbreak\nconst\ncontinue\ncrate\ndyn\nelse\nenum\nextern\nfalse\nfn\nfor\nif\nimpl\nin\nlet\nloop\nmatch\nmod\nmove\nmut\npub\nref\nreturn\nself\nSelf\nstatic\nstruct\nsuper\ntrait\ntrue\ntype\nunsafe\nuse\nwhere\nwhile")
}

// ----------------------------------------------------------------------- bash

fn highlight_bash(text: &Vec<u8>, out: &mut Vec<u8>) {
    let mut i: usize = 0;
    while i < text.len() {
        let c = text[i];

        // `#` opens a comment only at the start of a word, so `$#` and `x#y`
        // are left alone.
        if c == b'#' && (i == 0 || is_space(text[i - 1])) {
            let j = scan_to_eol(text, i);
            emit(TOK_COM, text, i, j, out);
            i = j;
        } else if c == b'\'' {
            let j = scan_quoted(text, i, b'\'', false);
            emit(TOK_STR, text, i, j, out);
            i = j;
        } else if c == b'"' {
            let j = scan_quoted(text, i, b'"', true);
            emit(TOK_STR, text, i, j, out);
            i = j;
        } else if c == b'$' {
            let j = scan_dollar(text, i);
            emit(TOK_VAR, text, i, j, out);
            i = j;
        } else if is_digit(c) && (i == 0 || !is_ident(text[i - 1])) {
            let j = scan_number(text, i);
            emit(TOK_NUM, text, i, j, out);
            i = j;
        } else if is_ident(c) {
            let j = scan_ident(text, i);
            if bash_kw(text, i, j) {
                emit(TOK_KW, text, i, j, out);
            } else {
                emit_plain(text, i, j, out);
            }
            i = j;
        } else {
            escape_byte(c, out);
            i += 1;
        }
    }
}

/// `$name`, `${...}`, `$1`, `$?`, `$@`, `$#`.
fn scan_dollar(text: &Vec<u8>, i: usize) -> usize {
    let n = at(text, i + 1);
    if n == b'{' {
        let mut j: usize = i + 2;
        while j < text.len() && text[j] != b'}' {
            j += 1;
        }
        if j < text.len() {
            return j + 1;
        }
        return text.len();
    }
    if is_ident(n) {
        return scan_ident(text, i + 1);
    }
    if n == b'?' || n == b'@' || n == b'#' || n == b'*' || n == b'$' || n == b'!' {
        return i + 2;
    }
    i + 1
}

fn bash_kw(text: &Vec<u8>, s: usize, e: usize) -> bool {
    kw_match(text, s, e, b"if\nthen\nelif\nelse\nfi\nfor\nwhile\nuntil\ndo\ndone\ncase\nesac\nin\nfunction\nselect\nreturn\nbreak\ncontinue\nlocal\nexport\nreadonly\ndeclare\nunset\nshift\neval\nexec\nexit\nset\ntrap\nsource\nalias\necho\ncd")
}

// ----------------------------------------------------------------------- lean

fn highlight_lean(text: &Vec<u8>, out: &mut Vec<u8>) {
    let mut i: usize = 0;
    while i < text.len() {
        let c = text[i];

        if c == b'-' && at(text, i + 1) == b'-' {
            let j = scan_to_eol(text, i);
            emit(TOK_COM, text, i, j, out);
            i = j;
        } else if c == b'/' && at(text, i + 1) == b'-' {
            let j = scan_block_comment(text, i, b'-', b'-', b'/');
            emit(TOK_COM, text, i, j, out);
            i = j;
        } else if c == b'"' {
            let j = scan_quoted(text, i, b'"', true);
            emit(TOK_STR, text, i, j, out);
            i = j;
        } else if is_digit(c) && (i == 0 || !is_ident(text[i - 1])) {
            let j = scan_number(text, i);
            emit(TOK_NUM, text, i, j, out);
            i = j;
        } else if is_ident(c) {
            let j = scan_ident(text, i);
            if lean_kw(text, i, j) {
                emit(TOK_KW, text, i, j, out);
            } else {
                emit_plain(text, i, j, out);
            }
            i = j;
        } else {
            escape_byte(c, out);
            i += 1;
        }
    }
}

fn lean_kw(text: &Vec<u8>, s: usize, e: usize) -> bool {
    kw_match(text, s, e, b"theorem\nlemma\ndef\nabbrev\nexample\ninstance\nstructure\ninductive\nclass\nwhere\nderiving\nmutual\npartial\nnoncomputable\nprivate\nprotected\nunsafe\nopaque\naxiom\nvariable\nuniverse\nopen\nnamespace\nsection\nend\nimport\nattribute\nmacro\nnotation\nsyntax\nby\nfun\nlet\nhave\nshow\nfrom\ncalc\nmatch\nwith\ndo\nif\nthen\nelse\nat\nType\nProp\nSort")
}
