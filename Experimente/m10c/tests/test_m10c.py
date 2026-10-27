import copy
import importlib.util
import json
import math
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

HERE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("m10c_run", HERE/"run.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
ROOT = HERE.parents[2]


def fixture(simultaneous=False):
    result = dict(effective_parameters=dict(core_membrane_tau_us=20000, core_threshold=1),
                  initial_synapses=[dict(id=0, pre=3, post=2), dict(id=1, pre=4, post=2)],
                  initial_neurons=[dict(id=3, polarity="Excitatory"), dict(id=4, polarity="Inhibitory")])
    arrivals = [(1000, 0, 3, 1.2), (1000, 1, 4, -.3)] if simultaneous else [
        (1000, 0, 3, 1.2), (2000, 0, 3, .5), (8000, 1, 4, -.2)]
    events = [dict(kind="arrival", time_us=t, synapse=e, source=s, target=2, amplitude=a)
              for t, e, s, a in arrivals]
    if not simultaneous:events.append(dict(kind="spike", time_us=1000, source=2))
    final = (.9*math.exp(-49000/20000) if simultaneous else -.2*math.exp(-42000/20000))
    phases = [dict(before_us=0, end_us=10000, potentials_before=[0.0]*32,
                   refractory_until_before=[0]*32, events=events),
              dict(before_us=10000, end_us=20000, events=[]),
              dict(before_us=20000, end_us=50000, events=[],
                   potentials_after=[final]+[0.0]*31,
                   refractory_until_after=[0 if simultaneous else 6000]+[0]*31)]
    q = dict(action_us=1, forecast_us=50000, trace=[{}]+phases,
             core_counts=[0 if simultaneous else 1]+[0]*31)
    return q, result


class AccountingTests(unittest.TestCase):
    def test_resets_can_reverse_residual_without_removing_spikes(self):
        q, result = fixture()
        r = m.core.reconstruct(q, result)
        v = r["vectors"]
        self.assertGreater(v["linear_drive"][0], 0)
        self.assertLess(v["residual_voltage"][0], 0)
        self.assertEqual(v["spike_counts"][0], 1)
        self.assertAlmostEqual(v["discarded_leak"][0], .5*math.exp(-48000/20000))
        self.assertAlmostEqual(v["reset_loss"][0], 1.2*math.exp(-49000/20000))
        self.assertAlmostEqual(v["linear_drive"][0]-v["reset_loss"][0], v["residual_voltage"][0])
        for signal in ["all_input", "all_leak_input", "accepted_leak_input"]:
            self.assertAlmostEqual(sum(g[signal][0] for g in r["groups"].values()), v[signal][0])
            self.assertAlmostEqual(sum(e[signal] for e in r["edges"].values()), v[signal][0])

    def test_simultaneous_excitation_and_inhibition_are_batched(self):
        q, result = fixture(True)
        r = m.core.reconstruct(q, result)
        self.assertEqual(r["vectors"]["spike_counts"][0], 0)
        self.assertAlmostEqual(r["vectors"]["all_input"][0], .9)

    def test_corrupted_voltage_and_spikes_are_rejected(self):
        q, result = fixture()
        bad = copy.deepcopy(q)
        bad["trace"][-1]["potentials_after"][0] += .01
        with self.assertRaisesRegex(ValueError, "voltage reconstruction"):
            m.core.reconstruct(bad, result)
        bad = copy.deepcopy(q)
        bad["trace"][1]["events"] = [e for e in bad["trace"][1]["events"] if e["kind"] != "spike"]
        with self.assertRaisesRegex(ValueError, "missing recorded spike"):
            m.core.reconstruct(bad, result)

    def test_projection_respects_scaling_and_mapping(self):
        decoder = dict(scale=[2, 1], centers={"-1": [-1, 0], "1": [1, 0]})
        c = m.core.coefficients(decoder)
        self.assertEqual(c, [.5, 0])
        self.assertEqual(m.core.project(c, [8, 0], [12, 20], 1)[0:2], (2, 1))
        self.assertEqual(m.core.project(c, [8, 0], [12, 20], -1)[0:2], (-2, -1))


class ArchivedTraceTests(unittest.TestCase):
    def test_smoke_reproduces_count_axis_and_rejects_manifest_changes(self):
        source = ROOT/"Messungen/Experimente/m10-path-audit"
        with TemporaryDirectory() as temp:
            output = Path(temp)
            manifest = m.plan(source, output, "smoke")
            path = output/"manifest.json"
            path.write_text(json.dumps(manifest))
            m.report(output)
            audit = json.loads((output/"diagnostics.json").read_text())
            self.assertEqual(audit["pairs"], 48)
            self.assertEqual(audit["windows"], 96)
            self.assertEqual(audit["new_neural_runs"], 0)
            self.assertTrue(audit["original_count_axis_reproduced"])
            manifest["traces"]["traces/seed-610.json"] = "changed"
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "manifest/source differs"):
                m.load(output)


if __name__ == "__main__":unittest.main()
