from __future__ import annotations

import argparse
import functools
import os
import http.server
import json
import re
import socketserver
import subprocess
import sys
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
CONTENT = ROOT / "content"
MANIFEST = CONTENT / "index.json"


def docs_on_disk() -> list[Path]:
    # Sort on the relative POSIX path rather than on Path objects. Path ordering
    # compares parts, so notes/lean/tactics/simp.md sorts before notes/lean.md
    # and a section's children end up listed above the page that heads them.
    return sorted(CONTENT.rglob("*.md"), key=lambda p: p.relative_to(CONTENT).as_posix())


def title_of(path: Path) -> str:
    """First ATX heading, else the filename prettified."""
    for line in path.read_text(encoding="utf-8").splitlines():
        if m := re.match(r"^#\s+(.+?)\s*$", line):
            return m.group(1)
    return path.stem.replace("-", " ").replace("_", " ").capitalize()


def normalise(entries: list) -> list[dict]:
    """Accept a bare "notes/hello.md" string as well as a full entry object.

    Adding a page through GitHub's web editor means hand-editing this manifest,
    where a one-line string has no key order to get wrong and no title to
    mistype. `index` rewrites either spelling into the canonical form.
    """
    docs = []
    for entry in entries:
        if isinstance(entry, str):
            docs.append({"path": entry})
        elif isinstance(entry, dict) and entry.get("path"):
            docs.append(entry)
    return docs


def read_manifest() -> list[dict]:
    if not MANIFEST.exists():
        return []
    try:
        data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        # Regenerating the manifest is how a malformed one gets repaired, so
        # this must survive the very file it is about to rewrite rather than
        # dying on it.
        print(f"warning: {MANIFEST.relative_to(ROOT)} is not valid JSON ({exc})", file=sys.stderr)
        return []
    return normalise(data.get("docs", []))


def cmd_index(args) -> int:
    """Regenerate content/index.json from the tree, preserving order and titles."""
    existing = {d["path"]: d for d in read_manifest()}
    on_disk = {p.relative_to(CONTENT).as_posix(): p for p in docs_on_disk()}

    # Manifest order is the sidebar's order, and it is meant to be rearranged by
    # hand: move the lines and `index` leaves them moved. Files not listed yet
    # are appended in path order, so adding a page never silently reshuffles the
    # ones already there. `--sort` throws the hand-ordering away again.
    if args.sort:
        order = list(on_disk)
    else:
        listed = [rel for rel in existing if rel in on_disk]
        order = listed + [rel for rel in on_disk if rel not in existing]

    # A hand-edited title in the manifest wins over the file's heading.
    docs = [
        {"path": rel, "title": existing.get(rel, {}).get("title") or title_of(on_disk[rel])}
        for rel in order
    ]

    payload = json.dumps({"docs": docs}, indent=2, ensure_ascii=False) + "\n"
    if args.check:
        # Compare meaning, not bytes. Key order, indentation and entry order are
        # cosmetic and `index` fixes them, so they must not fail a check that
        # gates anything. A path appearing or vanishing is the real drift.
        listed = {d["path"]: d.get("title") for d in read_manifest()}
        rebuilt = {d["path"]: d["title"] for d in docs}
        if listed != rebuilt:
            for path in sorted(set(rebuilt) - set(listed)):
                print(f"  missing from index.json: {path}", file=sys.stderr)
            for path in sorted(set(listed) - set(rebuilt)):
                print(f"  listed but not on disk: {path}", file=sys.stderr)
            for path in sorted(set(listed) & set(rebuilt)):
                if listed[path] != rebuilt[path]:
                    print(f"  {path}: title not canonical yet", file=sys.stderr)
            print("content/index.json is out of date. Run: make index", file=sys.stderr)
            return 1
        print(f"content/index.json is up to date ({len(docs)} docs)")
        return 0

    MANIFEST.write_text(payload, encoding="utf-8")
    print(f"wrote {MANIFEST.relative_to(ROOT)} ({len(docs)} docs)")
    return 0


