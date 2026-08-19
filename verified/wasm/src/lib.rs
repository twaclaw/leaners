//! Raw WebAssembly ABI. Deliberately no wasm-bindgen: the whole interface is
//! four exported functions plus linear memory, which keeps the artifact small
//! and the JavaScript side explicit.
//!
//! Glue, not verified code. This is the only place `unsafe` appears.
//!
//! It lives in its own crate because charon's driver stops after analysis and
//! emits no object code, so a `cdylib` in the extracted crate fails to link.
//! Keeping `leaners-render` an rlib is what makes `make extract` work.

use std::alloc::{Layout, alloc as galloc, dealloc as gdealloc};
use std::ptr;

/// Rendered output, held until the next call to `render`.
static mut RESULT: Vec<u8> = Vec::new();

fn layout(size: usize) -> Option<Layout> {
    // Bytes, so alignment 1. Checked rather than _unchecked: `size` crosses the
    // ABI from JavaScript, and a value over isize::MAX would make the layout
    // invalid, which the allocator is allowed to treat as undefined behaviour.
    Layout::from_size_align(size, 1).ok()
}

/// Reserve `size` bytes for the caller to write UTF-8 into. Null when the
/// size is zero, invalid, or the allocation fails; the caller must check.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }
    match layout(size) {
        Some(l) => unsafe { galloc(l) },
        None => ptr::null_mut(),
    }
}

/// Release a buffer obtained from `alloc`.
///
/// # Safety
///
/// `p` must be null, or a pointer returned by `alloc` that has not been freed,
/// and `size` must be the value passed to that `alloc` call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(p: *mut u8, size: usize) {
    if !p.is_null()
        && size != 0
        && let Some(l) = layout(size)
    {
        unsafe { gdealloc(p, l) };
    }
}

/// Render `len` bytes of Markdown at `p`.
///
/// The result is readable via `result_ptr` and `result_len` until the next call
/// to this function, which overwrites it.
///
/// # Safety
///
/// `p` must be null, or point to `len` initialised bytes that stay valid for the
/// duration of the call. Invalid UTF-8 is replaced rather than rejected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn render(p: *const u8, len: usize) {
    let input = if p.is_null() || len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(p, len) }
    };
    let src = String::from_utf8_lossy(input);
    unsafe { RESULT = leaners_render::markdown_to_html(&src).into_bytes() };
}

/// Start of the last rendered result.
///
/// # Safety
///
/// Valid only until the next call to `render`, which frees the previous buffer.
/// Taking the raw pointer first, rather than dereferencing `&raw const` inline,
/// keeps clippy quiet without forming a reference to the mutable static, which
/// edition 2024 rejects outright.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn result_ptr() -> *const u8 {
    let result = &raw const RESULT;
    unsafe { (*result).as_ptr() }
}

/// Length in bytes of the last rendered result.
///
/// # Safety
///
/// Same lifetime caveat as `result_ptr`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn result_len() -> usize {
    let result = &raw const RESULT;
    unsafe { (*result).len() }
}
