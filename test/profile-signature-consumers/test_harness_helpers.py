"""Small deterministic regressions for the integration harness."""
import unittest
import json
import os
import tempfile
import argparse
import sys
from pathlib import Path
from unittest.mock import patch

from extract_magic_link import extract_code, extract_display_url
from collect_evidence import export
from write_test_config import adapter_config
from run_step import run


class MagicLinkParsingTests(unittest.TestCase):
    def test_wrapped_stub_url(self):
        text = "\x1b[32mhttp://127.0.0.1:1234/_adm/beginners/users/magic-link/display?token=abc\n    &email=test%40localhost\x1b[0m"
        self.assertEqual(
            extract_display_url(text, "http://127.0.0.1:1234"),
            "http://127.0.0.1:1234/_adm/beginners/users/magic-link/display?token=abc&email=test%40localhost",
        )

    def test_html_entities(self):
        value = '<a href="http://localhost:1234/magic-link&#x2F;display?token=abc&amp;email=a">'
        self.assertEqual(extract_display_url(value), "http://localhost:1234/magic-link/display?token=abc&email=a")

    def test_wrapped_stub_footer_does_not_join_the_url(self):
        text = "http://localhost:12/magic-link/display?token=x&email=a\n    [2]: footer"
        self.assertEqual(extract_display_url(text), "http://localhost:12/magic-link/display?token=x&email=a")

    def test_rejects_external_or_wrong_port(self):
        self.assertEqual(extract_display_url("https://example.com/magic-link/display?token=x"), "")
        self.assertEqual(extract_display_url("http://localhost:99/magic-link/display?token=x", "http://localhost:98"), "")

    def test_code_accepts_extra_attributes(self):
        self.assertEqual(extract_code('<div class="code" id="login-code">123456</div>'), "123456")
        self.assertEqual(extract_code('<div id="code" class="large code">\n 123456\n</div>'), "123456")

    def test_invalid_codes_fail_closed(self):
        for value in ["12345", "abc123", "1234567", "<script>123456</script>"]:
            self.assertEqual(extract_code(f'<div class="code">{value}</div>'), "")


class EvidenceAndConfigTests(unittest.TestCase):
    def test_repeated_command_ids_keep_both_attempt_logs(self):
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory)
            for value in ("first", "second"):
                args = argparse.Namespace(evidence=evidence, id="repeat", timeout=5,
                    command=[sys.executable, "-c", f"print('{value}')"])
                self.assertEqual(run(args), 0)
            records = [json.loads(line) for line in (evidence / "commands.jsonl").read_text().splitlines()]
            self.assertNotEqual(records[0]["logPath"], records[1]["logPath"])
            self.assertEqual((evidence / records[0]["logPath"]).read_text().strip(), "first")
            self.assertEqual((evidence / records[1]["logPath"]).read_text().strip(), "second")

    def test_interrupted_benchmark_exports_lifecycle_without_claiming_e2e(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            runtime = base / "runtime"
            runtime.mkdir()
            output = base / "public"
            export(runtime, output, "full", "synthetic-run", 143, "benchmark")
            result = json.loads((output / "lifecycle-full.json").read_text())
            self.assertEqual(result["status"], "interrupted")
            self.assertFalse((output / "e2e-full.json").exists())

    def test_runtime_credentials_are_not_exported_and_failure_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime = Path(directory) / "runtime"
            output = Path(directory) / "public"
            runtime.mkdir()
            (runtime / "api.log").write_text("PRIVATE-MARKER synthetic token")
            (runtime / "pair.json").write_text('{"signature":"PRIVATE-MARKER"}')
            (runtime / "results.jsonl").write_text(json.dumps(dict(id="test", label="completed assertion", outcome="pass")) + "\n")
            export(runtime, output, "full", "synthetic-run", 7)
            self.assertEqual(json.loads((output / "e2e-full.json").read_text())["exitCode"], 7)
            for path in output.iterdir():
                self.assertNotIn("PRIVATE-MARKER", path.read_text())

    def test_adapter_settings_validate_ports_and_use_env_resolvers(self):
        with patch.dict(os.environ, {"PSIG_SMTP_PORT": "1234", "PSIG_REDIS_PORT": "5678"}):
            text = adapter_config("full")
            self.assertIn('security = "starttls"', text)
            self.assertIn('env = "PSIG_DATABASE_URL"', text)
        with patch.dict(os.environ, {"PSIG_SMTP_PORT": "65536"}):
            with self.assertRaises(ValueError):
                adapter_config("postgres-only")


if __name__ == "__main__":
    unittest.main()
