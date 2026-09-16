#!/usr/bin/env python3
"""Run the catalog generators without rewriting unrelated locale entries."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from i18n_catalog import load_catalog, merge_catalog_file


def load_remaining() -> tuple[dict[str, str], dict[str, str]]:
    # Import the data module normally so its main() is not executed and no
    # source-code slicing or exec() is needed.
    from gen_i18n_remaining import EN, ZH

    return dict(EN), dict(ZH)


def merge_into(path: Path, extra: dict[str, str]) -> None:
    before = len(load_catalog(path))
    result = merge_catalog_file(
        path,
        extra,
        banner="Locale catalog additions",
        overwrite=True,
    )
    after = len(load_catalog(path))
    print(
        f"{path.name}: {after} keys (+{result.added}, ~{result.updated}, "
        f"={result.unchanged})"
    )
    if after < before:
        raise RuntimeError(f"catalog key count decreased for {path}")


def main() -> None:
    subprocess.check_call([sys.executable, str(SCRIPTS / "gen_i18n_phase1.py")])
    en_extra, zh_extra = load_remaining()
    merge_into(ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml", en_extra)
    merge_into(ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml", zh_extra)
    print("merge ok")


if __name__ == "__main__":
    main()
