// The renderer seam.
//
// M0 uses marked. At M5 this is replaced by the Rust/WASM renderer and nothing
// else has to change, which is why it is async now: initialising a WASM module
// needs an await, and adding one later would mean touching every call site.
//
// Note that marked does not sanitise. That is a real XSS hole, and it is the
// exact hole the verified renderer is meant to close. See design.md section 5.

let ready = null;

async function init() {
  if (!window.marked) {
    await new Promise((resolve, reject) => {
      const s = document.createElement("script");
      s.src = "vendor/marked.min.js";
      s.onload = resolve;
      s.onerror = () => reject(new Error("failed to load vendor/marked.min.js"));
      document.head.appendChild(s);
    });
  }
  window.marked.setOptions({ gfm: true, breaks: false });
}

export async function renderMarkdown(src) {
  ready ??= init();
  await ready;
  return window.marked.parse(src);
}
