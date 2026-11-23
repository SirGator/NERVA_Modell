from copy import deepcopy
import math
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_consolidation_analysis import (
    RULES, archive_training, publish, recency_relay_source, replay_edge,
)
from contingency_electrical_analysis import f32


class ContingencyConsolidationTest(unittest.TestCase):
    def test_reactivated_edge_id_resolves_to_its_archived_source_cell(self):
        focus = {1394: (153, 105, 1000)}
        self.assertEqual(recency_relay_source(34123, focus), 153)
        self.assertIsNone(recency_relay_source(34127, focus))
        with self.assertRaisesRegex(ValueError, 'differs from archived focus'):
            recency_relay_source(34123, {1394: (1394, 105, 1000)})

    def test_publication_preserves_signed_subulp_mass_and_clips_at_bounds(self):
        weight, carry, clipped = publish(.5, 0., 1e-10, True)
        self.assertEqual((weight, carry, clipped), (.5, 1e-10, 0.))
        self.assertEqual(publish(weight, carry, -1e-10, True), (.5, 0., 0.))
        self.assertEqual(publish(.5, 1e-10, -1e-10, False), (.5, 1e-10, 0.))
        self.assertEqual(publish(2., 0., 1e-10, True), (2., 0., 1e-10))
        self.assertEqual(publish(0., 0., -1e-10, False), (0., 0., -1e-10))
        self.assertGreater(publish(.5, 2.9e-8, 2e-9, True)[0], .5)

    def test_replay_integrates_old_factor_and_delayed_tag_before_pairing(self):
        tag = f32(1e-5)
        decay = f32(tag * math.exp(-.001))
        clocks = [1000, 1500, 2000]
        start = [1, .5, 0., 999, 0., .25]
        tags = {(1000, 'post'): (0., tag), (1500, 'pre'): (decay, -tag)}
        factors = {1000: -.5, 1500: 0., 2000: 1.}
        ledgers, counts = replay_edge(start, clocks, factors, tags, RULES)
        native = ledgers['original']
        direct = ledgers['direct_f32']
        constant = ledgers['constant_positive']
        self.assertEqual(counts, {'positive': 0, 'negative': 1, 'zero': 1})
        self.assertLess(native['negative_requested'], 0)
        self.assertEqual(native['positive_requested'], 0)
        self.assertGreater(constant['positive_requested'], 0)
        self.assertEqual(constant['positive_requested'], -constant['negative_requested'])
        self.assertEqual(native['nonzero_requested_without_change'], 1)
        self.assertEqual(native['residual'], native['negative_requested'])
        self.assertEqual(direct['residual'], 0.)
        self.assertEqual(native['weight'], direct['weight'])
        tags[1500, 'pre'] = (0., -tag)
        with self.assertRaisesRegex(ValueError, 'does not follow delayed decay'):
            replay_edge(start, clocks, factors, tags, RULES)

    def test_expanded_instrumentation_preserves_archived_projection(self):
        archived = {'frames': [{'end_us': 9999}],
                    'arrivals': [[1, 7, 12, 14, 1.], [2, 8, 13, 12, 1.]],
                    'tag_events': [[4, 7, 'post', 0., .25]],
                    'quiet_integrations': [[2, 12, 0., 1., 1., 1, True]]}
        expanded = deepcopy(archived)
        expanded['arrivals'].append([3, 9, 13, 15, 1.])
        expanded['tag_events'].append([5, 9, 'post', 0., .125])
        archive_training(expanded, archived, {7}, {12})
        expanded['arrivals'][1][-1] = .5
        with self.assertRaisesRegex(ValueError, 'differs from archived arrivals'):
            archive_training(expanded, archived, {7}, {12})


if __name__ == '__main__':
    unittest.main()
