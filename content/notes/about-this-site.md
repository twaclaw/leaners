# About this site

Notes on verifying software with Lean and Rust. The site is also its own worked
example: the code that renders these pages is the code being verified.

![Architecture of this site](figures/architecture.svg)

## Markdown is content, not build input

Every page is a `.md` file in the repository, and nothing compiles it ahead of
time. The browser fetches the Markdown and renders it client side, so committing
a file publishes it and GitHub Pages serves the repository as it stands. There
is no build step for content.

The only bookkeeping is `content/index.json`, which
lists the pages and fixes their order in the sidebar. A CI job regenerates it
when a page is added through the browser.

The trade is that rendering moves into the reader's browser for *all* content.

## The renderer is Rust, compiled to WebAssembly

`pkg/render.wasm` is built from the Rust in `verified/` and committed. The
interface is deliberately raw: four exported functions and linear memory, no
`wasm-bindgen`.

### The trust boundary is the Ast, not the input

```
markdown bytes --[ pulldown-cmark, UNVERIFIED ]--> Ast --[ verified ]--> HTML bytes
                                                   ^
                                             trust boundary
```

The same split, drawn as the compiler it is:

![The renderer as a compiler: unverified frontend, typed IR, verified backend](figures/renderer-pipeline.svg)

Verifying a Markdown parser would be a more extensive project, since `CommonMark` is
several hundred special cases. So the parser is off the shelf and unverified,
**and only the `Ast -> HTML` half is verified.**

This compromise is designed to be stronger than verifying the parser: the
theorem to aim at quantifies over every `Ast`, so however buggy the parser is,
all it can do is hand `render` some `Ast`.

```lean
-- The target of the ladder. Not proved yet.
theorem render_safe : forall (a : Ast), no_input_derived_markup (render a)
```

A proof over all `Ast` values would make the guarantee unconditional across a
component nobody verified. That is the design the ladder is built toward, but
this particular theorem is not discharged yet. What is proved today is the layer
it rests on: `escape` emits no `<`, and `render` routes every input-derived byte
through `escape`. Lifting the first fact across `render` is the step that
remains, and until it is written the render-level guarantee is carried by the
tests in `verified/tests/`, not by a proof.

### Austere Rust

The modules on the verified side, `render.rs`, `escape.rs`, `slug.rs` and
`highlight.rs`, are written for a translator rather than for a human: `Vec<u8>`
instead of `String`, indexed `while` loops, no iterators, no closures, no traits,
no `unsafe`. Idiomatic Rust does not survive extraction, and the style is
miserable to retrofit later.

Syntax highlighting lives on that side too, for the same reason. A JavaScript
highlighter would rewrite the HTML the renderer had just produced, putting
unverified code back inside the boundary. Done in Rust it inherits the property
instead: every byte still leaves through `escape`, and the only markup it can
emit is a closed set of five span tags.

## What is verified, and how

`charon` compiles the Rust to LLBC, a low-level borrow calculus. `aeneas` turns
that into a pure functional model in Lean, and the proofs are written against
the model.

This works because Rust's ownership discipline has already ruled out aliasing,
so the Lean side sees pure functions over values, with no heap and no separation
logic. Rust removes the aliasing reasoning and Lean does the mathematics. The
two fit together for that reason, not because they share any foundation: Rust
has no dependent types at all.

### How a run proceeds

1. `charon` compiles the austere modules to LLBC and `aeneas` writes the Lean
   model into `proofs/Extracted/`. Only the parser frontend is excluded.
   `ast.rs` and `render.rs` were excluded too while the `Ast` was a tree
   recursing through `Vec`, which Lean's kernel rejects as a nested inductive
   once extracted; the `Ast` is a flat event stream now, so the renderer and
   its input type extract with the rest.
2. `lake` builds that generated model together with the hand-written files
   beside it: `Leaners/Spec.lean`, the pure specs; `Leaners/Refine.lean`, one
   theorem per extracted function proving it computes its spec; and
   `Leaners/Proofs/`, the properties below, stated about the specs and carried
   to the extracted model by the refinement.
3. The Rust binary and the Lean model are run over the same vector file and
   their output diffed, so the two are seen to agree on concrete inputs and not
   only in the proofs.
4. `build-manifest.json` hashes the Rust sources, the extracted Lean and the
   shipped `.wasm` together. That is what catches a Rust edit which never
   reached the extraction the proofs are about.

The properties, in increasing order of difficulty:

| # | Function | Property | Status |
|---|---|---|---|
| 0 | `escape_byte` | output contains no `<`, `>`, `&`, `"` except as entities | proved |
| 1 | `escape` | `unescape (escape s) = s`, the round trip | proved |
| 2 | `slugify` | charset invariant, and `slugify (slugify s) = slugify s` | charset proved, idempotence tests only |
| 3 | `assign_slugs` | anchor ids in a document contain no duplicates | tests only |
| 4 | `sanitize_url` | scheme is one of `http`, `https`, `mailto`, relative | proved |
| 5 | `render` | every tag opened is closed, correctly nested | tests only |
| 6 | `render` | every `<` in the output was emitted by `render`, never derived from input | tests only |

Steps 4 and 6 are the two that are not decoration. Every document here becomes
DOM in somebody's browser, so an escaping bug would be a live XSS vector on a
site about correctness.

## Remaining work

Four of the seven rungs are machine-checked against the extracted model. The
others are covered by `verified/tests/` while their proofs are written, in
roughly this order of weight:

- **Step 6, no input-derived markup in `render`.** The headline property, and
  the reason the unverified parser is meant to be harmless. The extracted model
  of `render` now exists; what remains is to lift the `escape` no-`<` lemma up
  through `render_inline`, `render_inlines` and `render_block`, where every case
  either emits a fixed tag literal or sends input bytes through `escape`.
- **Step 5, tag balance.** Every open tag closed and correctly nested. On the
  flat event stream this holds for balanced streams, so the statement carries a
  well-formedness hypothesis that `adapt` discharges by construction.
- **Step 2, `slugify` idempotence.** The charset half is proved; that
  `slugify (slugify s) = slugify s` is checked in `verified/tests/props.rs`.
- **Step 3, anchor id uniqueness.** That `assign_slugs` never repeats an id in a
  document is a property test today, not yet a theorem.

Stated plainly: the architecture is built so the top-level theorem is provable
over every `Ast`, but that theorem is still a target, and the strongest safety
claim on this page currently rests on tests for the render layer rather than on
a proof.
