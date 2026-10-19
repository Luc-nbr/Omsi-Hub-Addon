#!/usr/bin/env python3
"""The licences of the Rust libraries Omsi-Hub is built with, written to
ui/third-party-licenses.txt (the app carries that file, so every copy of Omsi-Hub hands their
notices on): each library with its version, licence and address, then the licence files they came
with, every text once with the libraries that share it.

Run after a change to Cargo.lock:  python3 tools/licenses.py
Only the libraries the app is built of count (no test-only ones); Omsi-Hub's own crates are left
out. The fork's openOMSI crates are listed too (THIRD_PARTY_NOTICES.md has their licence).
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "ui" / "third-party-licenses.txt"
NAMES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE", "COPYRIGHT")


def main() -> int:
    meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--locked"], cwd=ROOT, check=True, capture_output=True, text=True).stdout)
    own = set(meta["workspace_members"])
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    # (what the app is built of: the normal and build dependencies, not the tests')
    seen, todo = set(), list(own)
    while todo:
        i = todo.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] in (None, "build") for k in d["dep_kinds"]):
                todo.append(d["pkg"])
    pkgs = sorted((p for p in meta["packages"] if p["id"] in seen - own), key=lambda p: (p["name"], p["version"]))

    # (a text once: the same text with other line breaks or spaces is the same; the first kept)
    texts: dict[str, list[str]] = {}
    shown: dict[str, str] = {}
    lines = []
    for p in pkgs:
        who = f'{p["name"]} {p["version"]}'
        lines.append(f'{who}: {p.get("license") or "see its licence file"}' + (f'  <{p["repository"]}>' if p.get("repository") else ""))
        folder = pathlib.Path(p["manifest_path"]).parent
        found = sorted(f for f in folder.iterdir() if f.is_file() and f.name.upper().startswith(NAMES))
        if p.get("license_file"):
            found.append(folder / p["license_file"])
        for f in dict.fromkeys(found):
            try:
                t = f.read_text(encoding="utf-8", errors="replace").strip()
            except OSError:
                continue
            if t:
                key = " ".join(t.split())
                shown.setdefault(key, t)
                texts.setdefault(key, []).append(who)

    out = [
        "Omsi-Hub: the licences of the libraries it is built with",
        "",
        "Omsi-Hub is proprietary software (its LICENSE). It contains the libraries below, each under",
        "its own licence; Omsi-Hub's licence takes away none of the rights they give. The fonts and",
        "openOMSI's crates: THIRD_PARTY_NOTICES.md. Libraries under the MPL-2.0 are used unchanged; their",
        "source is at the address given.",
        "",
        "The libraries",
        "=============",
        "",
        *lines,
        "",
        "Their licence files",
        "===================",
    ]
    for key, who in sorted(texts.items(), key=lambda kv: kv[1][0]):
        out += ["", "-" * 78, "For: " + ", ".join(dict.fromkeys(who)), "-" * 78, "", shown[key]]
    OUT.write_text("\n".join(out) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(pkgs)} libraries, {len(texts)} licence texts")
    return 0


if __name__ == "__main__":
    sys.exit(main())
