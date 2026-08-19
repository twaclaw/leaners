//! Austere module: extracted to Lean via charon/aeneas by `make extract`.
//!
//! Ladder steps 2 and 3 (design.md section 5). `slugify` is deliberately not
//! injective: "A B" and "a-b" both give "a-b". Uniqueness is `assign`'s job, and
//! it appends a counter until the slug is unused, so the property there is
//! `Nodup` on the sequence of results.
//!
//! Proved so far: the charset invariant on `slugify`, stated about the model
//! `make extract` produces from this file (proofs/Leaners/Refine.lean carries
//! the refinement). Idempotence and `assign`'s `Nodup` are still open;
//! verified/tests/props.rs covers both empirically until they land.

use crate::escape::lower;

/// Lowercase ASCII alphanumerics survive; every other run becomes one '-'.
/// Leading and trailing '-' are trimmed.
pub fn slugify(input: &Vec<u8>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    let mut pending_dash: bool = false;

    while i < input.len() {
        let c = lower(input[i]);
        if (c >= b'a' && c <= b'z') || (c >= b'0' && c <= b'9') {
            if pending_dash && out.len() > 0 {
                out.push(b'-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
        i += 1;
    }
    out
}

fn eq(a: &Vec<u8>, b: &Vec<u8>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn push_usize(out: &mut Vec<u8>, mut n: usize) {
    if n == 0 {
        out.push(b'0');
        return;
    }
    let mut digits: Vec<u8> = Vec::new();
    while n > 0 {
        digits.push(b'0' + ((n % 10) as u8));
        n /= 10;
    }
    let mut i: usize = digits.len();
    while i > 0 {
        i -= 1;
        out.push(digits[i]);
    }
}

/// Ladder step 3: the slug actually used for an anchor, disambiguated against
/// the slugs already issued. The property to prove is that the sequence of
/// results contains no duplicates.
pub fn assign(taken: &Vec<Vec<u8>>, base: &Vec<u8>) -> Vec<u8> {
    let mut candidate: Vec<u8> = clone_bytes(base);
    let mut n: usize = 1;
    while contains(taken, &candidate) {
        n += 1;
        candidate = clone_bytes(base);
        candidate.push(b'-');
        push_usize(&mut candidate, n);
    }
    candidate
}

fn contains(haystack: &Vec<Vec<u8>>, needle: &Vec<u8>) -> bool {
    let mut i: usize = 0;
    while i < haystack.len() {
        if eq(&haystack[i], needle) {
            return true;
        }
        i += 1;
    }
    false
}

pub fn clone_bytes(v: &Vec<u8>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < v.len() {
        out.push(v[i]);
        i += 1;
    }
    out
}
