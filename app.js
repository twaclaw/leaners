import { CONFIG } from "./config.js";
import { renderMarkdown } from "./renderer.js";

// Pages serves project sites from /REPO/, so every fetch must be relative to
// the directory holding index.html. An absolute "/content/x.md" would 404.
const BASE = location.pathname.replace(/[^/]*$/, "");

const el = {
  content: document.getElementById("content"),
  doclist: document.getElementById("doclist"),
  actions: document.getElementById("actions"),
};

function repoCoords() {
  let { owner, repo, branch } = CONFIG;
  if (!owner || !repo) {
    const host = location.hostname.match(/^([^.]+)\.github\.io$/);
    const seg = location.pathname.split("/").filter(Boolean)[0];
    if (host) {
      owner ||= host[1];
      repo ||= seg || `${host[1]}.github.io`;
    }
  }
  return { owner, repo, branch: branch || "main" };
}

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

function renderHome(docs) {
  el.content.innerHTML = `<h1>leaners</h1>
    <p>Notes on software verification with Lean and Rust. Every page is a
    Markdown file in the repository, fetched and rendered in your browser.</p>
    <ul>${docs
      .map((d) => `<li><a href="#/${d.path.replace(/\.md$/, "")}">${d.title || d.path}</a></li>`)
      .join("")}</ul>`;
}

// Editing happens on github.com, which handles identity and permissions for
// us: collaborators commit directly, everyone else gets a fork and a PR.
function renderActions(docPath) {
  const { owner, repo, branch } = repoCoords();
  if (!owner || !repo) {
    el.actions.innerHTML = "";
    return;
  }
  const base = `https://github.com/${owner}/${repo}`;
  const links = [];
  if (docPath) {
    links.push(
      `<a href="${base}/edit/${branch}/${CONFIG.contentDir}/${docPath}" target="_blank" rel="noopener">Edit this page</a>`,
    );
  }
  links.push(
    `<a href="${base}/new/${branch}/${CONFIG.contentDir}" target="_blank" rel="noopener">New page</a>`,
  );
  el.actions.innerHTML = links.join(" ");
}

async function route() {
  const path = location.hash.replace(/^#\/?/, "").trim();
  const docs = await loadIndex();

  if (!path) {
    renderNav(docs, null);
    renderActions(null);
    renderHome(docs);
    return;
  }

  const docPath = path.endsWith(".md") ? path : `${path}.md`;
  renderNav(docs, docPath);
  renderActions(docPath);

  try {
    el.content.innerHTML = await renderMarkdown(await fetchText(`${CONFIG.contentDir}/${docPath}`));
  } catch (err) {
    el.content.innerHTML = `<h1>Cannot show this page</h1>
      <p><code>${CONFIG.contentDir}/${docPath}</code>: ${err.message}</p>
      <p><a href="#/">Back to the index</a></p>`;
  }
}

addEventListener("hashchange", route);
route();
