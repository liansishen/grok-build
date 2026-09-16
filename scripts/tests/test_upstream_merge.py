from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.upstream_merge import (
    ConflictHunk,
    classify_conflicts,
    marker_audit,
    parse_conflict_hunks,
    replace_safe_hunks,
)


class ConflictClassificationTests(unittest.TestCase):
    def test_diff3_hunk_keeps_the_base_and_classifies_comments_as_safe(self) -> None:
        text = (
            "before\n"
            "<<<<<<< HEAD\n"
            "// fork wording\n"
            "||||||| base\n"
            "// base wording\n"
            "=======\n"
            "// upstream wording\n"
            ">>>>>>> upstream\n"
            "after\n"
        )

        hunks = parse_conflict_hunks(text)

        self.assertEqual(len(hunks), 1)
        self.assertEqual(hunks[0].base, "// base wording\n")
        classified = classify_conflicts("src/app.rs", text)
        self.assertEqual(classified[0].category, "cosmetic")
        self.assertTrue(classified[0].safe_to_take_upstream)

    def test_different_code_is_left_for_review(self) -> None:
        text = (
            "<<<<<<< HEAD\n"
            "let decision = Decision::Allow;\n"
            "=======\n"
            "let decision = Decision::Reject;\n"
            ">>>>>>> upstream\n"
        )

        hunk = classify_conflicts("src/permission.rs", text)[0]

        self.assertEqual(hunk.category, "logic")
        self.assertFalse(hunk.safe_to_take_upstream)

    def test_i18n_and_local_patch_are_never_auto_resolved(self) -> None:
        i18n_text = (
            '<<<<<<< HEAD\n'
            'xai_grok_i18n::t_or("old.key", "Old")\n'
            '=======\n'
            'show_toast("New")\n'
            '>>>>>>> upstream\n'
        )
        patch_text = (
            "<<<<<<< HEAD\n"
            "// LOCAL-PATCH(example)\n"
            "let value = fork_value();\n"
            "=======\n"
            "let value = upstream_value();\n"
            ">>>>>>> upstream\n"
        )

        i18n = classify_conflicts("src/view.rs", i18n_text)[0]
        patch = classify_conflicts("src/view.rs", patch_text)[0]

        self.assertEqual(i18n.category, "i18n")
        self.assertFalse(i18n.safe_to_take_upstream)
        self.assertEqual(patch.category, "local-patch")
        self.assertFalse(patch.safe_to_take_upstream)

    def test_replace_safe_hunks_preserves_unresolved_hunks(self) -> None:
        text = (
            "<<<<<<< HEAD\n"
            "// fork\n"
            "=======\n"
            "// upstream\n"
            ">>>>>>> upstream\n"
            "<<<<<<< HEAD\n"
            "let value = 1;\n"
            "=======\n"
            "let value = 2;\n"
            ">>>>>>> upstream\n"
        )
        hunks = classify_conflicts("src/app.rs", text)

        result = replace_safe_hunks(text, hunks)

        self.assertNotIn("// fork", result)
        self.assertIn("// upstream", result)
        self.assertIn("<<<<<<< HEAD", result)
        self.assertIn("let value = 1;", result)


class MarkerAuditTests(unittest.TestCase):
    def test_marker_audit_matches_registry_sections(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "LOCAL_PATCHES.md").write_text(
                "## `example-patch` (2026-09-16)\n\nDetails.\n",
                encoding="utf-8",
            )
            source = repo / "src.rs"
            source.write_text(
                "// LOCAL-PATCH(example-patch): preserve behavior\n",
                encoding="utf-8",
            )

            audit = marker_audit(repo)

            self.assertTrue(audit.ok)
            self.assertEqual(audit.registry_ids, ("example-patch",))
            self.assertEqual(audit.marker_ids, ("example-patch",))

    def test_marker_audit_rejects_unregistered_ids(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "LOCAL_PATCHES.md").write_text(
                "## `known`\n",
                encoding="utf-8",
            )
            (repo / "src.rs").write_text(
                "// LOCAL-PATCH(unknown)\n",
                encoding="utf-8",
            )

            audit = marker_audit(repo)

            self.assertFalse(audit.ok)
            self.assertIn("unknown", " ".join(audit.errors))


if __name__ == "__main__":
    unittest.main()
