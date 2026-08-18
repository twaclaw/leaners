import { CONFIG } from "./config.js";
import { renderMarkdown } from "./renderer.js";
import { getToken, setToken, whoami, repoCoords } from "./github.js";
import { editView, newView } from "./editor.js";

// Pages serves project sites from /REPO/, so every fetch must be relative to
// the directory holding index.html. An absolute "/content/x.md" would 404.
const BASE = location.pathname.replace(/[^/]*$/, "");

const el = {
  content: document.getElementById("content"),
  doclist: document.getElementById("doclist"),
  actions: document.getElementById("actions"),
  auth: document.getElementById("auth"),
};

// Bypass the browser cache so an edit committed on GitHub shows up on reload
// rather than whenever the cache feels like expiring.
async function fetchText(path) {
  const res = await fetch(BASE + path, { cache: "no-cache" });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.text();
}

async function loadIndex() {
  try {
    return JSON.parse(await fetchText(`${CONFIG.contentDir}/index.json`)).docs ?? [];
  } catch {
    return [];
  }
}

function renderNav(docs, current) {
  el.doclist.replaceChildren(
    ...docs.map(({ path, title }) => {
      const li = document.createElement("li");
      const a = document.createElement("a");
      a.href = `#/${path.replace(/\.md$/, "")}`;
      a.textContent = title || path;
      if (path === current) a.className = "active";
      li.append(a);
      return li;
    }),
  );
}

// ---- auth bar ----------------------------------------------------------

async function renderAuth() {
  const token = getToken();
  if (!token) {
    el.auth.innerHTML = `<button id="signin">Add token</button>`;
    document.getElementById("signin").onclick = showTokenForm;
    return;
  }
  el.auth.innerHTML = `<span id="who">checking…</span> <button id="signout">Forget</button>`;
  document.getElementById("signout").onclick = () => {
    setToken("");
    renderAll();
  };
  try {
    document.getElementById("who").textContent = await whoami();
  } catch {
    document.getElementById("who").textContent = "invalid token";
  }
}

function showTokenForm() {
  const { owner, repo } = repoCoords();
  el.content.innerHTML = `<h1>Add a token</h1>
    <p>Editing on this site needs a fine-grained personal access token. It proves
    who you are; it does not grant access. Write permission still comes from
    being a collaborator on <code>${owner}/${repo}</code>.</p>
    <ol>
      <li>Open <a href="https://github.com/settings/personal-access-tokens/new" target="_blank" rel="noopener">Settings, Developer settings, Fine-grained tokens</a>.</li>
      <li>Resource owner <code>${owner}</code>, repository access: only <code>${repo}</code>.</li>
      <li>Repository permissions, Contents: <strong>Read and write</strong>.</li>
      <li>Paste it below.</li>
    </ol>
    <p><input id="token" type="password" size="48" placeholder="github_pat_..."> <button id="savetoken">Save</button></p>
    <p class="hint">Kept in sessionStorage, so it is discarded when you close the tab.</p>`;
  document.getElementById("savetoken").onclick = () => {
    setToken(document.getElementById("token").value.trim());
    location.hash = "#/";
    renderAll();
  };
}

// ---- routing -----------------------------------------------------------

const withExt = (p) => (p.endsWith(".md") ? p : `${p}.md`);

function renderHome(docs) {
  el.content.innerHTML = `<h1>leaners</h1>
    <p>Notes on software verification with Lean and Rust. Every page is a
    Markdown file in the repository, fetched and rendered in your browser.</p>
    <ul>${docs
      .map((d) => `<li><a href="#/${d.path.replace(/\.md$/, "")}">${d.title || d.path}</a></li>`)
      .join("")}</ul>`;
}

function renderActions(docPath) {
  const { owner, repo, branch } = repoCoords();
  const signedIn = Boolean(getToken());
  const parts = [];

  if (docPath) {
    parts.push(
      signedIn
        ? `<a href="#/edit/${docPath.replace(/\.md$/, "")}">Edit</a>`
        : `<a href="https://github.com/${owner}/${repo}/edit/${branch}/${CONFIG.contentDir}/${docPath}" target="_blank" rel="noopener">Edit on GitHub</a>`,
    );
  }
  parts.push(
    signedIn
      ? `<a href="#/new">New page</a>`
      : `<a href="https://github.com/${owner}/${repo}/new/${branch}/${CONFIG.contentDir}" target="_blank" rel="noopener">New on GitHub</a>`,
  );
  el.actions.innerHTML = owner && repo ? parts.join(" ") : "";
}

async function route() {
  const path = location.hash.replace(/^#\/?/, "").trim();
  const docs = await loadIndex();

  if (path === "new") {
    renderNav(docs, null);
    renderActions(null);
    return getToken() ? newView(el.content) : showTokenForm();
  }

  if (path.startsWith("edit/")) {
    const docPath = withExt(path.slice(5));
    renderNav(docs, docPath);
    renderActions(null);
    return getToken() ? editView(el.content, docPath) : showTokenForm();
  }

  if (!path) {
    renderNav(docs, null);
    renderActions(null);
    return renderHome(docs);
  }

  const docPath = withExt(path);
  renderNav(docs, docPath);
  renderActions(docPath);
  try {
    el.content.innerHTML = await renderMarkdown(await fetchText(`${CONFIG.contentDir}/${docPath}`));
  } catch (err) {
    el.content.innerHTML = `<h1>Not found</h1>
      <p>Could not load <code>${CONFIG.contentDir}/${docPath}</code> (${err.message}).</p>
      <p><a href="#/">Back to the index</a></p>`;
  }
}

function renderAll() {
  renderAuth();
  route();
}

addEventListener("hashchange", route);
renderAll();
