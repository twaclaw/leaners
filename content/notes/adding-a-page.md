# Adding a page

Entirely from the GitHub web interface, with no tooling installed. **New page**
in the top right opens GitHub's file creator. Two steps:

1. Create `content/notes/my-note.md` and write Markdown.
2. Edit `content/index.json` and add an entry:

```json
{ "path": "notes/my-note.md", "title": "My note" }
```

The nav and the home page both read that file.

## Why a manifest at all?

A static host cannot list a directory, so the site has no way to discover which
documents exist. The alternatives are worse: the GitHub API rate-limits
anonymous readers to 60 requests an hour, and a build step would reintroduce
exactly the staleness this design avoids.

If you have the repository checked out, `make index` regenerates the manifest
from the tree so you never edit it by hand.
