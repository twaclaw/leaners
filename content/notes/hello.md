# Hello

This page is a Markdown file at `content/notes/hello.md`. It is fetched and
rendered in your browser. There is no build step and no CI: the file in the
repository *is* the page.

## Try the loop

1. Click **Edit this page** in the top right.
2. Change a word and commit on `main`.
3. Come back here and reload.

The change is live. That is the whole point of the design: because Markdown is
data rather than build input, committing is publishing.

## What comes next

The renderer is currently `marked`, loaded from `vendor/`. It will be replaced
by a Rust renderer compiled to WebAssembly, whose `Ast -> HTML` half is verified
in Lean via Aeneas. Nothing about this page has to change when that happens.

See `design.md` in the repository root for the full plan.
