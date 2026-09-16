#!/usr/bin/env python3
"""Rehearse an upstream merge, classify conflicts, and audit local patch markers.

The rehearsal always runs in a detached temporary worktree. Only conflict hunks
that contain no business code, or identical code with cosmetic differences, may
be replaced automatically with the upstream side. Everything else remains
explicitly unresolved in the report.
"""

from __future__ import annotations

import argparse
import json
import re
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass, replace
from pathlib import Path
from typing import Iterable

MARKER_RE = re.compile(r"LOCAL-PATCH\(([^)\s]+)\)")
REGISTRY_HEADING_RE = re.compile(r"^##\s+`([^`]+)`(?:\s|$)", re.MULTILINE)
LOCALE_PARTS = {"locales", "locale"}
CODE_SUFFIXES = {".rs", ".py", ".toml", ".ps1", ".sh"}
TRANSLATION_NAMES = (
    "xai_grok_i18n::",
    "t_or(",
    "t_fmt(",
    "t_for(",
    "label_t(",
    "description_t(",
    "display_t(",
)


class MergeToolError(RuntimeError):
    """A safe, user-actionable merge-tool failure."""


@dataclass(frozen=True)
class ConflictHunk:
    """One parsed Git conflict hunk and the safe action, if any."""

    index: int
    ours: str
    base: str
    theirs: str
    category: str
    safe_to_take_upstream: bool
    reason: str


@dataclass(frozen=True)
class MarkerAudit:
    registry_ids: tuple[str, ...]
    marker_ids: tuple[str, ...]
    locations: dict[str, tuple[str, ...]]
    errors: tuple[str, ...]

    @property
    def ok(self) -> bool:
        return not self.errors


def run_git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        text=True,
        capture_output=True,
        check=False,
    )
    if check and result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise MergeToolError(f"git {' '.join(args)} failed: {detail}")
    return result


