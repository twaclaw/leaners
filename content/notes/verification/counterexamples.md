# Breaking the proofs: counterexamples

The verification workflow is: the austere Rust in `verified/src/` is
mechanically translated to Lean by charon and Aeneas (`make extract`), and the
static Lean in `proofs/Leaners/` proves things about that translation. The
result is only as good as its ability to say **no**. This page documents
concrete modifications to the Rust (and to the Lean) that make the pipeline
fail, where exactly each one fails, and what that teaches about which layer
defends against what. Each counterexample marked *verified* was actually run
against this repository; the failure locations quoted are real.

## The pipeline

The complete run, from Rust sources to checked proofs and a shipped binary:

```bash
make lint          # rustfmt + clippy, warnings are errors
cargo test --manifest-path verified/Cargo.toml   # unit, property and security tests
make extract       # Rust -> LLBC (charon) -> proofs/Extracted/*.lean (aeneas)
make proofs        # lake build: specs, refinement proofs, ladder theorems
make crosscheck    # Lean spec vs Rust binary on tests/vectors.txt, byte for byte
make wasm          # the shipped artifact, reproducibly built
make manifest      # re-record hashes binding wasm + sources + extraction
./verify.sh        # all of the above from clean, against build-manifest.json
```

After any change to `verified/src/`, the minimal honest loop is
`make extract && make proofs && make crosscheck`. Editing the Rust without
re-extracting leaves `proofs/Extracted/` describing code that no longer
exists; `verify.sh` catches that by re-extracting and comparing hashes.

## The two proof layers

Failures land in one of two places, and the distinction is the whole story:

- **`proofs/Leaners/Refine.lean`** proves each extracted function *equals* its
  spec in `proofs/Leaners/Spec.lean`: for example
  `escape.escape input out ⦃ out1 => out1.val = out.val ++ Spec.escape input.val ⦄`.
  This layer breaks whenever the Rust and the spec disagree, in either
  direction.
- **`proofs/Leaners/Proofs/`** proves the safety properties of the specs
  (the ladder: delimiter freedom, the escape round trip, the URL allowlist,
  the slug charset), and restates them as theorems about the extracted
  functions. This layer breaks when code and spec agree on something *wrong*.

An attacker (or a tired maintainer) therefore has to get past both: a change
to the Rust alone fails refinement; a matching change to the spec moves the
contradiction into the ladder theorems; and changing the ladder statements
themselves is the one move no machine catches, which is why the statements
are kept few, short, and in one reviewable file.

## Counterexample 1: stop escaping the apostrophe (verified)

In `verified/src/escape.rs`, delete the `'` branch of `escape_byte`:

```rust
    } else if b == b'"' {
        push_all(out, b"&quot;");
    } else {          // the b'\'' branch is gone
        out.push(b);
    }
```

`make extract` succeeds: this is perfectly good Rust, and extraction checks
types, not intentions. `make proofs` fails in the refinement layer:

```
error: Leaners/Refine.lean:98: unsolved goals
  h : b = 39#u8
  ⊢ ...out.val ++ Spec.escapeByte b...
```

The extracted `escape_byte` now pushes byte 39 raw where `Spec.escapeByte`
says `&#39;`, so `escape_byte_spec` is no longer provable, and everything
above it (`escape_loop_spec`, `escape_spec`, the extracted round trip) falls
with it. This is the common case: **any semantic change to the Rust breaks
refinement first**, because refinement is an equality on outputs.

`cargo test` also fails (`math_is_escaped...`, hostile-vector tests), and
`make crosscheck` reports a byte-level diff on the shared vectors. Three
independent alarms.

## Counterexample 2: restructure a branch (verified)

Deleting the whole `&` arm (rather than changing what it emits) fails
earlier and differently: the extracted decision tree loses a level, and the
refinement *proof script* no longer matches the program it walks:

```
error: Leaners/Refine.lean:97: Tactic `split` failed:
  Could not split an `if` or `match` expression in the goal
```

Worth knowing because it is the failure mode you will see most often in
practice: the proof breaks *mechanically* before it breaks *logically*. The
proof script mirrors the shape of the extracted code, so a reshaped function
needs its refinement proof re-walked even when the new code is correct.

## Counterexample 3: the missing semicolon (verified, the important one)

Change the `&` entity to `&amp` without the terminating semicolon, **in both
the Rust and the spec**, so the two still agree:

```rust
    if b == b'&' {
        push_all(out, b"&amp");   // was b"&amp;"
```

```lean
  if b = amp then [38#u8, 97#u8, 109#u8, 112#u8]   -- was ... 59#u8]
```

Now watch what still succeeds. Extraction: fine. The whole refinement layer:
**fine**, the code faithfully implements the wrong spec. Ladder step 0
(`escapeByte_no_delims`, no `<` `>` `"` `'` in the output): **fine**, `&amp`
contains no delimiter. The build fails only at ladder step 1:

```
✖ Building Leaners.Proofs.Escape
error: Leaners/Proofs/Escape.lean:65: unsolved goals
```

