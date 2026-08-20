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

// Mirrors normalise() in tools/leaners/cli.py: a hand-added page may be a bare
// "notes/hello.md" string, with the title filled in at the next `make index`.
const normaliseDocs = (docs) =>
  docs.map((d) => (typeof d === "string" ? { path: d } : d)).filter((d) => d?.path);

// Held so renderNav can say what went wrong. A syntax slip in a hand-edited
// manifest would otherwise empty the whole sidebar in silence, which reads as
// "the site is broken" rather than "one file needs a comma removed".
let indexError = null;

async function loadIndex() {
  indexError = null;
  let raw;
  try {
    raw = await fetchText(`${CONFIG.contentDir}/index.json`);
  } catch (err) {
    indexError = `cannot be fetched (${err.message})`;
    return [];
  }
  try {
    return normaliseDocs(JSON.parse(raw).docs ?? []);
  } catch (err) {
    indexError = err.message;
    return [];
  }
}

const stripExt = (path) => path.replace(/\.md$/, "");

function prettify(name) {
  return name.replace(/[-_]/g, " ").replace(/^./, (c) => c.toUpperCase());
}

// Turn the manifest's flat list of paths into the folder tree it describes, so
// the sidebar shows the same shape as content/ on disk.
//
// A folder may have a page of its own. Three spellings work, because moving a
// page under a folder should not force a rename:
//
//   notes/lean.md        a sibling of the notes/lean/ folder
//   notes/lean/lean.md   named after the folder it heads
//   notes/lean/index.md  the conventional index
//
// Anything else is an ordinary page sitting inside its folder.
function buildTree(docs) {
  const node = (label) => ({ label, page: null, items: [] });
  const root = node(null);
  const folders = new Map([["", root]]);
  const parentOf = new Map();
  const placed = new Set();

  // Pass 1 registers a node for every directory before anything is placed, so
  // that notes/lean.md is recognised as the page for notes/lean/ whichever of
  // the two the manifest happens to list first.
  for (const doc of docs) {
    let prefix = "";
    for (const dir of doc.path.split("/").slice(0, -1)) {
      const parent = prefix;
      prefix = prefix ? `${prefix}/${dir}` : dir;
      if (!folders.has(prefix)) {
        folders.set(prefix, node(prettify(dir)));
        parentOf.set(prefix, parent);
      }
    }
  }

  // A folder takes its position from the first manifest line that mentions it,
  // which is what makes manifest order the sidebar's order: move a line and the
  // whole section moves with it. Pages and sections interleave freely.
  const place = (path) => {
    if (path === "" || placed.has(path)) return;
    place(parentOf.get(path));
    placed.add(path);
    folders.get(parentOf.get(path)).items.push({ folder: folders.get(path) });
  };

  for (const doc of docs) {
    const segments = doc.path.split("/");
    const parentPath = segments.slice(0, -1).join("/");
    const folderName = segments.at(-2);
    const stem = stripExt(segments.at(-1));
    const named = folders.get(stripExt(doc.path));
    const parent = folders.get(parentPath);

    if (named && !named.page) {
      named.page = doc;
      place(stripExt(doc.path));
    } else if (folderName && !parent.page && (stem === "index" || stem === folderName)) {
      parent.page = doc;
      place(parentPath);
    } else {
      place(parentPath);
      parent.items.push({ doc });
    }
  }
  return root;
}

function docLink(doc, current) {
  const a = document.createElement("a");
  a.href = `#/${stripExt(doc.path)}`;
  // Same fallback as title_of() in cli.py, so a page added online as a bare
  // path reads as a name rather than as a path until the next reindex.
  a.textContent = doc.title || prettify(stripExt(doc.path.split("/").pop()));
  if (doc.path === current) a.className = "active";
  return a;
}

// A section headed by a page is a link; one that is only a folder is plain
// text, since there is nothing to navigate to.
function sectionLabel(folder, current) {
  if (folder.page) return docLink(folder.page, current);
  const span = document.createElement("span");
  span.className = "section";
  span.textContent = folder.label;
  return span;
}

// Depth first, in manifest order.
function listItems(folder, current) {
  return folder.items.map(({ doc, folder: child }) => {
    const li = document.createElement("li");
    if (doc) {
      li.append(docLink(doc, current));
      return li;
    }
    li.append(sectionLabel(child, current));
    const sub = document.createElement("ul");
    sub.append(...listItems(child, current));
    if (sub.childElementCount) li.append(sub);
    return li;
  });
}

function renderNav(tree, current) {
  const items = listItems(tree, current);
  if (indexError) {
    const li = document.createElement("li");
    li.className = "warn";
    li.textContent = `${CONFIG.contentDir}/index.json ${indexError}`;
    items.unshift(li);
  }
  el.doclist.replaceChildren(...items);
}

function renderHome(tree) {
  const h1 = document.createElement("h1");
  h1.textContent = "leaners";
  const p = document.createElement("p");
  p.textContent =
    "Welcome, dear Leaners!";
  const ul = document.createElement("ul");
  ul.className = "tree";
  ul.append(...listItems(tree, null));
  el.content.replaceChildren(h1, p, ul);
}

