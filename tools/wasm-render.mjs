// Renders Markdown through a compiled render.wasm and writes the HTML to
// stdout. Used by CI to check that the committed artifact and a rebuild from
// the committed sources compute the same function.
//
// Node has WebAssembly built in, so this needs no dependencies. The ABI is the
// same four exports renderer.js drives in the browser.
//
//   node tools/wasm-render.mjs pkg/render.wasm content/notes/*.md

import { readFile } from "node:fs/promises";

const [wasmPath, ...docs] = process.argv.slice(2);
if (!wasmPath || docs.length === 0) {
  console.error("usage: node tools/wasm-render.mjs <render.wasm> <doc.md>...");
  process.exit(2);
}

const { instance } = await WebAssembly.instantiate(await readFile(wasmPath), {});
const { alloc, dealloc, render, result_ptr, result_len, memory } = instance.exports;

for (const doc of docs) {
  const src = await readFile(doc);
  const ptr = alloc(src.length);
  if (src.length > 0) {
    if (!ptr) throw new Error(`wasm alloc failed for ${doc}`);
    new Uint8Array(memory.buffer, ptr, src.length).set(src);
  }
  render(ptr, src.length);
  dealloc(ptr, src.length);
  // memory.buffer is re-read after the call: growing the heap detaches it.
  const html = Buffer.from(new Uint8Array(memory.buffer, result_ptr(), result_len()));
  process.stdout.write(`===== ${doc} (${html.length} bytes)\n`);
  process.stdout.write(html);
  process.stdout.write("\n");
}
