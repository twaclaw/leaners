# Local helpers. None of this is required to deploy: GitHub Pages serves this
# repository directly, so `git push` is the deploy. See design.md section 9.

PY := uv run leaners
PORT ?= 8000

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

.PHONY: check
check: ## Validate the manifest and internal links
	@$(PY) index --check
	@$(PY) check

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
