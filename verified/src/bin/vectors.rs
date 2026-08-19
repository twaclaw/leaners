//! Prints the austere functions' results for each line of tests/vectors.txt, in
//! a format the Lean model reproduces exactly. `make crosscheck` diffs the two.
//!
//! Without mechanical extraction this is the only thing tying the hand-written
//! Lean model to the Rust that actually ships, so it is not optional. See
//! design.md section 7, fallback B2.

use leaners_render::escape::{escape, is_safe_url};
use leaners_render::slug::slugify;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors.txt");
    let text = std::fs::read_to_string(path).expect("read vectors");
    for line in text.lines() {
        let v = line.as_bytes().to_vec();
        let mut escaped = Vec::new();
        escape(&v, &mut escaped);
        println!("E {}", hex(&escaped));
        println!("S {}", hex(&slugify(&v)));
        println!("U {}", if is_safe_url(&v) { 1 } else { 0 });
    }
}
