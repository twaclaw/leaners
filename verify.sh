#!/usr/bin/env bash
# Rebuilds the shipped artifact and the extracted model from source, then checks
# both against build-manifest.json.
#
# The wasm build is bit-reproducible from a clean target directory *on the
# machine that recorded the manifest*, so a hash difference here means the
# sources and the shipped binary really have drifted, not that the build is
# noisy. Across hosts it is not: rustc embeds paths that depend on which
# toolchain components are installed, which is why CI checks the source hashes
# exactly and binds the binary by behaviour instead. See .github/workflows.
set -euo pipefail
cd "$(dirname "$0")"

# --local-wasm keeps the committed binary canonical and drops only the claim
# that cannot hold away from the machine that recorded it: instead of demanding
# that the rebuild reproduce the recorded bytes, it demands that the rebuild and
# the committed binary render the corpus identically, which is the comparison CI
# makes. Everything else, sources and extracted model included, is checked the
# same way in both modes.
local_wasm=
case "${1:-}" in
  --local-wasm) local_wasm=1 ;;
  "") ;;
  *) echo "usage: $0 [--local-wasm]" >&2; exit 2 ;;
esac

# Pin the compiler to the one build-manifest.json records, the way CI does from
# the same field. The bit-for-bit claim above is about the machine that recorded
# the manifest, and that is only meaningful if the rebuild uses the recorded
# rustc rather than whatever rustup default happens to be active today. The
# aeneas side needs no such treatment: `make extract` resolves its toolchain
# from the rev in the same file, and bin/aeneas is a native binary that wants
# nothing from opam at run time.
rustc_version=$(python3 -c \
  "import json; print(json.load(open('build-manifest.json'))['toolchains']['rustc'].split()[1])")
if ! rustup run "$rustc_version" rustc --version >/dev/null 2>&1; then
  echo "build-manifest.json records rustc $rustc_version, which is not installed:" >&2
  echo "  rustup toolchain install $rustc_version --profile minimal --component clippy --component rustfmt" >&2
  exit 1
fi
if ! rustup target list --installed --toolchain "$rustc_version" | grep -qx wasm32-unknown-unknown; then
  echo "rustc $rustc_version has no wasm32 target:" >&2
  echo "  rustup target add wasm32-unknown-unknown --toolchain $rustc_version" >&2
  exit 1
fi
if ! rustup component list --installed --toolchain "$rustc_version" | grep -q "^rust-src"; then
  # Not pedantry: without rust-src the toolchain's own sources are not on disk,
  # a panic location in core resolves differently, and the binary comes out 64
  # bytes shorter. The byte comparison below is meaningless with it missing.
  echo "rustc $rustc_version has no rust-src; the wasm will not match the recorded bytes:" >&2
  echo "  rustup component add rust-src --toolchain $rustc_version" >&2
  exit 1
fi
export RUSTUP_TOOLCHAIN="$rustc_version"
echo "==> building with the recorded rustc $rustc_version"

echo
echo "==> lint"
make lint

echo
echo "==> rebuilding pkg/render.wasm from a clean target"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# The reference is the committed blob, not whatever the working tree holds: a
# tree still carrying an earlier rebuild would otherwise be compared to itself,
# and the comparison would pass by saying nothing.
if git show HEAD:pkg/render.wasm > "$tmp/committed.wasm" 2>/dev/null; then
  cmp -s "$tmp/committed.wasm" pkg/render.wasm \
    || echo "note: the working tree's pkg/render.wasm is not the committed one" >&2
else
  echo "note: not a git checkout, taking the working tree's pkg/render.wasm" >&2
  cp pkg/render.wasm "$tmp/committed.wasm"
fi
cp pkg/render.wasm "$tmp/tree.wasm"
rm -rf verified/wasm/target
make wasm
cp pkg/render.wasm "$tmp/rebuilt.wasm"
# Verifying must not rewrite a tracked file: leave the tree exactly as found.
cp "$tmp/tree.wasm" pkg/render.wasm

