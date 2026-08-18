# Adding a page

Entirely from the GitHub web interface, with no tooling installed.

## 1. Create the file

Go to `content/notes/` in the repository and choose **Add file**, then
**Create new file**. Name it something like `my-note.md` and write Markdown.

## 2. List it

Edit `content/index.json` and add an entry:

```json
{ "path": "notes/my-note.md", "title": "My note" }
```

That is the only bookkeeping. The nav and the home page both read this file.

## Why a manifest at all?

A static host cannot list a directory, so the site has no way to discover which
documents exist. The alternatives are worse: the GitHub API rate-limits
anonymous readers to 60 requests an hour, and a build step would reintroduce
exactly the staleness this design avoids.

If you have the repository checked out, `make index` regenerates the manifest
from the tree so you never edit it by hand.