def resolve_repo(path: Path) -> Path:
    result = subprocess.run(
        ["git", "-C", str(path), "rev-parse", "--show-toplevel"],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise MergeToolError(f"not a Git worktree: {path}")
    return Path(result.stdout.strip()).resolve()


def non_comment_code_lines(text: str) -> list[str]:
    """Return conservative code lines, ignoring full-line comments/imports.

    A line that cannot be confidently identified as a comment or import is kept
    as code. This is deliberately conservative because false auto-resolution is
    more dangerous than leaving a cosmetic conflict for review.
    """

    code: list[str] = []
    in_block_comment = False
    for line in text.splitlines():
        value = line.strip()
        if not value:
            continue

        if in_block_comment:
            closing = value.find("*/")
            if closing < 0:
                continue
            in_block_comment = False
            value = value[closing + 2 :].strip()
            if not value:
                continue

        if value.startswith("//"):
            continue
        if value.startswith("/*"):
            closing = value.find("*/", 2)
            if closing < 0:
                in_block_comment = True
                continue
            value = value[closing + 2 :].strip()
            if not value:
                continue
        if value.startswith("*") and value.endswith("*/"):
            continue
        if value.startswith("use ") or value.startswith("pub use "):
            continue
        code.append(value)
    return code


def normalized_code(text: str) -> str:
    return "".join("".join(non_comment_code_lines(text)).split())


def is_locale_path(path: str) -> bool:
    return bool(LOCALE_PARTS.intersection(Path(path).parts)) or Path(path).suffix in {".po", ".mo"}


def contains_marker(text: str) -> bool:
    return MARKER_RE.search(text) is not None


def parse_conflict_hunks(text: str) -> list[ConflictHunk]:
    """Parse standard or diff3 conflict markers without altering the input."""

    lines = text.splitlines(keepends=True)
    hunks: list[ConflictHunk] = []
    cursor = 0
    while cursor < len(lines):
        if not lines[cursor].startswith("<<<<<<<"):
            cursor += 1
            continue

        start = cursor
        separator = None
        end = None
        base_separator = None
        cursor += 1
        while cursor < len(lines):
            line = lines[cursor]
            if line.startswith("|||||||") and separator is None:
                base_separator = cursor
            elif line.startswith("======="):
                separator = cursor
            elif line.startswith(">>>>>>>"):
                end = cursor
                break
            cursor += 1
        if separator is None or end is None:
            raise MergeToolError(f"unterminated conflict marker near line {start + 1}")

        ours_end = base_separator if base_separator is not None else separator
        base_start = base_separator + 1 if base_separator is not None else separator
        base_end = separator
        ours = "".join(lines[start + 1 : ours_end])
        base = "".join(lines[base_start:base_end]) if base_separator is not None else ""
        theirs = "".join(lines[separator + 1 : end])
        hunks.append(
            ConflictHunk(
                index=len(hunks),
                ours=ours,
                base=base,
                theirs=theirs,
                category="unclassified",
                safe_to_take_upstream=False,
                reason="",
            )
        )
        cursor = end + 1
    return hunks


def classify_hunk(path: str, hunk: ConflictHunk) -> ConflictHunk:
    """Classify one hunk and mark only provably cosmetic hunks as safe."""

    if is_locale_path(path):
        return replace(
            hunk,
            category="catalog",
            reason="locale files require key-level three-way merging",
        )

    combined = hunk.ours + hunk.theirs
    if contains_marker(combined):
        return replace(
            hunk,
            category="local-patch",
            reason="the hunk contains a LOCAL-PATCH marker",
        )

    ours_code = normalized_code(hunk.ours)
    theirs_code = normalized_code(hunk.theirs)
    if not ours_code and not theirs_code:
        return replace(
            hunk,
            category="cosmetic",
            safe_to_take_upstream=True,
            reason="both sides contain only comments, imports, or blank lines",
        )
    if ours_code == theirs_code:
        return replace(
            hunk,
            category="cosmetic",
            safe_to_take_upstream=True,
            reason="non-comment code is identical after whitespace normalization",
        )
    if any(name in combined for name in TRANSLATION_NAMES):
        return replace(
            hunk,
            category="i18n",
            reason="translation call shape or fallback text differs",
        )
    if "/tests/" in f"/{path}" or path.endswith("_tests.rs") or path.endswith(".snap"):
        return replace(
            hunk,
            category="test-or-snapshot",
            reason="test or snapshot expectation needs explicit review",
        )
    return replace(
        hunk,
        category="logic",
        reason="both sides contain different non-comment code",
    )


def classify_conflicts(path: str, text: str) -> list[ConflictHunk]:
    return [classify_hunk(path, hunk) for hunk in parse_conflict_hunks(text)]


def replace_safe_hunks(text: str, hunks: Iterable[ConflictHunk]) -> str:
    """Replace safe conflict spans with the upstream side, preserving all else."""

    decisions = {hunk.index: hunk for hunk in hunks}
    lines = text.splitlines(keepends=True)
    output: list[str] = []
    cursor = 0
    index = 0
    while cursor < len(lines):
        if not lines[cursor].startswith("<<<<<<<"):
            output.append(lines[cursor])
            cursor += 1
            continue

        start = cursor
        separator = None
        end = None
        cursor += 1
        while cursor < len(lines):
            if lines[cursor].startswith("======="):
                separator = cursor
            elif lines[cursor].startswith(">>>>>>>"):
                end = cursor
                break
            cursor += 1
        if separator is None or end is None:
            raise MergeToolError(f"unterminated conflict marker near line {start + 1}")

        hunk = decisions.get(index)
        if hunk is None:
            raise MergeToolError("conflict hunk numbering changed while applying decisions")
        if hunk.safe_to_take_upstream:
            output.append(hunk.theirs)
        else:
            output.extend(lines[start : end + 1])
        index += 1
        cursor = end + 1
    return "".join(output)


def marker_audit(repo: Path) -> MarkerAudit:
    registry_path = repo / "LOCAL_PATCHES.md"
    if not registry_path.is_file():
        raise MergeToolError(f"missing {registry_path}")
    registry_text = registry_path.read_text(encoding="utf-8")
    registry_ids = tuple(sorted(set(REGISTRY_HEADING_RE.findall(registry_text))))

    locations: dict[str, list[str]] = {}
    for path in repo.rglob("*"):
        if not path.is_file() or ".git" in path.parts or "target" in path.parts:
            continue
        if path.suffix not in CODE_SUFFIXES:
            continue
        relative_parts = path.relative_to(repo).parts
        if (
            "tests" in relative_parts
            or path.name.startswith("test_")
            or path.stem.endswith("_tests")
        ):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        for match in MARKER_RE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            locations.setdefault(match.group(1), []).append(f"{path.relative_to(repo)}:{line}")

    marker_ids = tuple(sorted(locations))
    errors: list[str] = []
    registry_set = set(registry_ids)
    marker_set = set(marker_ids)
    for patch_id in sorted(marker_set - registry_set):
        errors.append(f"marker id `{patch_id}` has no LOCAL_PATCHES.md section")
    for patch_id in sorted(registry_set - marker_set):
        errors.append(f"LOCAL_PATCHES.md section `{patch_id}` has no source marker")

    return MarkerAudit(
        registry_ids=registry_ids,
        marker_ids=marker_ids,
        locations={key: tuple(value) for key, value in sorted(locations.items())},
        errors=tuple(errors),
    )


def worktree_preflight(repo: Path) -> dict[str, object]:
    """Return a fail-closed cleanliness and worktree-isolation report."""

    repo = resolve_repo(repo)
    status = run_git(repo, "status", "--porcelain", check=True).stdout.splitlines()
    merge_head = run_git(repo, "rev-parse", "-q", "--verify", "MERGE_HEAD", check=False).stdout.strip()
    unresolved = run_git(
        repo, "diff", "--name-only", "--diff-filter=U", check=True
    ).stdout.splitlines()
    diff_check = run_git(repo, "diff", "--check", check=False)
    worktree_lines = run_git(repo, "worktree", "list", "--porcelain", check=True).stdout.splitlines()
    worktrees: list[dict[str, str]] = []
    current: dict[str, str] = {}
    for line in worktree_lines + [""]:
        if line.startswith("worktree "):
            if current:
                worktrees.append(current)
            current = {"path": line.removeprefix("worktree ")}
        elif line.startswith("HEAD "):
            current["head"] = line.removeprefix("HEAD ")
        elif line.startswith("branch "):
            current["branch"] = line.removeprefix("branch ")
        elif not line and current:
            worktrees.append(current)
            current = {}

    branch_paths: dict[str, list[str]] = {}
    for worktree in worktrees:
        branch = worktree.get("branch")
        if branch:
            branch_paths.setdefault(branch, []).append(worktree["path"])

    errors: list[str] = []
    if status:
        errors.append("worktree is dirty")
    if merge_head:
        errors.append(f"merge is in progress ({merge_head})")
    if unresolved:
        errors.append(f"unmerged files: {unresolved}")
    if diff_check.returncode:
        errors.append("git diff --check failed")
    for branch, paths in branch_paths.items():
        if len(paths) > 1:
            errors.append(f"branch {branch} is checked out in multiple worktrees: {paths}")

    target_dir_value = os.environ.get("CARGO_TARGET_DIR")
    target_dir = Path(target_dir_value).expanduser() if target_dir_value else repo / "target"
    if not target_dir.is_absolute():
        target_dir = repo / target_dir
    try:
        target_dir.resolve().relative_to(repo)
    except ValueError:
        errors.append(f"CARGO_TARGET_DIR is outside this worktree: {target_dir}")

    tracked_target = run_git(repo, "ls-files", "--", "target", check=True).stdout.splitlines()
    if tracked_target:
        errors.append(f"build artifacts are tracked under target/: {tracked_target[:5]}")

    return {
        "repo": str(repo),
        "branch": run_git(repo, "branch", "--show-current", check=True).stdout.strip(),
        "head": run_git(repo, "rev-parse", "HEAD", check=True).stdout.strip(),
        "status": status,
        "merge_head": merge_head or None,
        "unresolved_files": unresolved,
        "worktrees": worktrees,
        "cargo_target_dir": str(target_dir.resolve()),
        "tracked_target_files": tracked_target,
        "errors": errors,
        "ok": not errors,
    }


def _worktree_path(parent: Path | None) -> Path:
    if parent is not None:
        parent.mkdir(parents=True, exist_ok=True)
        path = Path(tempfile.mkdtemp(prefix="upstream-merge-", dir=parent))
        path.rmdir()
        return path
    return Path(tempfile.mkdtemp(prefix="grok-upstream-merge-"))


def rehearse(
    repo: Path,
    target: str,
    ours: str | None,
    report_path: Path | None,
    apply_safe: bool,
    require_clean: bool,
    worktree_parent: Path | None,
) -> int:
    repo = resolve_repo(repo)
    status = run_git(repo, "status", "--porcelain", check=True).stdout
    if status:
        raise MergeToolError("formal worktree is dirty; rehearsal refuses to use it")

    ours_ref = ours or run_git(repo, "rev-parse", "HEAD").stdout.strip()
    ours_commit = run_git(repo, "rev-parse", "--verify", ours_ref).stdout.strip()
    target_commit = run_git(repo, "rev-parse", "--verify", target).stdout.strip()
    base_commit = run_git(repo, "merge-base", ours_commit, target_commit).stdout.strip()
    worktree = _worktree_path(worktree_parent)
    merge_result: subprocess.CompletedProcess[str] | None = None
    files: list[dict[str, object]] = []

    try:
        run_git(repo, "worktree", "add", "--detach", str(worktree), ours_commit)
        merge_result = run_git(
            worktree,
            "-c",
            "merge.conflictStyle=diff3",
            "merge",
            "--no-commit",
            "--no-ff",
            target_commit,
            check=False,
        )
        conflict_files = run_git(
            worktree, "diff", "--name-only", "--diff-filter=U", check=True
        ).stdout.splitlines()
        for relative in conflict_files:
            path = worktree / relative
            text = path.read_text(encoding="utf-8", errors="replace")
            hunks = classify_conflicts(relative, text)
            before = text
            if apply_safe:
                text = replace_safe_hunks(text, hunks)
                if text != before:
                    path.write_text(text, encoding="utf-8")
            remaining = parse_conflict_hunks(text)
            if apply_safe and not remaining:
                run_git(worktree, "add", "--", relative)
            files.append(
                {
                    "path": relative,
                    "hunks": [asdict(hunk) for hunk in hunks],
                    "remaining_hunks": len(remaining),
                    "safe_applied": apply_safe and text != before,
                }
            )

        counts: dict[str, int] = {}
        for item in files:
            for hunk in item["hunks"]:  # type: ignore[union-attr]
                category = hunk["category"]  # type: ignore[index]
                counts[category] = counts.get(category, 0) + 1
        unresolved = [
            item["path"]
            for item in files
            if int(item["remaining_hunks"]) > 0
        ]
        report: dict[str, object] = {
            "tool": "scripts/upstream_merge.py",
            "ours": ours_commit,
            "target": target_commit,
            "merge_base": base_commit,
            "merge_exit_code": merge_result.returncode,
            "merge_stdout": merge_result.stdout,
            "merge_stderr": merge_result.stderr,
            "conflicted_files": len(files),
            "conflict_hunks": sum(counts.values()),
            "category_counts": dict(sorted(counts.items())),
            "unresolved_files_after_safe_apply": unresolved,
            "files": files,
        }
        encoded = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
        if report_path:
            report_path.parent.mkdir(parents=True, exist_ok=True)
            report_path.write_text(encoded, encoding="utf-8")
        else:
            print(encoded, end="")

        if require_clean and unresolved:
            return 2
        return 0
    finally:
        if merge_result is not None and (worktree / ".git").exists():
            run_git(worktree, "merge", "--abort", check=False)
        run_git(repo, "worktree", "remove", "--force", str(worktree), check=False)
        if worktree.exists():
            shutil.rmtree(worktree, ignore_errors=True)


def print_marker_audit(repo: Path, as_json: bool) -> int:
    audit = marker_audit(resolve_repo(repo))
    payload = asdict(audit) | {"ok": audit.ok}
    if as_json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"registry ids: {', '.join(audit.registry_ids) or '(none)'}")
        print(f"source marker ids: {', '.join(audit.marker_ids) or '(none)'}")
        for patch_id, locations in audit.locations.items():
            print(f"{patch_id}: {', '.join(locations)}")
        for error in audit.errors:
            print(f"ERROR: {error}", file=sys.stderr)
    return 0 if audit.ok else 1