echo
if [ -n "$local_wasm" ]; then
  echo "==> the rebuild must render the corpus exactly like the committed wasm"
  command -v node >/dev/null || {
    echo "node is required for the behavioural comparison" >&2; exit 1; }
  mapfile -t docs < <(find content -name '*.md' | sort)
  docs+=(verified/tests/vectors.txt)
  node tools/wasm-render.mjs "$tmp/committed.wasm" "${docs[@]}" > "$tmp/committed.html"
  node tools/wasm-render.mjs "$tmp/rebuilt.wasm" "${docs[@]}" > "$tmp/rebuilt.html"
  if diff -u "$tmp/committed.html" "$tmp/rebuilt.html" > "$tmp/render.diff"; then
    echo "identical HTML on ${#docs[@]} documents"
    wasm_result="renders the corpus exactly like the committed binary"
  else
    echo "the rebuild does not render like the committed wasm:" >&2
    head -40 "$tmp/render.diff" >&2
    exit 1
  fi
else
  echo "==> the rebuild must match the artifact hash in build-manifest.json"
  recorded=$(python3 -c \
    "import json; print(json.load(open('build-manifest.json'))['files']['pkg/render.wasm'])")
  rebuilt=$(sha256sum "$tmp/rebuilt.wasm" | cut -d" " -f1)
  if [ "$recorded" = "$rebuilt" ]; then
    echo "pkg/render.wasm reproduces bit for bit"
    wasm_result="reproduces the recorded bytes"
  else
    echo "the rebuild does not reproduce the recorded artifact:" >&2
    echo "  recorded $recorded" >&2
    echo "  rebuilt  $rebuilt" >&2
    echo "Bytes are only expected to match on the machine that recorded the" >&2
    echo "manifest. Elsewhere, rerun with --local-wasm to make the comparison" >&2
    echo "CI makes: same sources, same rendered output, different bytes." >&2
    exit 1
  fi
fi

echo
echo "==> re-extracting the Lean model from the Rust"
if make extract; then
  extract_result="re-extracted from the Rust, hash checked against the manifest"
else
  extract_result="SKIPPED, proofs/Extracted was NOT regenerated from the Rust"
  echo "extraction unavailable on this machine; the .wasm check below still applies" >&2
fi

echo
echo "==> which lines of verified/src the extracted model covers"
# Aeneas stamps each definition with the source span it came from; the report
# maps those spans back onto the Rust and says, line by line, what is in the
# model, what the Makefile excludes on purpose, and what fell through. It reads
# the model on disk, so it is meaningful in both branches above: freshly
# regenerated, or the committed one when extraction was skipped.
if coverage_table=$(make -s extract-report 2>&1); then
  echo "$coverage_table"
  coverage_result=$(printf '%s\n' "$coverage_table" | tail -n 1 | sed 's/^coverage: //')
else
  echo "$coverage_table" >&2
  coverage_result="report unavailable"
fi

echo
echo "==> comparing the sources and the extracted model against build-manifest.json"
make manifest-check MANIFEST_CHECK_FLAGS=--sources-only

echo
echo "==> the proofs themselves"
make proofs
make crosscheck

# Reaching here means every step above passed, but "passed" is not the same for
# all of them: the extraction is allowed to be missing, so say plainly which
# checks actually ran rather than leaving the last line of output to imply it.
echo
echo "==> verify.sh passed"
echo "  wasm:       $wasm_result"
echo "  sources:    match build-manifest.json"
echo "  extraction: $extract_result"
echo "  coverage:   $coverage_result"
echo "  proofs:     lake build clean"
echo "  crosscheck: the Lean model agrees with the Rust on the vector corpus"
case "$extract_result" in
  SKIPPED*) echo
            echo "One check did not run. The model the proofs are about was not"
            echo "regenerated, so nothing here rules out a Rust edit that changed it." ;;
esac