def strip_code(text: str) -> str:
    """Blanks out fenced blocks and inline spans so documentation examples are
    not mistaken for real links. Line count is preserved so any future
    line-numbered diagnostics stay accurate."""
    out, fenced = [], False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
            out.append("")
            continue
        out.append("" if fenced else re.sub(r"`[^`]*`", "", line))
    return "\n".join(out)


def cmd_check(args) -> int:
    """Validate the manifest against the tree and resolve internal links."""
    problems: list[str] = []

    listed = {d["path"] for d in read_manifest()}
    on_disk = {p.relative_to(CONTENT).as_posix() for p in docs_on_disk()}

    for missing in sorted(listed - on_disk):
        problems.append(f"index.json lists {missing}, which does not exist")
    for unlisted in sorted(on_disk - listed):
        problems.append(f"{unlisted} exists but is not in index.json")

    for p in docs_on_disk():
        # Links inside code fences and inline code spans are examples, not links.
        text = strip_code(p.read_text(encoding="utf-8"))
        here = PurePosixPath(p.relative_to(CONTENT).as_posix()).parent

        # In-site links look like #/notes/hello
        for m in re.finditer(r"\]\(#/([^)\s]+)\)", text):
            target = m.group(1)
            target = target if target.endswith(".md") else f"{target}.md"
            if target not in on_disk:
                problems.append(f"{p.relative_to(CONTENT)}: dead link to #/{m.group(1)}")

        # Document-relative links such as ./sibling.md or ../other/page.md. These
        # are the form that also works when the file is read on GitHub, and
        # app.js rewrites them into routes. Resolve them the same way it does.
        for m in re.finditer(r"\]\((?!#|/|[a-z][a-z0-9+.\-]*:)([^)\s]+\.md)\)", text, re.I):
            rel = m.group(1)
            target = os.path.normpath((here / rel).as_posix()).replace("\\", "/")
            if target not in on_disk:
                problems.append(f"{p.relative_to(CONTENT)}: dead link to {rel}")

    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        print(f"\n{len(problems)} problem(s)", file=sys.stderr)
        return 1
    print(f"ok: {len(on_disk)} docs, manifest consistent, no dead internal links")
    return 0


MANIFEST_FILES = [
    "pkg/render.wasm",
    "verified/Cargo.toml",
    "verified/wasm/Cargo.toml",
]
MANIFEST_GLOBS = ["verified/src/**/*.rs", "verified/wasm/src/*.rs", "proofs/Extracted/*.lean"]

# Compiled output, as opposed to sources. Its hash pins the artifact to the
# sources only for a rebuild on the machine that recorded it: rustc does not
# promise byte-identical wasm across hosts, and a toolchain carrying different
# components (rust-src, say) embeds different paths. `--sources-only` drops it
# so CI can check the sources exactly and bind the binary a different way.
MANIFEST_ARTIFACTS = {"pkg/render.wasm"}


def sha256(path: Path) -> str:
    import hashlib

    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def tool_version(cmd: list[str]) -> str | None:
    """Version string, or None when the tool is not installed here."""
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, check=False, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return None
    return r.stdout.strip().splitlines()[0] if r.returncode == 0 and r.stdout.strip() else None


def build_manifest() -> dict:
    """Hashes binding pkg/ to the sources it was built from, plus the toolchain
    versions that did it. See design.md section 9: without this, nothing
    mechanically ties the shipped .wasm to the Rust the proofs are about."""
    files: dict[str, str] = {}
    for rel in MANIFEST_FILES:
        p = ROOT / rel
        if p.exists():
            files[rel] = sha256(p)
    for pattern in MANIFEST_GLOBS:
        for p in sorted(ROOT.glob(pattern)):
            files[p.relative_to(ROOT).as_posix()] = sha256(p)

    aeneas_dir = Path(os.environ.get("AENEAS_DIR", "/opt/repos/toolchains/aeneas"))
    aeneas_rev = None
    if (aeneas_dir / ".git").exists():
        r = subprocess.run(
            ["git", "-C", str(aeneas_dir), "rev-parse", "HEAD"],
            capture_output=True, text=True, check=False,
        )
        aeneas_rev = r.stdout.strip() or None

    toolchain = ROOT / "proofs" / "lean-toolchain"
    return {
        "files": files,
        "toolchains": {
            "rustc": tool_version(["rustc", "--version"]),
            "charon": tool_version([str(aeneas_dir / "charon" / "bin" / "charon"), "version"]),
            "aeneas_rev": aeneas_rev,
            "lean": toolchain.read_text(encoding="utf-8").strip() if toolchain.exists() else None,
        },
    }