def print_preflight(repo: Path, as_json: bool) -> int:
    report = worktree_preflight(repo)
    if as_json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(f"worktree: {report['repo']}")
        print(f"branch: {report['branch']}")
        print(f"target: {report['cargo_target_dir']}")
        for error in report["errors"]:
            print(f"ERROR: {error}", file=sys.stderr)
    return 0 if report["ok"] else 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    rehearsal = subparsers.add_parser("rehearse", help="run a merge in a detached worktree")
    rehearsal.add_argument("target", help="upstream ref or commit to merge")
    rehearsal.add_argument("--repo", type=Path, default=Path.cwd())
    rehearsal.add_argument("--ours", help="ours ref; defaults to the current HEAD")
    rehearsal.add_argument("--report", type=Path)
    rehearsal.add_argument("--apply-safe", action="store_true")
    rehearsal.add_argument("--require-clean", action="store_true")
    rehearsal.add_argument(
        "--worktree-parent",
        type=Path,
        help="private parent directory for the temporary worktree",
    )

    audit = subparsers.add_parser("audit-markers", help="check LOCAL-PATCH registry consistency")
    audit.add_argument("--repo", type=Path, default=Path.cwd())
    audit.add_argument("--json", action="store_true")


    preflight = subparsers.add_parser("preflight", help="check worktree and build isolation")
    preflight.add_argument("--repo", type=Path, default=Path.cwd())
    preflight.add_argument("--json", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "audit-markers":
            return print_marker_audit(args.repo, args.json)
        if args.command == "preflight":
            return print_preflight(args.repo, args.json)
        return rehearse(
            repo=args.repo,
            target=args.target,
            ours=args.ours,
            report_path=args.report,
            apply_safe=args.apply_safe,
            require_clean=args.require_clean,
            worktree_parent=args.worktree_parent,
        )
    except MergeToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
