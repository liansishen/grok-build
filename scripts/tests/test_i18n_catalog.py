from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.i18n_catalog import CatalogError, load_catalog, merge_catalog_file


class IncrementalCatalogTests(unittest.TestCase):
    def test_flat_catalog_preserves_unrelated_lines_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "en.toml"
            path.write_text(
                "# hand-maintained header\n"
                '"keep.key" = "Keep this comment nearby"\n'
                "\n",
                encoding="utf-8",
            )
            before = path.read_text(encoding="utf-8")

            first = merge_catalog_file(
                path,
                {"new.key": "New {name}", "keep.key": "Updated"},
                banner="generated additions",
            )
            after_first = path.read_text(encoding="utf-8")
            second = merge_catalog_file(
                path,
                {"new.key": "New {name}", "keep.key": "Updated"},
                banner="generated additions",
            )
            after_second = path.read_text(encoding="utf-8")

            self.assertEqual(first.added, 1)
            self.assertEqual(first.updated, 1)
            self.assertEqual(second.changed, 0)
            self.assertEqual(after_first, after_second)
            self.assertIn("# hand-maintained header", after_first)
            self.assertIn('"keep.key" = "Updated"', after_first)
            self.assertIn('"new.key" = "New {name}"', after_first)
            self.assertNotEqual(before, after_first)
            self.assertEqual(load_catalog(path)["new.key"], "New {name}")

    def test_nested_sections_receive_new_fields_without_reordering_existing_text(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "zh-CN.toml"
            path.write_text(
                "[settings.language]\n"
                'label = "语言"\n'
                "\n"
                "[other]\n"
                'value = "值"\n',
                encoding="utf-8",
            )

            merge_catalog_file(
                path,
                {
                    "settings.language.description": "界面语言",
                    "settings.new.label": "新增",
                },
                banner="settings",
            )

            content = path.read_text(encoding="utf-8")
            self.assertLess(content.index('label = "语言"'), content.index('description = "界面语言"'))
            self.assertIn("[settings.new]", content)
            self.assertEqual(load_catalog(path)["settings.language.description"], "界面语言")
            self.assertEqual(load_catalog(path)["settings.new.label"], "新增")
            self.assertIn('[other]\nvalue = "值"', content)

    def test_non_overwrite_mode_refuses_translation_conflicts_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "zh-CN.toml"
            path.write_text('"key" = "原文"\n', encoding="utf-8")
            before = path.read_text(encoding="utf-8")

            with self.assertRaises(CatalogError):
                merge_catalog_file(path, {"key": "新译文"}, overwrite=False)

            self.assertEqual(path.read_text(encoding="utf-8"), before)


if __name__ == "__main__":
    unittest.main()
