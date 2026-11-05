import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parents[1]


class BaselineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(["cargo", "build", "--offline", "--manifest-path", str(HERE / "Cargo.toml")],
                       check=True, capture_output=True)
        cls.binary = HERE / "target/debug/nerva-research"
        cls.temp = TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.reference = Path(cls.temp.name) / "reference"
        result = cls.invoke("run", cls.reference)
        if result.returncode:
            raise AssertionError(result.stderr)

    @classmethod
    def invoke(cls, mode, output):
        return subprocess.run([sys.executable, str(HERE / "baseline.py"), mode,
                               "--binary", str(cls.binary), "--output", str(output)],
                              capture_output=True, text=True)

    def test_fixed_reference_and_verify_without_new_trials(self):
        with TemporaryDirectory() as temp:
            root = Path(temp) / "baseline"
            shutil.copytree(self.reference, root)
            receipt_bytes = (root / "baseline-receipt.json").read_bytes()
            receipt = json.loads(receipt_bytes)["record"]
            self.assertEqual(receipt["counts"]["trials"], 24)
            self.assertEqual(receipt["counts"]["full_executions"], 48)
            self.assertEqual(len(receipt["metrics"]), 12)
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertNotIn("base", manifest["spec"])
            self.assertEqual(len(manifest["variants"]), 1)
            result = self.invoke("verify", root)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("COMPLETED", result.stdout)
            self.assertEqual((root / "baseline-receipt.json").read_bytes(), receipt_bytes)
            self.assertEqual({p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                              for p in (root / "records").glob("*.json")}, receipt["records_sha256"])
            # Resuming a sealed baseline also uses report only.
            result = self.invoke("run", root)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("COMPLETED", result.stdout)

    def test_verify_rejects_tampering_and_missing_records_without_repair(self):
        for mutation in ["metric", "missing", "extra", "manifest", "receipt"]:
            with self.subTest(mutation=mutation), TemporaryDirectory() as temp:
                root = Path(temp) / "baseline"
                shutil.copytree(self.reference, root)
                record = next((root / "records").glob("*.json"))
                if mutation == "metric":
                    data = json.loads(record.read_bytes())
                    data["record"]["metrics"]["sequence_score"] = 999
                    record.write_text(json.dumps(data))
                elif mutation == "missing":
                    record.unlink()
                elif mutation == "extra":
                    (root / "records/extra.json").write_text("{}")
                elif mutation == "manifest":
                    (root / "manifest.json").write_text("{}")
                else:
                    path = root / "baseline-receipt.json"
                    data = json.loads(path.read_bytes())
                    data["sha256"] = "0" * 64
                    path.write_text(json.dumps(data))
                before = {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}
                result = self.invoke("verify", root)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("COMPLETED", result.stdout)
                self.assertEqual({str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}, before)

    def test_verify_requires_existing_receipt(self):
        with TemporaryDirectory() as temp:
            root = Path(temp) / "missing"
            result = self.invoke("verify", root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("existing baseline receipt", result.stderr)
            self.assertFalse(root.exists())


if __name__ == "__main__":
    unittest.main()