That is `unescapeN_escapeByte`, the `b = amp` case of the round trip
`unescapeN (escape s) = s`: unescaping `&amp` followed by arbitrary text can
no longer recover the original `&`, and Lean refuses exactly there. This is
the mechanical justification for a design claim in `design.md`: *"output has
no `<`" is weak, the round trip is the property that carries the weight*.
A renderer that emits `&amp` is a real bug class (ambiguous parses,
double-escaping), sails through the delimiter check, and only injectivity
catches it.

## Counterexample 4: widen the allowlist, spec side (verified)

Add `data:` to `Spec.isSafeUrl` while leaving the Rust alone. This is the
"prove something easier instead" move. It fails immediately in refinement:

```
✖ Building Leaners.Refine
error: Leaners/Refine.lean:304: unsolved goals    -- is_safe_url_spec
```

The shipped Rust rejects `data:`, the weakened spec accepts it, and the
refinement equality snaps. The point: refinement protects **both
directions**. The spec is not free decoration on top of the code; it is
pinned to the code by theorem.

## Counterexample 5: widen the allowlist, both sides (reasoned)

Add `|| starts_with_ci(url, b"data:")` to the Rust *and* the matching
disjunct to the spec. Refinement would hold again (after re-walking the
proof, as in counterexample 2). Two things now object:

- `rejects_data` in `proofs/Leaners/Proofs/Url.lean` becomes unprovable:
  `isSafeUrl ("data:" ++ rest) = false` is simply false now.
- `isSafeUrl_allowlist` stops compiling as stated, because its conclusion
  enumerates the three permitted schemes. To "fix" it you must edit the
  theorem statement itself, and that edit ("...or the scheme is `data:`") is
  exactly the kind of one-line diff a reviewer reads.

The concrete rejection lemmas (`rejects_javascript`, `rejects_data`,
`rejects_javascript_mixed_case`) exist for this scenario: they are the
attack cases pinned as theorems, so weakening the allowlist cannot be silent.
Also `cargo test dangerous_url_schemes_are_dropped` fails, as does the
crosscheck vector `data:text/html;base64,...`.

## Counterexample 6: slug separator becomes underscore (reasoned)

Change `out.push(b'-')` to `out.push(b'_')` in `slugify` (and `dash` to 95 in
the spec, to keep refinement green). `slugAux_charset` in
`proofs/Leaners/Proofs/Slug.lean` fails: `okByte` says a slug byte is a
lower-case alphanumeric or byte 45, and 95 is neither. As with
counterexample 5, the last line of defense is that `okByte` itself is a
two-line definition a human actually reads.

## Counterexample 7: introduce a panic (reasoned)

Change a loop bound to `i <= input.len()` so the final iteration indexes out
of bounds. The extraction models indexing as fallible: `Vec.index_usize`
returns `fail` past the end, and the `⦃ ⦄` specs are total correctness, where
`fail` satisfies no postcondition. Every refinement theorem over the broken
loop becomes unprovable; no property needs to mention panics for panics to be
excluded. (The `≤ Usize.max` hypotheses on the theorems are the honest edge
of this: they say the proofs hold exactly up to the capacity at which the
real Rust would abort.)

## Counterexample 8: ship without re-extracting (mechanism runs on every verify)

Edit the Rust, rebuild the wasm, re-record `build-manifest.json`, but keep the
stale `proofs/Extracted/` so the old proofs still compile. Locally everything
looks green. `./verify.sh` closes this hole: it re-runs `make extract` and
then compares the fresh extraction against the manifest, so the recorded
hashes can only all match when wasm, Rust, and the extraction the proofs are
about came from the same sources. CI cannot repeat the hash comparison, because
rustc does not promise byte-identical wasm across hosts, so it checks the source
hashes exactly and then renders a corpus through both the committed binary and a
rebuild from those sources, requiring identical HTML. Byte equality is a
same-machine check; behavioural equality is the portable one.

## What no proof here catches

Stated plainly, because a list of counterexamples invites overconfidence:

- **`render.rs`, `ast.rs`, `adapt.rs`**: not extracted yet (the `Ast` recurses
  through `Vec`, which Lean's kernel rejects as a nested inductive). Ladder
  steps 5 and 6, tag balance and "no input-derived `<`", are covered by
  `cargo test` only. Adding a raw-HTML passthrough to `adapt.rs` would void
  the whole design and only the `raw_html_cannot_reach_the_output` test
  would notice.
- **`highlight.rs`**: extracted but nothing is proved about it; the
  closed-span-set property lives in
  `highlighting_emits_no_tag_outside_the_closed_set` (a test).
- **The wasm and JS glue**: `verified/wasm/src/lib.rs` and `renderer.js` are
  outside the model entirely.
- **The statements themselves**: `Spec.lean` (120 lines) and the theorem
  statements in `Proofs/` are the trusted text. A wrong `okByte`, a weakened
  allowlist conclusion, or a vacuous rewrite of the round trip is caught by
  review, not by Lean.
- **The toolchain**: charon, aeneas, rustc, and Lean itself are trusted, with
  versions pinned in `build-manifest.json`. `make crosscheck` exists to keep
  this honest empirically: the same 37 vectors must produce identical bytes
  from the Lean spec and from the compiled Rust.

The open proof obligations, for completeness: slug idempotence, `assign`'s
no-duplicates property (ladder step 3), and the two render theorems. Until
they land, `verified/tests/props.rs` covers them with seeded property tests.
