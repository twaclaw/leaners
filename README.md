# leaners

Notes on software verification with Lean and Rust. The site is plain Markdown,
fetched and rendered in the browser, served straight from this repository by
GitHub Pages.

**There is no build step for content.** Content is data, not build input, so committing a
Markdown file publishes it. That is what makes "edit online, see it live" work.

The renderer is Rust compiled
to WASM. Its escaping, URL and slug functions are mechanically extracted to
Lean by charon and Aeneas, each extraction is proved to refine a small
hand-written spec, and the safety properties (the escape round trip, the URL
scheme allowlist, the slug charset) are theorems about that extracted model,
which is to say about the code that ships. The `Ast -> HTML` renderer itself
is written in the same extraction style but not yet extracted.

## Deploying

```sh
make setup REPO=owner/leaners     # one time: point at a GitHub remote
git add -A && git commit -m "Initial site" && git push -u origin main
```

Then **Settings > Pages > Deploy from a branch > `main` / `/ (root)`**.

After that, `make publish` validates and pushes. Editing through GitHub's web
editor needs no local tooling at all.

## Local development

```sh
make serve      # preview at http://127.0.0.1:8000
make index      # regenerate content/index.json from the tree
make check      # validate the manifest and internal links
```

A server is required for preview: ES modules do not load over `file://`.

## Layout

| Path | Purpose |
|---|---|
| `index.html`, `app.js`, `style.css` | the shell: routing, nav, fetch |
| `renderer.js` | the renderer seam, driving the WASM module in `pkg/` |
| `config.js` | repo coordinates for "Edit this page" links |
| `content/` | the documents, plus `index.json` listing them |
| `pkg/` | the committed WASM renderer, rebuilt by `make wasm` |
| `verified/` | the Rust renderer: unverified frontend, austere backend |
| `proofs/` | the Lean models, proofs, and the aeneas-extracted model |
| `tools/` | local Python helpers, not needed to deploy |

`make verify` rebuilds the WASM and the extracted model from source and checks
both against `build-manifest.json`, then runs the proofs and the Rust/Lean
crosscheck. On every push CI checks the source hashes exactly and requires the
committed WASM to render the corpus identically to a rebuild from those
sources.