def cmd_manifest(args) -> int:
    path = ROOT / "build-manifest.json"
    current = build_manifest()
    payload = json.dumps(current, indent=2, sort_keys=True) + "\n"

    if args.check:
        if not path.exists():
            print("build-manifest.json is missing. Run: make manifest", file=sys.stderr)
            return 1
        recorded = json.loads(path.read_text(encoding="utf-8"))
        skip = MANIFEST_ARTIFACTS if args.sources_only else set()
        problems = []
        for rel, digest in recorded.get("files", {}).items():
            if rel in skip:
                continue
            actual = current["files"].get(rel)
            if actual is None:
                problems.append(f"{rel}: recorded but missing from the tree")
            elif actual != digest:
                problems.append(
                    f"{rel}: hash differs from the manifest "
                    f"(recorded {digest[:12]}, found {actual[:12]})"
                )
        for rel in current["files"]:
            if rel not in recorded.get("files", {}) and rel not in skip:
                problems.append(f"{rel}: present but not recorded")
        # A toolchain the manifest names but this machine lacks is reported, not
        # failed: you can review the repo without charon installed.
        for name, want in recorded.get("toolchains", {}).items():
            got = current["toolchains"].get(name)
            if got is not None and want is not None and got != want:
                problems.append(f"toolchain {name}: manifest says {want!r}, found {got!r}")
        for problem in problems:
            print(problem, file=sys.stderr)
        if problems:
            print(f"\n{len(problems)} problem(s). The shipped artifact and the sources "
                  "the proofs are about may have drifted.", file=sys.stderr)
            return 1
        checked = len(recorded.get("files", {})) - len(skip & set(recorded.get("files", {})))
        what = "source files" if args.sources_only else "files"
        print(f"build-manifest.json matches ({checked} {what})")
        return 0

    path.write_text(payload, encoding="utf-8")
    print(f"wrote build-manifest.json ({len(current['files'])} files)")
    return 0


def line_ranges(numbers: list[int]) -> list[str]:
    """Compresses sorted line numbers into "12" / "14-19" range strings."""
    out: list[str] = []
    for n in numbers:
        prev = out[-1] if out else None
        if prev and int(prev.split("-")[-1]) == n - 1:
            out[-1] = f"{prev.split('-')[0]}-{n}"
        else:
            out.append(str(n))
    return out


def item_span(lines: list[str], name: str) -> set[int]:
    """1-based lines of `fn name` in a file, signature through closing brace.
    Empty when the file does not define it."""
    sig = re.compile(rf"^\s*(pub\s+)?fn\s+{re.escape(name)}\b")
    for start, line in enumerate(lines):
        if not sig.match(line):
            continue
        depth, opened = 0, False
        for end in range(start, len(lines)):
            depth += lines[end].count("{") - lines[end].count("}")
            opened = opened or "{" in lines[end]
            if opened and depth == 0:
                return set(range(start + 1, end + 2))
    return set()


