# Adding a new page

Notes can sit in subfolders. `content/notes/lean/tactics/simp.md` is reached at
`#/notes/lean/tactics/simp`, and the sidebar nests it to match.

## On GitHub, in the browser

1. **New page**, and type a filename. Slashes create folders, so
   `notes/lean/tactics/simp.md` works even if those folders do not exist yet.
2. Add the path to `content/index.json`, as a single string:

```json
{
  "docs": [
    "notes/lean/tactics/simp.md"
  ]
}
```

That is enough. CI rewrites it into the full form and fills the title in from
the page's first heading. Until it does, the sidebar labels the page after its
filename.

## On your machine

Create the file and run `make publish`. The manifest is regenerated as part of
publishing, so there is nothing to remember and nothing to hand-edit.


## Linking between pages

Both forms work:

```markdown
[experiences](./experiences.md)            relative, also works on GitHub
[experiences](#/notes/lean/experiences)    a route, only works here
```

Prefer the relative form. It reads correctly in GitHub's file view and in any
editor, and the site rewrites it into a route when the page is rendered.
Relative links resolve against the linking document's own folder, so
`./sibling.md` and `../other/page.md` mean what you expect.

`make check` validates both forms and fails on a dead one.

## A folder can have a page of its own

Three spellings work, so moving a page under a folder never forces a rename:

| File | Heads |
|---|---|
| `notes/rust.md` | `notes/rust/` |
| `notes/lean/lean.md` | `notes/lean/` |
| `notes/lean/tactics/index.md` | `notes/lean/tactics/` |

A folder without one appears in the sidebar as plain text rather than a link.
