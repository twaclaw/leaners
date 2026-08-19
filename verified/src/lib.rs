//! Markdown to HTML, split into a frontend we trust nothing about and a backend
//! meant to be verified. See design.md section 5.
//!
//!   markdown --[ adapt, UNVERIFIED ]--> Ast --[ render, VERIFIED ]--> HTML
//!                                        ^ trust boundary
//!
//! This crate is an rlib so that charon can extract it. The WebAssembly shim is a
//! separate cdylib in `verified/wasm/`, because charon's driver emits no object
//! code and a cdylib here would fail to link.

pub mod adapt;
pub mod ast;
pub mod escape;
pub mod highlight;
pub mod render;
pub mod slug;

pub fn markdown_to_html(src: &str) -> String {
    let blocks = adapt::parse(src);
    let mut out: Vec<u8> = Vec::new();
    render::render(&blocks, &mut out);
    // render only ever emits ASCII markup plus bytes copied from the input,
    // and the input was valid UTF-8, so this cannot lose anything.
    String::from_utf8_lossy(&out).into_owned()
}