def cmd_extract_report(args) -> int:
    """Line-by-line account of what `make extract` covered. Aeneas stamps every
    definition it emits with the Rust source span it came from; mapping those
    spans back onto verified/src says exactly which lines are in the model,
    which were excluded on purpose, and which fell through."""
    crate = ROOT / "verified"

    span_re = re.compile(r"Source: '([^']+)', lines (\d+):\d+-(\d+):\d+")
    covered: dict[str, set[int]] = {}
    for lean in sorted((ROOT / "proofs" / "Extracted").glob("*.lean")):
        for m in span_re.finditer(lean.read_text(encoding="utf-8")):
            lines = covered.setdefault(m.group(1), set())
            lines.update(range(int(m.group(2)), int(m.group(3)) + 1))
    if not covered:
        print("no extracted model under proofs/Extracted, nothing to report", file=sys.stderr)
        return 1

    # The same --exclude patterns `make extract` hands to charon, so the report
    # cannot call something "missing" that the Makefile excludes on purpose.
    module_excludes: list[tuple[str, str]] = []
    item_excludes: list[tuple[str, str]] = []
    for pattern in args.exclude:
        parts = pattern.split("::")
        if parts[0] != "crate" or len(parts) < 2:
            print(f"unsupported exclude pattern: {pattern}", file=sys.stderr)
            return 2
        if parts[-1] == "_":
            module_excludes.append(("/".join(parts[1:-1]), pattern))
        else:
            item_excludes.append((parts[-1], pattern))

    rows: list[tuple[str, int, int, int, int, str]] = []
    missing_where: list[str] = []
    for path in sorted((crate / "src").rglob("*.rs")):
        rel = path.relative_to(crate).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()

        note = ""
        skipped_lines: set[int] = set()
        if rel.startswith("src/bin/"):
            skipped_lines = set(range(1, len(lines) + 1))
            note = "bin target, extraction runs with --lib"
        for mod_path, pattern in module_excludes:
            if rel == f"src/{mod_path}.rs" or rel.startswith(f"src/{mod_path}/"):
                skipped_lines = set(range(1, len(lines) + 1))
                note = f"excluded: {pattern}"
        if not skipped_lines:
            for name, pattern in item_excludes:
                span = item_span(lines, name)
                if span:
                    skipped_lines |= span
                    note = f"excluded: {pattern}"

        cov = covered.get(rel, set())
        code = extracted = skipped = 0
        missing: list[int] = []
        for n, raw in enumerate(lines, start=1):
            s = raw.strip()
            # Blanks, comments and use/mod declarations carry no semantics the
            # model could cover, so they belong in no bucket.
            if not s or s.startswith("//") or s.startswith("#["):
                continue
            if re.match(r"(pub(\(\w+\))?\s+)?(use|mod)\s", s):
                continue
            code += 1
            if n in cov:
                extracted += 1
            elif n in skipped_lines:
                skipped += 1
            else:
                missing.append(n)
        rows.append((rel, code, extracted, skipped, len(missing), note))
        missing_where += [f"{rel}:{r}" for r in line_ranges(missing)]

    width = max(len(r[0]) for r in rows)
    print(f"{'file':<{width}}  code  extracted  skipped  missing")
    for rel, code, ex, sk, mi, note in rows:
        print(f"{rel:<{width}}  {code:>4}  {ex:>9}  {sk:>7}  {mi:>7}  {note}".rstrip())
    total = [sum(r[i] for r in rows) for i in (1, 2, 3, 4)]
    print(f"{'total':<{width}}  {total[0]:>4}  {total[1]:>9}  {total[2]:>7}  {total[3]:>7}")
    if missing_where:
        print("not extracted: " + ", ".join(missing_where))
    print(
        f"coverage: {total[1]} of {total[0]} code lines extracted, "
        f"{total[2]} skipped as unverified by design, {total[3]} not extracted"
    )
    return 0


def cmd_serve(args) -> int:
    """Preview locally. Needed because ES modules will not load over file://."""
    class Handler(http.server.SimpleHTTPRequestHandler):
        # SimpleHTTPRequestHandler sends no Cache-Control, so a browser applies
        # heuristic freshness and can keep serving a stale render.wasm or app.js
        # for hours after a rebuild, which looks exactly like a broken change.
        # A preview server should never cache anything.
        def end_headers(self):
            self.send_header("Cache-Control", "no-store, must-revalidate")
            super().end_headers()

    handler = functools.partial(Handler, directory=str(ROOT))

    class Server(socketserver.TCPServer):
        allow_reuse_address = True

    with Server(("127.0.0.1", args.port), handler) as httpd:
        print(f"serving {ROOT} at http://127.0.0.1:{args.port}/  (ctrl-c to stop)")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print()
    return 0


