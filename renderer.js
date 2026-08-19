// The renderer seam.
//
// This is the Rust renderer compiled to WebAssembly. Its `Ast -> HTML` half is
// the part meant to be verified in Lean via Aeneas; the Markdown parser it
// sits behind is ordinary pulldown-cmark and is never verified. See design.md
// section 5 for why that split still yields an unconditional safety property.
//
// The interface is raw: four exported functions and linear memory, no
// wasm-bindgen. Kept async because instantiating a module requires it.

const BASE = location.pathname.replace(/[^/]*$/, "");

let ready = null;

async function init() {
  const url = `${BASE}pkg/render.wasm`;
  // Same reason app.js bypasses the cache: a rebuilt renderer must not sit
  // invisible behind a cached copy of the previous one.
  const opts = { cache: "no-cache" };
  let instance;
  try {
    ({ instance } = await WebAssembly.instantiateStreaming(fetch(url, opts), {}));
  } catch {
    // Falls back for any server that does not send application/wasm.
    const bytes = await (await fetch(url, opts)).arrayBuffer();
    ({ instance } = await WebAssembly.instantiate(bytes, {}));
  }
  return instance.exports;
}

export async function renderMarkdown(src) {
  ready ??= init();
  const wasm = await ready;

  const bytes = new TextEncoder().encode(src);
  const ptr = wasm.alloc(bytes.length);
  if (bytes.length > 0) {
    // A null pointer here would alias the bottom of linear memory, and the
    // write below would silently corrupt the module rather than fail.
    if (!ptr) throw new Error("wasm alloc failed");
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  }
  wasm.render(ptr, bytes.length);
  wasm.dealloc(ptr, bytes.length);

  // Read the result before any further call: it lives until the next render.
  // memory.buffer is re-read here because growing the heap detaches the old one.
  return new TextDecoder().decode(
    new Uint8Array(wasm.memory.buffer, wasm.result_ptr(), wasm.result_len()),
  );
}
