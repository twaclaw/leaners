//! Reads Markdown on stdin, writes HTML on stdout. Exists so the renderer can
//! be tested and debugged natively, long before WebAssembly is involved.

use std::io::{Read, Write};

fn main() {
    let mut src = String::new();
    std::io::stdin()
        .read_to_string(&mut src)
        .expect("read stdin");
    let html = leaners_render::markdown_to_html(&src);
    std::io::stdout()
        .write_all(html.as_bytes())
        .expect("write stdout");
}