def git(*args, capture=True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=ROOT, text=True,
        capture_output=capture, check=False,
    )


def cmd_publish(args) -> int:
    """Validate, commit and push. GitHub Pages serves main directly, so this
    is the whole deploy: there is nothing to build."""
    if git("rev-parse", "--git-dir").returncode != 0:
        print("not a git repository", file=sys.stderr)
        return 1
    if not git("remote", "get-url", "origin").stdout.strip():
        print("no 'origin' remote. Run: make setup REPO=owner/name", file=sys.stderr)
        return 1

    # Regenerate rather than refuse: the manifest is derived from the tree, and
    # `add` below picks it up so it travels in the same commit as the page.
    if cmd_index(argparse.Namespace(check=False, sort=False)) != 0:
        return 1
    if cmd_check(args) != 0:
        return 1

    # Stage tracked changes plus anything new under content/. Deliberately not
    # `add -A`, so stray files in the repo root are never swept into a commit.
    git("add", "-u")
    git("add", str(CONTENT.relative_to(ROOT)))

    staged = git("diff", "--cached", "--name-only").stdout.split()
    if staged:
        print("committing:")
        for f in staged:
            print(f"  {f}")
        if git("commit", "-m", args.message).returncode != 0:
            print("commit failed", file=sys.stderr)
            return 1

    # Committing and publishing are different questions: there may be nothing
    # to commit but still unpushed commits from earlier.
    branch = git("rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    git("fetch", "--quiet", "origin", branch)
    counts = git("rev-list", "--left-right", "--count", f"origin/{branch}...HEAD")
    behind, ahead = (0, 1) if counts.returncode != 0 else map(int, counts.stdout.split())

    if behind and not ahead:
        print(f"behind origin/{branch} by {behind}. Run: git pull", file=sys.stderr)
        return 1
    if behind and ahead:
        print(f"diverged from origin/{branch} ({behind} behind, {ahead} ahead).", file=sys.stderr)
        return 1
    if not ahead:
        print("nothing to publish; already up to date")
        return 0

    print(f"pushing {ahead} commit(s) to origin/{branch}")
    if git("push", "origin", branch, capture=False).returncode != 0:
        print("push failed", file=sys.stderr)
        return 1

    print("\npushed. GitHub Pages usually reflects it within a minute.")
    return 0


def main() -> int:
    # Keep our output interleaved correctly with git's, which writes straight
    # to the terminal while Python's stdout is block-buffered under `make`.
    sys.stdout.reconfigure(line_buffering=True)

    parser = argparse.ArgumentParser(prog="leaners", description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_index = sub.add_parser("index", help="regenerate content/index.json")
    p_index.add_argument("--check", action="store_true", help="verify instead of writing")
    p_index.add_argument("--sort", action="store_true", help="discard hand-ordering, sort by path")
    p_index.set_defaults(func=cmd_index)

    p_man = sub.add_parser("manifest", help="record hashes binding pkg/ to its sources")
    p_man.add_argument("--check", action="store_true", help="verify instead of writing")
    p_man.add_argument("--sources-only", action="store_true",
                       help="skip compiled artifacts, whose bytes are host dependent")
    p_man.set_defaults(func=cmd_manifest)

    p_check = sub.add_parser("check", help="validate manifest and internal links")
    p_check.set_defaults(func=cmd_check)

    p_report = sub.add_parser(
        "extract-report", help="which lines of verified/src the extracted model covers"
    )
    p_report.add_argument(
        "--exclude", action="append", default=[], metavar="PATTERN",
        help="charon exclude pattern the extraction ran with, repeatable",
    )
    p_report.set_defaults(func=cmd_extract_report)

    p_publish = sub.add_parser("publish", help="validate, commit and push to GitHub Pages")
    p_publish.add_argument("-m", "--message", default="Update content", help="commit message")
    p_publish.set_defaults(func=cmd_publish)

    p_serve = sub.add_parser("serve", help="preview at http://127.0.0.1:8000")
    p_serve.add_argument("--port", type=int, default=8000)
    p_serve.set_defaults(func=cmd_serve)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
