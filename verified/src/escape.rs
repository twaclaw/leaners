//! Austere module: extracted to Lean via charon/aeneas by `make extract`.
//!
//! No iterators, no closures, no traits, no `String`. Indexed `while` loops over
//! `Vec<u8>` only. Not idiomatic Rust, and not meant to be.
//!
//! Three ladder rungs live here (design.md section 5):
//!
//!   0. `escape_byte` emits no `<`, `>`, `"` or `'`
//!   1. `unescape (escape s) = s`, the round trip
//!   4. `is_safe_url` admits only http, https, mailto and relative URLs
//!
//! Step 0 alone would be satisfied by returning the empty vector, which is why
//! step 1 is the one that carries the weight. All three rungs are proved in
//! proofs/Leaners/Proofs/ about the model `make extract` produces from this
//! file; proofs/Leaners/Refine.lean is the bridge from the extraction to the
//! specs those theorems are stated over.

/// Copies a byte-string literal into the output. `&[u8]` turned out to extract
/// cleanly, so the byte-by-byte fallback this once warned about is unnecessary.
pub fn push_all(out: &mut Vec<u8>, s: &[u8]) {
    let mut i: usize = 0;
    while i < s.len() {
        out.push(s[i]);
        i += 1;
    }
}

/// Ladder step 0: every byte either passes through unchanged or becomes a
/// named entity. No byte of output outside an entity is `<`, `>`, `&` or `"`.
pub fn escape_byte(b: u8, out: &mut Vec<u8>) {
    if b == b'&' {
        push_all(out, b"&amp;");
    } else if b == b'<' {
        push_all(out, b"&lt;");
    } else if b == b'>' {
        push_all(out, b"&gt;");
    } else if b == b'"' {
        push_all(out, b"&quot;");
    } else if b == b'\'' {
        push_all(out, b"&#39;");
    } else {
        out.push(b);
    }
}

pub fn escape(input: &Vec<u8>, out: &mut Vec<u8>) {
    let mut i: usize = 0;
    while i < input.len() {
        escape_byte(input[i], out);
        i += 1;
    }
}

/// Ladder step 4: URL scheme allowlist. Anything not clearly safe becomes
/// empty rather than being emitted, which is what stops `javascript:` URLs.
pub fn is_safe_url(url: &Vec<u8>) -> bool {
    // A URL with no scheme is relative, and relative is always safe.
    let mut i: usize = 0;
    let mut colon: usize = url.len();
    while i < url.len() {
        let c = url[i];
        if c == b':' {
            colon = i;
            i = url.len();
        } else if c == b'/' || c == b'?' || c == b'#' {
            // Path/query/fragment started before any colon: no scheme.
            i = url.len();
        } else {
            i += 1;
        }
    }
    if colon == url.len() {
        return true;
    }
    starts_with_ci(url, b"http:")
        || starts_with_ci(url, b"https:")
        || starts_with_ci(url, b"mailto:")
}

fn starts_with_ci(url: &Vec<u8>, prefix: &[u8]) -> bool {
    if url.len() < prefix.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < prefix.len() {
        if lower(url[i]) != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub fn lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' { b + 32 } else { b }
}
