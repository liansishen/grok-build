"""Small, incremental helpers for the checked-in locale catalogs."""

from __future__ import annotations

import json
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping


class CatalogError(ValueError):
    """The catalog cannot be updated without losing information."""


@dataclass(frozen=True)
class MergeResult:
    added: int
    updated: int
    unchanged: int
    keys: tuple[str, ...]

    @property
    def changed(self) -> int:
        return self.added + self.updated


SECTION_RE = re.compile(r"^(?P<indent>\s*)\[(?P<section>[^]]+)\]\s*(?:#.*)?(?:\r?\n)?$")
ASSIGNMENT_RE = re.compile(
    r"^(?P<indent>\s*)(?P<key>\"(?:\\.|[^\"])*\"|[A-Za-z0-9_-]+)\s*="
)


def flatten_catalog(value: object, prefix: str = "") -> dict[str, str]:
    """Flatten TOML tables while preserving quoted dotted keys."""

    if isinstance(value, dict):
        result: dict[str, str] = {}
        for key, child in value.items():
            child_prefix = f"{prefix}.{key}" if prefix else str(key)
            result.update(flatten_catalog(child, child_prefix))
        return result
    if isinstance(value, str):
        return {prefix: value}
    raise CatalogError(f"catalog value `{prefix}` is not a string")


def load_catalog(path: Path) -> dict[str, str]:
    try:
        parsed = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise CatalogError(f"cannot read or parse {path}: {error}") from error
    return flatten_catalog(parsed)


def _decode_key(token: str) -> str:
    if token.startswith('"'):
        try:
            value = json.loads(token)
        except json.JSONDecodeError as error:
            raise CatalogError(f"invalid quoted TOML key {token!r}: {error}") from error
        if not isinstance(value, str):
            raise CatalogError(f"TOML key is not a string: {token!r}")
        return value
    return token


def _decode_section(section: str) -> str:
    # Catalog sections currently use bare dotted names. Keep quoted components
    # intact for future generated sections instead of guessing at their value.
    parts = [part.strip() for part in section.split(".")]
    return ".".join(_decode_key(part) for part in parts)


def _line_keys(lines: list[str]) -> dict[str, list[int]]:
    section = ""
    locations: dict[str, list[int]] = {}
    for index, line in enumerate(lines):
        section_match = SECTION_RE.match(line)
        if section_match:
            section = _decode_section(section_match.group("section"))
            continue
        assignment = ASSIGNMENT_RE.match(line)
        if not assignment:
            continue
        key = _decode_key(assignment.group("key"))
        full_key = f"{section}.{key}" if section else key
        locations.setdefault(full_key, []).append(index)
    return locations


def _encode(value: str) -> str:
    # JSON string syntax is also valid TOML basic-string syntax for the values
    # used by the catalogs, and handles Unicode, escapes, and placeholders.
    return json.dumps(value, ensure_ascii=False)


def _key_token(key: str) -> str:
    return key if re.fullmatch(r"[A-Za-z0-9_-]+", key) else _encode(key)


def _section_header(section: str) -> str:
    parts = section.split(".")
    encoded = []
    for part in parts:
        if re.fullmatch(r"[A-Za-z0-9_-]+", part):
            encoded.append(part)
        else:
            encoded.append(_encode(part))
    return f"[{'.'.join(encoded)}]"


def _ensure_newline(line: str) -> str:
    return line if line.endswith(("\n", "\r")) else line + "\n"


def _insert_new_keys(lines: list[str], additions: list[tuple[str, str]], banner: str) -> None:
    if not additions:
        return

    has_sections = any(SECTION_RE.match(line) for line in lines)
    if not has_sections:
        if lines and lines[-1].strip():
            lines.append("\n")
        if banner and (not lines or not lines[-1].strip() == f"# {banner}"):
            lines.append(f"# {banner}\n")
        for key, value in additions:
            lines.append(f"{_encode(key)} = {_encode(value)}\n")
        return

    pending = list(additions)
    while pending:
        key, value = pending.pop(0)
        if "." not in key:
            insertion = next(
                (index for index, line in enumerate(lines) if SECTION_RE.match(line)),
                len(lines),
            )
            lines.insert(insertion, f"{_key_token(key)} = {_encode(value)}\n")
            continue

        section, field = key.rsplit(".", 1)
        header = _section_header(section)
        header_index = next(
            (
                index
                for index, line in enumerate(lines)
                if SECTION_RE.match(line)
                and SECTION_RE.match(line).group("section").strip() == header[1:-1]
            ),
            None,
        )
        if header_index is None:
            if lines and lines[-1].strip():
                lines.append("\n")
            lines.append(f"{header}\n")
            lines.append(f"{_key_token(field)} = {_encode(value)}\n")
            continue

        next_header = next(
            (index for index in range(header_index + 1, len(lines)) if SECTION_RE.match(lines[index])),
            len(lines),
        )
        insert_at = next_header
        if insert_at > header_index and lines[insert_at - 1].strip():
            lines.insert(insert_at, "\n")
            insert_at += 1
        lines.insert(insert_at, f"{_key_token(field)} = {_encode(value)}\n")


def merge_catalog_file(
    path: Path,
    additions: Mapping[str, str],
    *,
    banner: str = "",
    overwrite: bool = True,
) -> MergeResult:
    """Incrementally add/update string keys without rewriting unrelated lines."""

    original = path.read_text(encoding="utf-8")
    existing = load_catalog(path)
    lines = original.splitlines(keepends=True)
    locations = _line_keys(lines)
    added: list[tuple[str, str]] = []
    updates: list[tuple[int, str, str]] = []
    unchanged = 0

    for key, value in additions.items():
        if not isinstance(key, str) or not isinstance(value, str):
            raise CatalogError(f"catalog additions must be string pairs: {key!r}")
        if key not in existing:
            added.append((key, value))
            continue
        if existing[key] == value:
            unchanged += 1
            continue
        if not overwrite:
            raise CatalogError(
                f"refusing to overwrite `{key}` in {path}; existing and generated values differ"
            )
        line_numbers = locations.get(key)
        if not line_numbers:
            raise CatalogError(
                f"cannot locate existing key `{key}` in {path}; use an explicit catalog migration"
            )
        if len(line_numbers) != 1:
            raise CatalogError(f"duplicate key `{key}` in {path}")
        line_number = line_numbers[0]
        match = ASSIGNMENT_RE.match(lines[line_number])
        assert match is not None
        newline = "\r\n" if lines[line_number].endswith("\r\n") else "\n"
        updates.append(
            (
                line_number,
                f"{match.group('indent')}{match.group('key')} = {_encode(value)}{newline}",
                key,
            )
        )

    for line_number, replacement, _ in updates:
        lines[line_number] = replacement
    _insert_new_keys(lines, added, banner)
    result_text = "".join(_ensure_newline(line) for line in lines)

    # Validate before writing. A failed validation leaves the original file in place.
    try:
        flatten_catalog(tomllib.loads(result_text))
    except (tomllib.TOMLDecodeError, CatalogError) as error:
        raise CatalogError(f"generated invalid TOML for {path}: {error}") from error

    if result_text != original:
        path.write_text(result_text, encoding="utf-8")
    return MergeResult(
        added=len(added),
        updated=len(updates),
        unchanged=unchanged,
        keys=tuple(key for key, _ in added) + tuple(key for _, _, key in updates),
    )
