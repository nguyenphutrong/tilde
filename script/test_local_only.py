#!/usr/bin/env python3
import tempfile
from pathlib import Path
import unittest

from check_local_only import inventory


class LocalOnlyGuardTests(unittest.TestCase):
    def scan(self, manifest, source="", lock='[[package]]\nname="local"\nversion="1"\n'):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "app").mkdir()
            (root / "app/Cargo.toml").write_text(manifest)
            (root / "app/main.rs").write_text(source)
            (root / "Cargo.lock").write_text(lock)
            return inventory(root, ["app/Cargo.toml", "app/main.rs"])

    def test_rejects_renamed_target_specific_optional_runtime(self):
        _, forbidden = self.scan(
            '[target.\'cfg(windows)\'.dependencies]\n'
            'runtime = { package="ort", version="2", optional=true }\n'
        )
        self.assertEqual(forbidden, ["app/Cargo.toml: ort"])

    def test_rejects_transitive_runtime_in_lockfile(self):
        _, forbidden = self.scan("", lock='[[package]]\nname="candle-core"\nversion="1"\n')
        self.assertEqual(forbidden, ["Cargo.lock: candle-core"])

    def test_tracks_cloud_dependencies_even_without_feature_activation(self):
        found, forbidden = self.scan('[dependencies]\nai_types = { workspace=true }\n')
        self.assertEqual(found, {"dependency:app/Cargo.toml:ai_types": 1})
        self.assertEqual(forbidden, [])

    def test_new_endpoint_and_duplicate_are_detected_without_disclosing_value(self):
        line = 'const URL: &str = "https://api.warp.dev/graphql";\n'
        once, _ = self.scan("", line)
        twice, _ = self.scan("", line + line)
        self.assertEqual(list(once.values()), [1])
        self.assertEqual(list(twice.values()), [2])
        self.assertNotIn("api.warp.dev", str(once))
        changed, _ = self.scan("", line.replace("graphql", "new-api"))
        self.assertNotEqual(once, changed)

    def test_preserves_github_updates_and_loopback_networking(self):
        found, forbidden = self.scan("", '\n'.join([
            '"https://api.github.com/repos/nguyenphutrong/tilde/releases/latest"',
            '"http://127.0.0.1:8080"',
            '"ws://localhost:9000"',
        ]))
        self.assertEqual(found, {})
        self.assertEqual(forbidden, [])


if __name__ == "__main__":
    unittest.main()
