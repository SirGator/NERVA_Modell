import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parents[1]


class FrontendTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--offline", "--manifest-path", str(HERE/"Cargo.toml")], check=True, capture_output=True)
        cls.binary = HERE/"target/debug/nerva-research"

    def invoke(self, mode, config, output):
        return subprocess.run([sys.executable, str(HERE/"run.py"), mode, str(config),
            "--binary", str(self.binary), "--output", str(output)], capture_output=True, text=True)

    def test_plan_run_sqlite_resume_and_readonly_report(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)/"results"
            config = HERE/"config/smoke.toml"
            process = self.invoke("plan", config, root)
            self.assertEqual(process.returncode, 0, process.stderr)
            self.assertIn("16 trials", process.stdout)
            self.assertEqual(list((root/"records").iterdir()), [])
            self.assertFalse((root/"results.sqlite").exists())
            process = self.invoke("run", config, root)
            self.assertEqual(process.returncode, 0, process.stderr)
            with sqlite3.connect(root/"results.sqlite") as db:
                self.assertEqual(db.execute("SELECT count(*) FROM variants").fetchone()[0], 2)
                self.assertEqual(db.execute("SELECT count(*) FROM trials").fetchone()[0], 16)
                self.assertEqual(db.execute("SELECT count(*) FROM metrics").fetchone()[0], 192)
                self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
                self.assertEqual(db.execute("SELECT DISTINCT typeof(seed) FROM trials").fetchall(), [("text",)])
            before = {p.name: p.read_bytes() for p in (root/"records").glob("*.json")}
            for mode in ["run", "report"]:
                process = self.invoke(mode, config, root)
                self.assertEqual(process.returncode, 0, process.stderr)
                self.assertNotIn("COMPLETED", process.stdout)
            self.assertEqual(before, {p.name: p.read_bytes() for p in (root/"records").glob("*.json")})
            self.assertEqual((root/"source.toml").read_bytes(), config.read_bytes())
            self.assertIn("binary_sha256", json.loads((root/"catalog-provenance.json").read_text()))

    def test_corrupted_record_is_rejected_without_overwriting_it(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)/"results"; config = HERE/"config/smoke.toml"
            self.assertEqual(self.invoke("run", config, root).returncode, 0)
            path = next((root/"records").glob("*.json"))
            value = json.loads(path.read_text()); value["record"]["metrics"]["sequence_score"] = 999
            path.write_text(json.dumps(value)); before = path.read_bytes()
            process = self.invoke("run", config, root)
            self.assertNotEqual(process.returncode, 0)
            self.assertIn("integrity", process.stderr)
            self.assertEqual(path.read_bytes(), before)

    def test_invalid_spec_does_not_start_partial_studies(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)
            config = root/"bad.toml"
            config.write_text((HERE/"config/smoke.toml").read_text().replace('"sequence_score"', '"typo_score"'))
            output = root/"output"
            process = self.invoke("run", config, output)
            self.assertNotEqual(process.returncode, 0)
            self.assertFalse(output.exists())
            process = self.invoke("report", HERE/"config/smoke.toml", output)
            self.assertNotEqual(process.returncode, 0)
            self.assertFalse(output.exists())

    def test_requested_example_expands_to_72_seed_variants_and_288_controls(self):
        with TemporaryDirectory() as temp:
            output = Path(temp)/"plan"
            process = self.invoke("plan", HERE/"config/stdp_learning.toml", output)
            self.assertEqual(process.returncode, 0, process.stderr)
            manifest = json.loads((output/"manifest.json").read_text())
            self.assertEqual(manifest["counts"]["variants"]*manifest["counts"]["seeds"], 72)
            self.assertEqual(len(manifest["jobs"]), 288)
            self.assertEqual(manifest["counts"]["full_executions"], 576)


if __name__ == "__main__":unittest.main()
