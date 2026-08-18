// In-page editing. Writes commits straight to the branch via the Contents API,
// so saving here is identical to committing through GitHub's web editor.

import { readFile, writeFile } from "./github.js";
import { renderMarkdown } from "./renderer.js";
import { CONFIG } from "./config.js";

const DIR = CONFIG.contentDir;

const escapeHtml = (s) =>
  s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

function shell({ heading, fields, body, action }) {
  return `<h1>${heading}</h1>
    ${fields}
    <div class="panes">
      <textarea id="src" spellcheck="false">${escapeHtml(body)}</textarea>
      <div id="preview" class="preview"></div>
    </div>
    <div class="actions">
      <button id="save">${action}</button>
      <input id="msg" type="text" placeholder="Commit message (optional)">
      <span id="status"></span>
    </div>`;
}

function wirePreview() {
  const src = document.getElementById("src");
  const preview = document.getElementById("preview");
  const update = async () => (preview.innerHTML = await renderMarkdown(src.value));
  src.addEventListener("input", update);
  update();
  return src;
}

function status(text, kind = "") {
  const el = document.getElementById("status");
  el.textContent = text;
  el.className = kind;
}

export async function editView(el, docPath) {
  const path = `${DIR}/${docPath}`;
  el.innerHTML = "<p>Loading&hellip;</p>";

  let file;
  try {
    file = await readFile(path);
  } catch (err) {
    el.innerHTML = `<h1>Cannot edit</h1><p class="err">${escapeHtml(err.message)}</p>`;
    return;
  }
  if (file.text === null) {
    el.innerHTML = `<h1>Not found</h1><p><code>${path}</code> does not exist.</p>`;
    return;
  }

  el.innerHTML = shell({
    heading: `Editing <code>${docPath}</code>`,
    fields: "",
    body: file.text,
    action: "Save",
  });

  const src = wirePreview();
  document.getElementById("save").onclick = async () => {
    const msg = document.getElementById("msg").value || `Update ${docPath}`;
    status("Saving…");
    try {
      await writeFile(path, src.value, msg, file.sha);
      status("Saved.", "ok");
      setTimeout(() => (location.hash = `#/${docPath.replace(/\.md$/, "")}`), 600);
    } catch (err) {
      status(
        err.message === "CONFLICT"
          ? "Someone else changed this file. Reload and reapply your edit."
          : err.message,
        "err",
      );
    }
  };
}

export async function newView(el) {
  el.innerHTML = shell({
    heading: "New page",
    fields: `<div class="fields">
      <label>Path <code>${DIR}/</code>
        <input id="path" type="text" placeholder="notes/my-note.md" size="30"></label>
      <label>Title <input id="title" type="text" placeholder="My note" size="24"></label>
    </div>`,
    body: "# My note\n\nWrite here.\n",
    action: "Create",
  });

  const src = wirePreview();
  document.getElementById("save").onclick = async () => {
    let rel = document.getElementById("path").value.trim().replace(/^\/+/, "");
    const title = document.getElementById("title").value.trim();
    if (!rel) return status("A path is required.", "err");
    if (!rel.endsWith(".md")) rel += ".md";
    if (!title) return status("A title is required.", "err");

    const path = `${DIR}/${rel}`;
    const msg = document.getElementById("msg").value || `Add ${rel}`;

    try {
      status("Checking…");
      if ((await readFile(path)).text !== null) return status(`${rel} already exists.`, "err");

      // Two writes, so two commits. Not atomic: if the second fails the page
      // exists but is unlisted, which `make check` reports and a retry fixes.
      status("Creating page…");
      await writeFile(path, src.value, msg);

      status("Updating index…");
      const index = await readFile(`${DIR}/index.json`);
      const data = JSON.parse(index.text);
      data.docs.push({ path: rel, title });
      data.docs.sort((a, b) => a.path.localeCompare(b.path));
      await writeFile(`${DIR}/index.json`, JSON.stringify(data, null, 2) + "\n", `List ${rel}`, index.sha);

      status("Created.", "ok");
      setTimeout(() => (location.hash = `#/${rel.replace(/\.md$/, "")}`), 600);
    } catch (err) {
      status(err.message === "CONFLICT" ? "Index changed underneath. Try again." : err.message, "err");
    }
  };
}
