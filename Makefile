# Local helpers. None of this is required to deploy: GitHub Pages serves this
# repository directly, so `git push` is the deploy. See design.md section 9.

PY := uv run --no-project tools/leaners/cli.py
PORT ?= 8000

# Extraction toolchain. The rev comes from build-manifest.json rather than from
# a fixed path or an exported variable, so a machine that carries several aeneas
# versions cannot quietly extract this repo with the wrong one: the checkout a
# project uses is the one that project recorded. Override AENEAS_DIR if your
# checkouts live somewhere other than the rev-keyed layout below.
#
#   ~/repos/toolchains/aeneas/.src        the clone
#   ~/repos/toolchains/aeneas/<rev>/      a worktree per pinned rev, built there
#
# Both tools are built from source: aeneas is OCaml, charon is a rustc driver on
# a pinned nightly, and the pair must match the worktree's charon-pin.
AENEAS_REV := $(shell python3 -c "import json; print(json.load(open('build-manifest.json'))['toolchains']['aeneas_rev'])" 2>/dev/null)
AENEAS_DIR ?= $(HOME)/repos/toolchains/aeneas/$(AENEAS_REV)
# An AENEAS_DIR exported into the shell silently wins over the default above,
# which is the one way this can point somewhere unrelated to the recorded rev.
# Say so in the failure rather than leaving it to be noticed.
AENEAS_DIR_ORIGIN := $(origin AENEAS_DIR)

CHARON := $(AENEAS_DIR)/charon/bin/charon
AENEAS := $(AENEAS_DIR)/bin/aeneas
LLBC := verified/target/llbc/leaners_render.llbc

# What `make extract` leaves out of the model: the unverified frontend, which
# pulls in pulldown-cmark, and the String glue around it. One variable, shared
# with `extract-report`, so the extraction and the coverage summary cannot
# disagree about what was excluded on purpose.
EXTRACT_EXCLUDES := crate::adapt::_ crate::markdown_to_html

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

# Panic locations embed absolute source paths, so without these remaps the
# binary ships the local username and checkout location, and the same sources
# hash differently on every machine. Mapping the three roots that can appear
# (the repo, the cargo registry, the toolchain's own sources) to fixed names is
# what lets verify.sh and CI compare hashes at all.
WASM_RUSTFLAGS = --remap-path-prefix=$(CURDIR)=/leaners \
	--remap-path-prefix=$(or $(CARGO_HOME),$(HOME)/.cargo)=/cargo \
	--remap-path-prefix=$(or $(RUSTUP_HOME),$(HOME)/.rustup)/toolchains/$(shell rustup show active-toolchain 2>/dev/null | cut -d' ' -f1)=/toolchain

.PHONY: wasm
wasm: ## Build the Rust renderer and copy it into pkg/
	@RUSTFLAGS="$(WASM_RUSTFLAGS)" cargo build --release \
		--manifest-path verified/wasm/Cargo.toml --target wasm32-unknown-unknown
	@mkdir -p pkg
	@cp verified/wasm/target/wasm32-unknown-unknown/release/leaners_wasm.wasm pkg/render.wasm
	@ls -l pkg/render.wasm

.PHONY: lint
lint: ## Clippy and rustfmt over both crates, warnings are errors
	@cargo fmt --manifest-path verified/Cargo.toml --check
	@cargo fmt --manifest-path verified/wasm/Cargo.toml --check
	@cargo clippy --manifest-path verified/Cargo.toml --all-targets -- -D warnings
	@cargo clippy --manifest-path verified/wasm/Cargo.toml --all-targets -- -D warnings
	@echo "clippy and rustfmt clean"

