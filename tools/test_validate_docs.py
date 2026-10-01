#!/usr/bin/env python3
"""Regression tests for the repository documentation validator."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import validate_docs


class DocumentationValidatorTest(unittest.TestCase):
  """Exercise each contract enforced by the validator."""

  def setUp(self) -> None:
    """Create a minimal mirrored documentation tree."""
    self.temporary_directory = tempfile.TemporaryDirectory()
    self.root = Path(self.temporary_directory.name)
    (self.root / "docs").mkdir()
    (self.root / "docs_cn").mkdir()

  def tearDown(self) -> None:
    """Remove the temporary documentation tree."""
    self.temporary_directory.cleanup()

  def write_pair(self, english: str, chinese: str) -> None:
    """Write one English page and its Chinese mirror."""
    (self.root / "docs" / "page.md").write_text(english, encoding="utf-8")
    (self.root / "docs_cn" / "page_cn.md").write_text(chinese, encoding="utf-8")

  def test_accepts_valid_json_links_and_heading_parity(self) -> None:
    """Accept a mirrored page whose JSON and local anchor are valid."""
    self.write_pair(
      '# Page\n\n[Section](#section)\n\n## Section\n\n```json\n{"key": "value"}\n```\n',
      '# 页面\n\n[小节](#小节)\n\n## 小节\n\n```json\n{"key": "值"}\n```\n',
    )
    self.assertEqual(validate_docs.validate(self.root), [])

  def test_rejects_duplicate_json_keys(self) -> None:
    """Reject repeated keys even though the standard decoder accepts them."""
    self.write_pair('# Page\n\n```json\n{"url": "a", "url": "b"}\n```\n', '# 页面\n')
    messages = [finding.message for finding in validate_docs.validate(self.root)]
    self.assertTrue(any("duplicate JSON key 'url'" in message for message in messages))

  def test_rejects_broken_link_and_anchor(self) -> None:
    """Reject missing files and missing anchors in existing Markdown files."""
    self.write_pair(
      '# Page\n\n[Missing](missing.md) [Anchor](#absent)\n',
      '# 页面\n',
    )
    messages = [finding.message for finding in validate_docs.validate(self.root)]
    self.assertTrue(any("target does not exist" in message for message in messages))
    self.assertTrue(any("anchor does not exist" in message for message in messages))

  def test_rejects_broken_html_link(self) -> None:
    """Reject missing local targets referenced through inline HTML."""
    self.write_pair('# Page\n\n<img src="missing.png">\n', '# 页面\n')
    messages = [finding.message for finding in validate_docs.validate(self.root)]
    self.assertTrue(any("target does not exist" in message for message in messages))

  def test_rejects_heading_level_drift(self) -> None:
    """Reject a translation whose heading hierarchy differs from English."""
    self.write_pair('# Page\n\n## Section\n', '# 页面\n\n### 小节\n')
    messages = [finding.message for finding in validate_docs.validate(self.root)]
    self.assertTrue(any("heading levels differ" in message for message in messages))


if __name__ == "__main__":
  unittest.main()
