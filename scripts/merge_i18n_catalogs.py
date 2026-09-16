#!/usr/bin/env python3
"""Run the catalog generators without rewriting unrelated locale entries."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from i18n_catalog import (
    catalog_pair_is_valid,
    catalog_pair_report,
    load_catalog,
    merge_catalog_file,
)


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


def validate_catalog_pair(en_path: Path, zh_path: Path) -> None:
    report = catalog_pair_report(load_catalog(en_path), load_catalog(zh_path))
    print(
        "source report: "
        f"duplicates={len(report['duplicate_sources'])}, "
        f"ambiguous={len(report['ambiguous_sources'])}"
    )
    if not catalog_pair_is_valid(report):
        raise RuntimeError(json.dumps(report, ensure_ascii=False, indent=2))


def main() -> None:
    subprocess.check_call([sys.executable, str(SCRIPTS / "gen_i18n_phase1.py")])
    en_extra, zh_extra = load_remaining()
    merge_into(ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml", en_extra)
    merge_into(ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml", zh_extra)
    validate_catalog_pair(
        ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml",
        ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml",
    )
    print("merge ok")


if __name__ == "__main__":
    main()
