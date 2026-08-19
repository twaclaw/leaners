#!/usr/bin/env bash
# Rebuilds the shipped artifact and the extracted model from source, then checks
# both against build-manifest.json.
#
# This is what turns the binding between proof and artifact from a social claim
# ("the person who built it says so") into a mechanical one. Without it, someone
# could commit a .wasm built from different code and nothing would notice. See
# design.md section 9.
#
# The wasm build is bit-reproducible from a clean target directory *on the
# machine that recorded the manifest*, so a hash difference here means the
# sources and the shipped binary really have drifted, not that the build is
# noisy. Across hosts it is not: rustc embeds paths that depend on which
# toolchain components are installed, which is why CI checks the source hashes
# exactly and binds the binary by behaviour instead. See .github/workflows.
set -euo pipefail
cd "$(dirname "$0")"

echo "==> lint"
make lint

echo
echo "==> rebuilding pkg/render.wasm from a clean target"
rm -rf verified/wasm/target
make wasm

echo
echo "==> re-extracting the Lean model from the Rust"
if make extract; then
  :
else
  echo "extraction unavailable on this machine; the .wasm check below still applies" >&2
fi

echo
echo "==> comparing everything against build-manifest.json"
make manifest-check

echo
echo "==> the proofs themselves"
make proofs
make crosscheck
