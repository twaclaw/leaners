import { CONFIG } from "./config.js";
import { renderMarkdown } from "./renderer.js";

// Pages serves project sites from /REPO/, so every fetch must be relative to
// the directory holding index.html. An absolute "/content/x.md" would 404.
const BASE = location.pathname.replace(/[^/]*$/, "");

const el = {
  content: document.getElementById("content"),
  doclist: document.getElementById("doclist"),
  edit: document.getElementById("edit"),
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
    const { docs } = JSON.parse(await fetchText(`${CONFIG.contentDir}/index.json`));
    return docs ?? [];
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
  const ul = docs
    .map((d) => `<li><a href="#/${d.path.replace(/\.md$/, "")}">${d.title || d.path}</a></li>`)
    .join("");
  el.content.innerHTML = `<h1>leaners</h1>
    <p>Notes on software verification with Lean and Rust. Every page here is a
    Markdown file in the repository, fetched and rendered in your browser.</p>
    <ul>${ul}</ul>`;
}

function setEditLink(docPath) {
  const { owner, repo, branch } = repoCoords();
  if (!docPath || !owner || !repo) {
    el.edit.style.visibility = "hidden";
    return;
  }
  el.edit.style.visibility = "visible";
  el.edit.href = `https://github.com/${owner}/${repo}/edit/${branch}/${CONFIG.contentDir}/${docPath}`;
}

async function route() {
  const slug = location.hash.replace(/^#\/?/, "").trim();
  const docs = await loadIndex();

  if (!slug) {
    renderNav(docs, null);
    setEditLink(null);
    renderHome(docs);
    return;
  }

  const docPath = slug.endsWith(".md") ? slug : `${slug}.md`;
  renderNav(docs, docPath);
  setEditLink(docPath);

  try {
    const src = await fetchText(`${CONFIG.contentDir}/${docPath}`);
    el.content.innerHTML = await renderMarkdown(src);
  } catch (err) {
    el.content.innerHTML = `<h1>Not found</h1>
      <p>Could not load <code>${CONFIG.contentDir}/${docPath}</code> (${err.message}).</p>
      <p><a href="#/">Back to the index</a></p>`;
  }
}

addEventListener("hashchange", route);
route();
