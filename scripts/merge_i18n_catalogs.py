#!/usr/bin/env python3
"""Re-run phase1 generator, then merge Phase 2–4 keys into catalogs."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def flat_section(d: dict[str, str]) -> str:
    tables: dict[tuple[str, ...], dict[str, str]] = {}
    for k, v in d.items():
        *path, last = k.split(".")
        tables.setdefault(tuple(path), {})[last] = v
    out: list[str] = []
    for path, fields in sorted(tables.items(), key=lambda x: x[0]):
        out.append(f"[{'.'.join(path)}]")
        for fk, fv in fields.items():
            out.append(f'{fk} = "{esc(fv)}"')
        out.append("")
    return "\n".join(out)


def flatten(node: object, prefix: str = "") -> dict[str, str]:
    out: dict[str, str] = {}
    if isinstance(node, dict):
        for k, v in node.items():
            p = f"{prefix}.{k}" if prefix else str(k)
            if isinstance(v, dict):
                out.update(flatten(v, p))
            else:
                out[p] = str(v)
    return out


def load_remaining() -> tuple[dict[str, str], dict[str, str]]:
    path = ROOT / "scripts/gen_i18n_remaining.py"
    src = path.read_text(encoding="utf-8")
    # EN/ZH live after helper defs; stop before append_missing/main.
    code = src.split("def append_missing")[0]
    ns: dict = {
        "__name__": "gen_rem",
        "__file__": str(path),
    }
    exec(code, ns)
    return ns["EN"], ns["ZH"]


def merge_into(path: Path, extra: dict[str, str]) -> None:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    all_keys = flatten(data)
    before = len(all_keys)
    all_keys.update(extra)
    path.write_text(
        "# Locale catalog (merged phase1 + remaining)\n" + flat_section(all_keys),
        encoding="utf-8",
    )
    tomllib.loads(path.read_text(encoding="utf-8"))
    print(f"{path.name}: {len(all_keys)} keys (+{len(all_keys) - before})")


def main() -> None:
    subprocess.check_call([sys.executable, str(ROOT / "scripts/gen_i18n_phase1.py")])
    en_extra, zh_extra = load_remaining()
    merge_into(ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml", en_extra)
    merge_into(ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml", zh_extra)
    print("merge ok")


if __name__ == "__main__":
    main()
