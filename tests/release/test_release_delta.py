#!/usr/bin/env python3
import importlib.util
import pathlib
import sys
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[2] / "scripts/release/check-release-delta.py"
SPEC = importlib.util.spec_from_file_location("release_delta", SCRIPT)
release_delta = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = release_delta
SPEC.loader.exec_module(release_delta)


class ReleaseDeltaHelpersTest(unittest.TestCase):
    def test_matching_domain_is_fail_closed(self):
        contract = {
            "path_rules": [
                {"domain": "runtime", "patterns": ["src/**"]},
                {"domain": "docs", "patterns": ["docs/**"]},
            ]
        }
        self.assertEqual(release_delta.matching_domain("src/cli/mod.rs", contract), "runtime")
        self.assertEqual(release_delta.matching_domain("docs/install.md", contract), "docs")
        self.assertIsNone(release_delta.matching_domain("mystery/new.surface", contract))

    def test_domain_and_global_triggers_are_collected(self):
        contract = {
            "domains": {
                "runtime": {"trigger": "runtime_changed"},
                "docs": {"trigger": "docs_changed"},
            },
            "global_triggers": [
                {"id": "contract_changed", "patterns": ["release/**"]}
            ],
        }
        got = release_delta.triggered_checks(
            ["src/lib.rs", "release/technical-release-contract.json"],
            {"runtime"},
            contract,
        )
        self.assertEqual(got, {"runtime_changed", "contract_changed"})


if __name__ == "__main__":
    unittest.main()