// Markdown written for a repository uses document-relative links: ./foo.md,
// ../bar/baz.md. Hash routing puts the document path in the fragment, so the
// browser resolves those against the site root instead of against the document
// and they 404. Rewriting them into routes is what makes the same file readable
// both here and on GitHub.
//
// Only the href attribute of existing anchors is touched, never the markup, so
// the renderer's guarantee that no input-derived `<` reaches the output is
// unaffected: the replacement is a `#/` string built here.
function resolveDocLinks(root, docPath) {
  const dir = docPath.replace(/[^/]*$/, "");
  for (const a of root.querySelectorAll("a[href]")) {
    const raw = a.getAttribute("href");
    // Absolute URLs, in-page anchors and existing routes are already correct.
    if (!raw || raw.startsWith("#") || raw.startsWith("/") || /^[a-z][a-z0-9+.\-]*:/i.test(raw)) {
      continue;
    }
    if (!/\.md($|[?#])/i.test(raw)) continue;
    a.setAttribute("href", `#/${stripExt(normalisePath(dir + raw))}`);
  }
}

// Collapses "." and ".." the way a browser would, without needing a base URL.
function normalisePath(path) {
  const out = [];
  for (const seg of path.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") out.pop();
    else out.push(seg);
  }
  return out.join("/");
}

// Editing happens on github.com, which handles identity and permissions for
// us: collaborators commit directly, everyone else gets a fork and a PR.
//
// Built with DOM APIs rather than innerHTML: docPath comes from the URL
// fragment, so anyone who can get a link clicked chooses its contents. As a
// string in an href attribute it is inert; interpolated into markup it would
// be the exact injection the renderer is verified not to produce.
function renderActions(docPath) {
  const { owner, repo, branch } = repoCoords();
  if (!owner || !repo) {
    el.actions.replaceChildren();
    return;
  }
  const base = `https://github.com/${owner}/${repo}`;
  const link = (href, label) => {
    const a = document.createElement("a");
    a.href = href;
    a.target = "_blank";
    a.rel = "noopener";
    a.textContent = label;
    return a;
  };
  const links = [];
  if (docPath) {
    links.push(link(`${base}/edit/${branch}/${CONFIG.contentDir}/${docPath}`, "Edit this page"));
  }
  links.push(link(`${base}/new/${branch}/${CONFIG.contentDir}`, "New page"));
  el.actions.replaceChildren(...links);
}

async function route() {
  const path = location.hash.replace(/^#\/?/, "").trim();
  const tree = buildTree(await loadIndex());

  if (!path) {
    renderNav(tree, null);
    renderActions(null);
    renderHome(tree);
    return;
  }

  const docPath = path.endsWith(".md") ? path : `${path}.md`;
  renderNav(tree, docPath);
  renderActions(docPath);

  try {
    el.content.innerHTML = await renderMarkdown(await fetchText(`${CONFIG.contentDir}/${docPath}`));
    resolveDocLinks(el.content, docPath);
  } catch (err) {
    // DOM APIs, not innerHTML: docPath is hash-derived, and err.message can
    // quote server responses. Neither may be interpolated into markup.
    const h1 = document.createElement("h1");
    h1.textContent = "Cannot show this page";
    const p = document.createElement("p");
    const code = document.createElement("code");
    code.textContent = `${CONFIG.contentDir}/${docPath}`;
    p.append(code, `: ${err.message}`);
    const back = document.createElement("p");
    const a = document.createElement("a");
    a.href = "#/";
    a.textContent = "Back to the index";
    back.append(a);
    el.content.replaceChildren(h1, p, back);
  }
}

// Clicking a figure opens it over the page. This is shell behaviour and touches
// no markup: the overlay is built with DOM APIs and takes only the `src` and
// `alt` of an image the renderer already emitted, so the guarantee about what
// reaches the document is unaffected. The listener sits on #content, which
// survives every route change, rather than on images that do not.
const lightbox = document.createElement("div");
lightbox.id = "lightbox";
lightbox.setAttribute("role", "dialog");
lightbox.setAttribute("aria-modal", "true");
lightbox.setAttribute("aria-label", "Enlarged figure");
const lightboxImg = document.createElement("img");
lightbox.append(lightboxImg);
document.body.append(lightbox);

function closeLightbox() {
  lightbox.classList.remove("open");
  lightboxImg.removeAttribute("src");
  lightboxImg.removeAttribute("alt");
}

el.content.addEventListener("click", (event) => {
  const img = event.target.closest("img");
  if (!img) return;
  lightboxImg.src = img.currentSrc || img.src;
  lightboxImg.alt = img.alt;
  lightbox.classList.add("open");
});
lightbox.addEventListener("click", closeLightbox);
addEventListener("keydown", (event) => {
  if (event.key === "Escape") closeLightbox();
});

addEventListener("hashchange", route);
route();
