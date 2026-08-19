from __future__ import annotations

import argparse
import functools
import http.server
import json
import re
import socketserver
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONTENT = ROOT / "content"
MANIFEST = CONTENT / "index.json"


def docs_on_disk() -> list[Path]:
    return sorted(p for p in CONTENT.rglob("*.md"))


def title_of(path: Path) -> str:
    """First ATX heading, else the filename prettified."""
    for line in path.read_text(encoding="utf-8").splitlines():
        if m := re.match(r"^#\s+(.+?)\s*$", line):
            return m.group(1)
    return path.stem.replace("-", " ").replace("_", " ").capitalize()


def read_manifest() -> list[dict]:
    if not MANIFEST.exists():
        return []
    return json.loads(MANIFEST.read_text(encoding="utf-8")).get("docs", [])


def cmd_index(args) -> int:
    """Regenerate content/index.json from the tree, preserving manual titles."""
    existing = {d["path"]: d for d in read_manifest()}
    docs = []
    for p in docs_on_disk():
        rel = p.relative_to(CONTENT).as_posix()
        # A hand-edited title in the manifest wins over the file's heading.
        docs.append({"path": rel, "title": existing.get(rel, {}).get("title") or title_of(p)})

    payload = json.dumps({"docs": docs}, indent=2, ensure_ascii=False) + "\n"
    if args.check:
        current = MANIFEST.read_text(encoding="utf-8") if MANIFEST.exists() else ""
        if current != payload:
            print("content/index.json is out of date. Run: make index", file=sys.stderr)
            return 1
        print(f"content/index.json is up to date ({len(docs)} docs)")
        return 0

    MANIFEST.write_text(payload, encoding="utf-8")
    print(f"wrote {MANIFEST.relative_to(ROOT)} ({len(docs)} docs)")
    return 0


def cmd_check(args) -> int:
    """Validate the manifest against the tree and resolve internal links."""
    problems: list[str] = []

    listed = {d["path"] for d in read_manifest()}
    on_disk = {p.relative_to(CONTENT).as_posix() for p in docs_on_disk()}

    for missing in sorted(listed - on_disk):
        problems.append(f"index.json lists {missing}, which does not exist")
    for unlisted in sorted(on_disk - listed):
        problems.append(f"{unlisted} exists but is not in index.json")

    # In-site links look like #/notes/hello
    for p in docs_on_disk():
        for m in re.finditer(r"\]\(#/([^)\s]+)\)", p.read_text(encoding="utf-8")):
            target = m.group(1)
            target = target if target.endswith(".md") else f"{target}.md"
            if target not in on_disk:
                problems.append(f"{p.relative_to(CONTENT)}: dead link to #/{m.group(1)}")

    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        print(f"\n{len(problems)} problem(s)", file=sys.stderr)
        return 1
    print(f"ok: {len(on_disk)} docs, manifest consistent, no dead internal links")
    return 0


def cmd_serve(args) -> int:
    """Preview locally. Needed because ES modules will not load over file://."""
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(ROOT))

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

    if cmd_index(argparse.Namespace(check=True)) != 0:
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
    p_index.set_defaults(func=cmd_index)

    p_check = sub.add_parser("check", help="validate manifest and internal links")
    p_check.set_defaults(func=cmd_check)

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
