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

## How verification proceeds

`make verify` runs `./verify.sh`, which performs these steps in order. Each one
names where its output lands, because the next one reads it from there.

1. **Rebuild the artifact.** After `make lint`, `make wasm` compiles
   `verified/` for `wasm32-unknown-unknown` and produces `pkg/render.wasm`, the
   binary the site loads. The rebuild is compared against the artifact hash in
   `build-manifest.json`, and the tree is left as it was found.
2. **Extract.** `make extract` runs charon over `verified/` to produce
   `verified/target/llbc/leaners_render.llbc`, then aeneas turns that LLBC into
   `proofs/Extracted/LeanersRender.lean`. Only the austere backend is
   extracted: `adapt.rs` is excluded as the unverified frontend, and `ast.rs`
   and `render.rs` because Lean's kernel rejects the nested inductive that
   `Inline` becomes.
3. **Compare against the manifest.** `build-manifest.json` records hashes for
   the Rust sources and the extracted Lean, plus the toolchain revisions that
   produced them. This is the step that notices a Rust edit which never made it
   into the extraction the proofs are about.
4. **Prove.** `make proofs` builds `proofs/` with lake, compiling the generated
   model together with the hand-written files beside it: `Leaners/Spec.lean`
   (pure specs, the only hand-written definitions), `Leaners/Refine.lean` (one
   theorem per extracted function, proving it computes its spec), and
   `Leaners/Proofs/` (the safety theorems, stated about the specs and carried
   to the extracted model by the refinement).
5. **Crosscheck.** `make crosscheck` runs the Rust `vectors` binary and the
   Lean `vectors` executable over `verified/tests/vectors.txt` and diffs their
   output, so the model and the code are seen to agree on concrete inputs and
   not only in the proofs.

The run ends with a summary naming each check. Watch the `extraction:` line: a
missing charon or aeneas is not fatal, so a run can pass with the model left
un-regenerated, and the summary says so when that happens.

Byte equality for the WASM only holds on the machine that recorded the
manifest. Anywhere else use `./verify.sh --local-wasm`, which keeps the
committed binary canonical and substitutes the comparison CI makes: the rebuild
must render the corpus identically. CI runs the cheap half on every push,
checking the source hashes exactly and binding the committed WASM by behaviour.
The Lean build stays local, since Mathlib does not belong in a one-minute gate.
