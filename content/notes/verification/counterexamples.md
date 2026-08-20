# Breaking the proofs: counterexamples

The verification workflow is: the austere Rust in `verified/src/` is
mechanically translated to Lean by charon and Aeneas (`make extract`), and the
static Lean in `proofs/Leaners/` proves things about that translation. This page documents
concrete modifications to the Rust (and to the Lean) that make the pipeline
fail, where exactly each one fails, and what that teaches about which layer
defends against what.

## The pipeline

The complete run, from Rust sources to checked proofs and a shipped binary:

```bash
./verify.sh [--local-wasm]
```

Which runs the following steps:

```bash
make lint          # rustfmt + clippy, warnings are errors
cargo test --manifest-path verified/Cargo.toml   # unit, property and security tests
make extract       # Rust -> LLBC (charon) -> proofs/Extracted/*.lean (aeneas)
make proofs        # lake build: specs, refinement proofs, ladder theorems
make crosscheck    # Lean spec vs Rust binary on tests/vectors.txt, byte for byte
make wasm          # the shipped artifact, reproducibly built
make manifest      # re-record hashes binding wasm + sources + extraction
```

After any change to `verified/src/`, the minimal loop is
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


## Counterexample 1: stop escaping the apostrophe

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


## Counterexample 2: restructure a branch (verified)

Deleting the whole `&` arm (rather than changing what it emits) fails
earlier and differently: the extracted decision tree loses a level, and the
refinement *proof script* no longer matches the program it walks:

```
error: Leaners/Refine.lean:97: Tactic `split` failed:
  Could not split an `if` or `match` expression in the goal
```

The proof breaks *mechanically* before it breaks *logically*. The
proof script mirrors the shape of the extracted code, so a reshaped function
needs its refinement proof re-walked even when the new code is correct.

## Counterexample 3: the missing semicolon

Change the `&` entity to `&amp` without the terminating semicolon, **in both
the Rust and the spec**, so the two still agree:

```rust
    if b == b'&' {
        push_all(out, b"&amp");   // was b"&amp;"
```

```lean
  if b = amp then [38#u8, 97#u8, 109#u8, 112#u8]   -- was ... 59#u8]
```

Extraction: fine. The whole refinement layer:
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

## Counterexample 4: widen the allowlist, spec side

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

