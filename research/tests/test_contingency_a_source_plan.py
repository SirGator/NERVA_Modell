import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_a_source_plan import (build_plan, replacements, shortest_cycle, source_options)


class ASourcePlanTest(unittest.TestCase):
    def test_source_options_enforce_occupancy_polarity_radius_and_length(self):
        positions = {0: ([0., 0., 0.], True), 1: ([.1, 0., 0.], False),
                     2: ([.01, 0., 0.], True), 3: ([.02, 0., 0.], True),
                     4: ([.01, 0., 0.], False), 5: ([.2, 0., 0.], True),
                     6: ([.4, 0., 0.], True)}
        graft = [[10, 0, 1, .25, 1000, True]]
        options = source_options([[1, 2, 1, .3, 1000, False]], graft, positions, [10])
        # Even an initially disabled occupied pair is unavailable.
        self.assertEqual([source for _, source in options[10]], [5, 3])
        self.assertEqual(graft[0][1], 0)

    def test_assignment_maximizes_count_then_deviation_then_lexical_pairs(self):
        with patch('contingency_a_source_plan.source_options', return_value={
                10: [(0., 1)], 11: [(.1, 1)], 12: [(0., 2)]}):
            singles, group = replacements([], [], {}, [10, 11, 12])
        self.assertEqual(singles, [[10, 1], [11, 1], [12, 2]])
        self.assertEqual(group, [[10, 1], [12, 2]])
        with patch('contingency_a_source_plan.source_options', return_value={
                10: [(0., 1)], 11: [(0., 1)], 12: [(0., 2)]}):
            self.assertEqual(replacements([], [], {}, [10, 11, 12])[1],
                             [[10, 1], [12, 2]])
        with patch('contingency_a_source_plan.source_options', return_value={
                10: [(0., 1), (.2, 3)], 11: [(.1, 1)], 12: [(0., 2)]}):
            self.assertEqual(replacements([], [], {}, [10, 11, 12])[1],
                             [[10, 3], [11, 1], [12, 2]])

    def test_cycle_search_shortest_lexical_and_excludes_other_target_slots(self):
        graft = [[10, 0, 20, .25, 1000, True], [11, 1, 21, .25, 1000, True],
                 [12, 2, 22, .25, 1000, True], [13, 3, 20, .25, 1000, True],
                 [14, 4, 24, .25, 1000, True]]
        options = {10: [(0., 1), (0., 2), (0., 3)], 11: [(0., 4)],
                   12: [(0., 4)], 13: [(0., 0)], 14: [(0., 0)]}
        with patch('contingency_a_source_plan.source_options', return_value=options):
            self.assertEqual(shortest_cycle([], graft, {}, [10], 20), [10, 11, 14])
            self.assertEqual(shortest_cycle([], graft, {}, [10], 20, max_length=2), [])



ARCHIVE = Path(__file__).resolve().parents[1] / 'results/visual_contingency_a_transfer_20261123'


@unittest.skipUnless((ARCHIVE / 'raw/v00000-s34313-i000.json').exists(), 'full archive not present')
class ArchivedASourcePlanTest(unittest.TestCase):
    def test_archived_plans_and_checked_prior_proof(self):
        expected = {34313: ([1632, 1602, 1515, 1438], [1632, 1602, 1515, 1438]),
                    34327: ([1632, 1545, 1445, 1649, 1433], [1632, 1445, 1486, 1439, 1467])}
        for seed, cycles in expected.items():
            name = f'v00000-s{seed}-i000.json'
            growth = (ARCHIVE / 'input' / name).read_bytes()
            transfer = (ARCHIVE / 'raw' / name).read_bytes()
            plan = build_plan(growth, transfer)
            self.assertEqual(len(plan['active_edges']), 3)
            self.assertEqual(len(plan['silent_edges']), 2)
            for index, graft in enumerate(('native_initial', 'sham_initial')):
                self.assertEqual(plan['grafts'][graft]['cycle'], cycles[index])
                self.assertEqual(len(plan['grafts'][graft]['group_replacements']), 2 if seed == 34313 else 3)
            altered = json.loads(transfer)
            altered['full_replay_checked'] = False
            with self.assertRaisesRegex(ValueError, 'unexpected A transfer record'):
                build_plan(growth, json.dumps(altered).encode())


if __name__ == '__main__':
    unittest.main()
