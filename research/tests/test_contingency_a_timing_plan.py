import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_a_timing_plan import arrival, build_plan, plan_directory

ARCHIVE = Path(__file__).resolve().parents[1] / 'results/visual_contingency_a_source_20261123'


class ArrivalSelectionTest(unittest.TestCase):
    def test_sign_time_and_uniqueness(self):
        probe = dict(start_us=100, arrivals=[[101, 7, 9, -1.], [105, 7, 9, 1.], [110, 7, 9, 2.]])
        self.assertEqual(arrival(probe, 9, False), (0, 1))
        self.assertEqual(arrival(probe, 9, True, latest_before=10), (1, 5))
        self.assertEqual(arrival(probe, 9, True, at=10), (2, 10))
        probe['arrivals'].append([110, 7, 9, 3.])
        with self.assertRaisesRegex(ValueError, 'not unique'):
            arrival(probe, 9, True, at=10)


@unittest.skipUnless((ARCHIVE / 'raw/v00000-s34313-i000.json').exists(), 'full archive not present')
class ArchivedTimingPlanTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = {seed: [(ARCHIVE / part / f'v00000-s{seed}-i000.json').read_bytes()
                            for part in ('growth_input', 'transfer_input', 'plans', 'raw')]
                      for seed in (34313, 34327)}
        cls.plans = {seed: build_plan(*data) for seed, data in cls.inputs.items()}

    def test_archive_predicates_and_unchanged_probes(self):
        for seed, plan in self.plans.items():
            raw = json.loads(self.inputs[seed][3])
            self.assertEqual(plan['source_plan'], json.loads(self.inputs[seed][2]))
            for carrier in plan['carriers'].values():
                family = carrier['source_family']
                prior = raw['native'] if family == 'native' else raw['grafts'][family]
                self.assertEqual(carrier['probes'], prior[carrier['source_condition']]['probes'])
                self.assertEqual(carrier['arms']['reference'], [])
                for edits in carrier['arms'].values():
                    self.assertEqual(edits, sorted(edits))
                    self.assertTrue(all(0 <= at < 200000 for _, at in edits))
        native = self.plans[34313]['carriers']['native']
        self.assertEqual(len(native['arms']['exc_spread']), 2)
        self.assertEqual(len(native['arms']['exc_translate']), 3)
        carriers = self.plans[34327]['carriers']
        self.assertEqual(carriers['native']['arms']['inh_delay'][0][1], 36335)
        self.assertEqual(carriers['native']['arms']['inh_advance'][0][1], 1001)
        for name, expected in [('exc_align', 43668), ('exc_left', 42668), ('exc_right', 44668)]:
            self.assertEqual(carriers['grown']['arms'][name][0][1], expected)
        self.assertEqual([at for _, at in carriers['rotated']['arms']['exc_translate_pair']], [35335, 35335])

    def test_rejects_unchecked_or_modified_prior(self):
        data = list(self.inputs[34313])
        raw = json.loads(data[3])
        raw['full_replay_checked'] = False
        with self.assertRaisesRegex(ValueError, 'unexpected A source record'):
            build_plan(*data[:3], json.dumps(raw).encode())
        prior = json.loads(data[2])
        prior['active_edges'].reverse()
        with self.assertRaisesRegex(ValueError, 'independent reconstruction'):
            build_plan(*data[:2], json.dumps(prior).encode(), data[3])

    def test_directory_outputs_deterministic_job_named_plans(self):
        with tempfile.TemporaryDirectory() as output:
            names = plan_directory(ARCHIVE, output)
            before = {name: (Path(output) / name).read_bytes() for name in names}
            self.assertEqual(plan_directory(ARCHIVE, output), names)
            self.assertEqual(before, {name: (Path(output) / name).read_bytes() for name in names})
            self.assertEqual(len(names), 2)


if __name__ == '__main__':
    unittest.main()
