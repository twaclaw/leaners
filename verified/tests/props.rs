//! Deterministic property tests over the austere modules.
//!
//! The Lean side proves some of these statements on hand-written models, and
//! two (slug idempotence, `assign` uniqueness) are still open there. Until the
//! proofs are ported onto the extracted model, empirical coverage on the Rust
//! that actually ships is what ties the statements to this crate on more than
//! the fixed vectors in vectors.txt. Seeded PRNG, no dependencies, so a
//! failure always reproduces.

use leaners_render::escape::{escape, is_safe_url};
use leaners_render::slug::{assign, clone_bytes, slugify};

/// xorshift64. Good enough to spray bytes, and deterministic across runs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn bytes(&mut self, max_len: usize) -> Vec<u8> {
        let len = (self.next() as usize) % (max_len + 1);
        (0..len).map(|_| (self.next() & 0xff) as u8).collect()
    }

    fn bytes_from(&mut self, alphabet: &[u8], max_len: usize) -> Vec<u8> {
        let len = (self.next() as usize) % (max_len + 1);
        (0..len)
            .map(|_| alphabet[(self.next() as usize) % alphabet.len()])
            .collect()
    }
}

/// The inverse the round-trip theorem is stated against. Mirrors
/// proofs/Leaners/Escape.lean `unescape`: a recognised entity decodes, any
/// other `&` copies through.
fn unescape(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let tail = &s[i..];
        if tail.starts_with(b"&amp;") {
            out.push(b'&');
            i += 5;
        } else if tail.starts_with(b"&lt;") {
            out.push(b'<');
            i += 4;
        } else if tail.starts_with(b"&gt;") {
            out.push(b'>');
            i += 4;
        } else if tail.starts_with(b"&quot;") {
            out.push(b'"');
            i += 6;
        } else if tail.starts_with(b"&#39;") {
            out.push(b'\'');
            i += 5;
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

// Ladder step 1 on the shipping Rust: escaping loses nothing, and the output
// carries no raw delimiter. Every `&` in the output must begin an entity.
#[test]
fn escape_round_trips_and_emits_no_delimiters() {
    let mut rng = Rng(0x1ea4e55);
    for _ in 0..2000 {
        let input = rng.bytes(64);
        let mut escaped = Vec::new();
        escape(&input, &mut escaped);

        assert_eq!(unescape(&escaped), input, "round trip failed: {input:?}");
        for (i, &b) in escaped.iter().enumerate() {
            assert!(
                b != b'<' && b != b'>' && b != b'"' && b != b'\'',
                "raw delimiter in {escaped:?}"
            );
            if b == b'&' {
                let tail = &escaped[i..];
                assert!(
                    [&b"&amp;"[..], b"&lt;", b"&gt;", b"&quot;", b"&#39;"]
                        .iter()
                        .any(|e| tail.starts_with(e)),
                    "stray & in {escaped:?}"
                );
            }
        }
    }
}

// Ladder step 2, the two halves the Lean side splits: the charset invariant
// (proved) and idempotence (still open there, so it is covered here).
#[test]
fn slugify_charset_shape_and_idempotence() {
    let mut rng = Rng(0x510965);
    for _ in 0..2000 {
        let input = rng.bytes(48);
        let slug = slugify(&input);

        for &b in &slug {
            assert!(
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-',
                "charset violation in {slug:?} from {input:?}"
            );
        }
        assert!(
            slug.first() != Some(&b'-') && slug.last() != Some(&b'-'),
            "untrimmed: {slug:?}"
        );
        assert!(
            !slug.windows(2).any(|w| w == b"--"),
            "uncollapsed run: {slug:?}"
        );
        assert_eq!(
            slugify(&slug),
            slug,
            "not idempotent: {input:?} -> {slug:?}"
        );
    }
}

// Ladder step 3: the sequence of assigned anchors has no duplicates. A tiny
// alphabet forces heavy collisions so the counter path is actually exercised.
#[test]
fn assign_never_repeats_a_slug() {
    let mut rng = Rng(0xa551);
    let mut taken: Vec<Vec<u8>> = Vec::new();
    for _ in 0..300 {
        let base = slugify(&rng.bytes_from(b"ab -", 4));
        let id = assign(&taken, &base);
        assert!(!taken.contains(&id), "duplicate anchor {id:?}");
        taken.push(clone_bytes(&id));
    }
}

/// Ladder step 4, restated independently: the scheme is the bytes before the
/// first `:`, provided no `/`, `?` or `#` comes first, and only three schemes
/// are allowed. A second implementation of the same spec, so a bug would have
/// to be made twice to slip through.
fn safe_url_oracle(url: &[u8]) -> bool {
    for (i, &b) in url.iter().enumerate() {
        if b == b':' {
            let scheme: Vec<u8> = url[..i].iter().map(|c| c.to_ascii_lowercase()).collect();
            return scheme == b"http" || scheme == b"https" || scheme == b"mailto";
        }
        if b == b'/' || b == b'?' || b == b'#' {
            return true;
        }
    }
    true
}

#[test]
fn is_safe_url_matches_the_oracle() {
    let mut rng = Rng(0x0e11);
    // Weighted towards the bytes the function branches on.
    let alphabet = b"abcHTPSMmailto:/?#.@0123456789-\t ";
    for _ in 0..4000 {
        let url = rng.bytes_from(alphabet, 24);
        assert_eq!(
            is_safe_url(&url),
            safe_url_oracle(&url),
            "disagree on {:?}",
            String::from_utf8_lossy(&url)
        );
    }
    for _ in 0..1000 {
        let url = rng.bytes(24);
        assert_eq!(
            is_safe_url(&url),
            safe_url_oracle(&url),
            "disagree on {url:?}"
        );
    }
}
