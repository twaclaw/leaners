# Local helpers. None of this is required to deploy: GitHub Pages serves this
# repository directly, so `git push` is the deploy. See design.md section 9.

PY := uv run --no-project tools/leaners/cli.py
PORT ?= 8000

# Extraction toolchain. Override AENEAS_DIR if your checkout lives elsewhere.
# Both are built from source: aeneas is OCaml, charon is a rustc driver on a
# pinned nightly, and the pair must match aeneas/charon-pin.
AENEAS_DIR ?= /opt/repos/toolchains/aeneas
CHARON := $(AENEAS_DIR)/charon/bin/charon
AENEAS := $(AENEAS_DIR)/bin/aeneas
LLBC := verified/target/llbc/leaners_render.llbc

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

.PHONY: serve
serve: ## Preview at http://127.0.0.1:8000
	@$(PY) serve --port $(PORT)

.PHONY: index
index: ## Regenerate content/index.json from the tree
	@$(PY) index

.PHONY: sort
sort: ## Discard hand-ordering in index.json, sort by path
	@$(PY) index --sort

.PHONY: check
check: ## Validate the manifest and internal links
	@$(PY) index --check
	@$(PY) check

.PHONY: wasm
wasm: ## Build the Rust renderer and copy it into pkg/
	@cargo build --release --manifest-path verified/wasm/Cargo.toml --target wasm32-unknown-unknown
	@mkdir -p pkg
	@cp verified/wasm/target/wasm32-unknown-unknown/release/leaners_wasm.wasm pkg/render.wasm
	@ls -l pkg/render.wasm

.PHONY: extract
extract: ## Re-extract the Lean model from the Rust via charon + aeneas
	@test -x "$(CHARON)" || { \
		echo "charon not found at $(CHARON)"; \
		echo "build it with: cd $(AENEAS_DIR) && make setup-charon"; exit 1; }
	@test -x "$(AENEAS)" || { \
		echo "aeneas not found at $(AENEAS)"; \
		echo "build it with: cd $(AENEAS_DIR) && make"; exit 1; }
	@mkdir -p verified/target/llbc proofs/Extracted
	@# `-- --lib` is load-bearing: without it charon also walks the bin targets and
	@# the last one wins, so you get a model of tests/vectors.rs instead of the
	@# library. adapt.rs is excluded because it is the unverified frontend and
	@# pulls in pulldown-cmark.
	@#
	@# ast.rs and render.rs are excluded for a harder reason: Inline::Emph(Vec<Inline>)
	@# recurses through Vec, and Lean's kernel rejects the resulting nested
	@# inductive ("non valid occurrence of the datatypes being declared"). Ladder
	@# steps 5 and 6 stay on the hand-written model until the Ast is reshaped to
	@# recurse through Box, or flattened into an event stream.
	@cd verified && "$(CHARON)" cargo --preset=aeneas \
		--exclude 'crate::adapt::_' --exclude 'crate::markdown_to_html' \
		--exclude 'crate::ast::_' --exclude 'crate::render::_' \
		--dest-file target/llbc/leaners_render.llbc -- --lib
	@"$(AENEAS)" -backend lean "$(LLBC)" -dest proofs/Extracted
	@echo "extracted into proofs/Extracted. Review the diff: that is how you"
	@echo "notice a Rust change that altered the model's meaning."

.PHONY: proofs
proofs: ## Build the Lean model and its proofs
	@cd proofs && lake build

.PHONY: crosscheck
crosscheck: ## Verify the Lean model agrees with the Rust on tests/vectors.txt
	@mkdir -p .crosscheck
	@cargo run --release --quiet --manifest-path verified/Cargo.toml --bin vectors \
		> .crosscheck/rust.txt
	@cd proofs && lake build vectors >/dev/null
	@cd proofs && ./.lake/build/bin/vectors ../verified/tests/vectors.txt \
		> ../.crosscheck/lean.txt
	@diff -u .crosscheck/rust.txt .crosscheck/lean.txt \
		&& echo "model matches the Rust on $$(grep -c '' verified/tests/vectors.txt) vectors"

.PHONY: figures
figures: ## Re-export figures/*.svg from their .drawio sources
	@# --embed-svg-fonts false keeps the file ~20kB instead of ~470kB, and the
	@# .drawio must avoid whiteSpace=wrap: with it, drawio emits labels as
	@# foreignObject, which browsers refuse to render inside an <img>.
	@for f in figures/*.drawio; do \
		drawio --export --format svg --theme light --embed-svg-fonts false \
			--output "$${f%.drawio}.svg" "$$f" || exit 1; \
	done

.PHONY: manifest
manifest: ## Record hashes binding pkg/render.wasm to the sources it came from
	@$(PY) manifest

.PHONY: manifest-check
manifest-check: ## Verify pkg/ still matches the sources recorded in build-manifest.json
	@$(PY) manifest --check

.PHONY: verify
verify: ## Full integrity check: rebuild, re-extract, compare hashes, run proofs
	@./verify.sh

.PHONY: publish
publish: ## Publish to GitHub Pages
	@$(PY) publish -m "$(or $(M),Update content)"

.PHONY: setup
setup: ## One-time: point this repo at a GitHub remote
	@test -n "$(REPO)" || { echo "usage: make setup REPO=owner/name"; exit 1; }
	@git remote add origin "git@github.com:$(REPO).git" 2>/dev/null \
		|| git remote set-url origin "git@github.com:$(REPO).git"
	@git branch -M main
	@echo "remote set to $(REPO)."
	@echo "Next: push, then enable Pages (Settings > Pages > Deploy from a branch > main / root)."
