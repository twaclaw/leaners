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

## What renders this

A Rust renderer compiled to WebAssembly, whose escaping, URL and slug functions
are extracted to Lean by Aeneas and proved there. It replaced the JavaScript
renderer this page originally described, and nothing about the page had to
change: the seam was async from the start.