.PHONY: extract
extract: ## Re-extract the Lean model from the Rust via charon + aeneas
	@test -n "$(AENEAS_REV)" || { \
		echo "no toolchains.aeneas_rev in build-manifest.json"; exit 1; }
	@test -x "$(CHARON)" || { \
		echo "charon not found at $(CHARON)"; \
		[ "$(AENEAS_DIR_ORIGIN)" = environment ] && \
			echo "AENEAS_DIR comes from your environment and overrides the rev-keyed default; unset it"; \
		echo "build it with: cd $(AENEAS_DIR) && make setup-charon"; exit 1; }
	@test -x "$(AENEAS)" || { \
		echo "aeneas $(AENEAS_REV) not found at $(AENEAS)"; \
		[ "$(AENEAS_DIR_ORIGIN)" = environment ] && \
			echo "AENEAS_DIR comes from your environment and overrides the rev-keyed default; unset it"; \
		echo "build it with: cd $(AENEAS_DIR) && make"; exit 1; }
	@# The path already names the rev, but AENEAS_DIR can be overridden and a
	@# worktree can be moved off its commit, so check the checkout itself.
	@# A non-git AENEAS_DIR (a nix store path, say) is left alone: the hash of
	@# the extracted model in build-manifest.json is the backstop either way.
	@head=$$(git -C "$(AENEAS_DIR)" rev-parse HEAD 2>/dev/null); \
	if [ -n "$$head" ] && [ "$$head" != "$(AENEAS_REV)" ]; then \
		echo "$(AENEAS_DIR) is at $$head"; \
		echo "build-manifest.json wants $(AENEAS_REV)"; exit 1; \
	fi
	@mkdir -p verified/target/llbc proofs/Extracted
	@# `-- --lib` is load-bearing: without it charon also walks the bin targets and
	@# the last one wins, so you get a model of tests/vectors.rs instead of the
	@# library. The excludes are the unverified frontend and its String glue; see
	@# EXTRACT_EXCLUDES above.
	@#
	@# ast.rs and render.rs used to be excluded too: Inline::Emph(Vec<Inline>)
	@# recursed through Vec, and Lean's kernel rejects the resulting nested
	@# inductive ("non valid occurrence of the datatypes being declared"). The
	@# Ast is a flat event stream now, nothing in it recurses, and both modules
	@# extract.
	@cd verified && "$(CHARON)" cargo --preset=aeneas \
		$(foreach e,$(EXTRACT_EXCLUDES),--exclude '$(e)') \
		--dest-file target/llbc/leaners_render.llbc -- --lib
	@# -loops-to-rec extracts loops as recursive functions rather than through
	@# the `loop` combinator: that is the shape every documented Aeneas proof
	@# works against (unfold + step + termination_by), and the refinement
	@# proofs in proofs/Leaners/Refine/ are written in exactly that idiom.
	@"$(AENEAS)" -backend lean -loops-to-rec "$(LLBC)" -dest proofs/Extracted
	@echo "extracted into proofs/Extracted. Review the diff: that is how you"
	@echo "notice a Rust change that altered the model's meaning."

.PHONY: extract-report
extract-report: ## Which lines of verified/src the extracted model covers
	@$(PY) extract-report $(foreach e,$(EXTRACT_EXCLUDES),--exclude $(e))

.PHONY: proofs
proofs: ## Build the Lean model and its proofs
	@# Mathlib is pinned to a released rev, so upstream CI has already published
	@# its oleans: fetch them instead of spending hours recompiling 8000 modules.
	@# Failure is tolerated so an offline machine still works, just slowly.
	@cd proofs && lake exe cache get || echo "no mathlib cache available, building from source"
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

# cli.py resolves the extraction toolchain the same way this file does, but it
# reads AENEAS_DIR from its environment, so the two agree only if the value is
# passed through. Without this the recorded aeneas_rev and charon version come
# out null, quietly dropping the pins the extracted model depends on.
manifest manifest-check: export AENEAS_DIR := $(AENEAS_DIR)

.PHONY: manifest
manifest: ## Record hashes binding pkg/render.wasm to the sources it came from
	@$(PY) manifest

.PHONY: manifest-check
manifest-check: ## Verify pkg/ still matches the sources recorded in build-manifest.json
	@$(PY) manifest --check $(MANIFEST_CHECK_FLAGS)

.PHONY: verify
verify: ## Full integrity check: rebuild, re-extract, compare hashes, run proofs
	@# VERIFY_ARGS=--local-wasm on a machine that did not record the manifest.
	@./verify.sh $(VERIFY_ARGS)

.PHONY: publish
publish: ## Publish to GitHub Pages
	@$(PY) publish -m "$(or $(M),Update content)"
