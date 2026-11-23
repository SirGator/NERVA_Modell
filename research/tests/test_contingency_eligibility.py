from copy import deepcopy
import math
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_eligibility_analysis import ARMS, CHEMICAL_FIELDS, compare_focus, source_record
from contingency_electrical_analysis import f32


class ContingencyEligibilityTest(unittest.TestCase):
    def test_native_checksum_preserves_number_spelling_and_string_whitespace(self):
        envelope = b'{"checksum_fnv1a64":"f4792a2b7b5c00c7","record": { "key": "a b", "n": 1e-7 }}'
        source, checksum = source_record(envelope)
        self.assertEqual(source, {'key': 'a b', 'n': 1e-7})
        self.assertEqual(checksum, 'f4792a2b7b5c00c7')
        with self.assertRaisesRegex(ValueError, 'checksum differs'):
            source_record(envelope.replace(b'1e-7', b'2e-7'))

    def test_terminal_arrival_provenance_and_signed_held_factor_integral(self):
        tau = 100.
        assay = SimpleNamespace(topology=[[4, 8, 9, .5, True], [5, 10, 9, .5, True]],
                                initial={4: .5, 5: .5}, edge_ids={4, 5}, checkpoint=100)
        focus = [
            dict(id=identity, pre=pre, post=9, delay_us=10, initial_factor=-.5,
                 initial_residual=0., initial_tag=1., initial_tag_anchor=90)
            for identity, pre in ((4, 8), (5, 10))
        ]
        # At 100 the old negative signal applies. The sample at 100 changes the
        # sign for 110, and the sample at 110 is held through 130.
        first = tau * 1e-6 * (1. - math.exp(-.1))
        second = f32(math.exp(-.1)) * first
        rounded_at_110 = f32(f32(math.exp(-.1)) * math.exp(-.1))
        third = rounded_at_110 * tau * 1e-6 * (1. - math.exp(-.2))
        last = f32(rounded_at_110 * math.exp(-.2))
        history = {
            'verified_original_local_state': True, 'verified_checkpoint_rule_and_local_clocks': True,
            'clock_times_us': [100, 110, 130], 'chemical_fields': CHEMICAL_FIELDS,
            'chemical_samples': [[100, 9, 0., 0., 0., .25, 0., 0., 1.],
                                 [110, 9, 0., 0., 0., -.75, 0., 0., 1.]],
            'post_spikes': [[110, 9]], 'pre_arrivals': [[100, 5], [110, 5]],
        }
        for arm in ARMS:
            history[arm] = {'synapses': [
                dict(synapse_id=edge['id'], source_id=edge['pre'], target_id=edge['post'],
                     enabled=True, plastic=True, pre_excitatory=True, initial_weight=.5, weight=.5,
                     weight_changes=0, residual=0., eligibility=0.,
                     **{f'{sign}_{field}': 0. for sign in ('positive', 'negative')
                        for field in ('blocked', 'requested', 'actual', 'tag_integral')})
                for edge in focus
            ]}
        history['clear_pairing']['synapses'][0].update(
            positive_requested=second * .25, negative_requested=-(first * .5 + third * .75),
            positive_tag_integral=first + second + third, eligibility=last)
        mapping = {'focus': focus, 'fixed_history': history, 'eligibility_tau_us': tau,
                   'fixed_weights': {arm: [[4, .5], [5, .5]] for arm in ARMS}}
        frames = [{'core_spikes': [[10, 100], [9, 110]]}]
        edges, _ = compare_focus(mapping, frames, {4, 5}, assay)
        self.assertEqual(edges[0]['arrivals'], {'development': 0, 'task': 0})
        self.assertEqual(edges[0]['source_spikes'], 0)
        self.assertEqual(edges[0]['target_spikes'], 1)
        self.assertAlmostEqual(edges[0]['contrasts']['inherited_eligibility']['net_requested'],
                               second * .25 - first * .5 - third * .75)
        self.assertEqual(edges[1]['arrivals'], {'development': 1, 'task': 1})
        self.assertEqual(edges[1]['source_spikes'], 1)
        self.assertIsNone(edges[1]['unpaired_integral'])
        wrong_factor = deepcopy(mapping)
        wrong_factor['fixed_history']['chemical_samples'][0][5] = .5
        with self.assertRaisesRegex(ValueError, 'unpaired inherited tag integral'):
            compare_focus(wrong_factor, frames, {4, 5}, assay)
        with self.assertRaisesRegex(ValueError, 'matching native source spike'):
            compare_focus(mapping, [{'core_spikes': [[9, 110]]}], {4, 5}, assay)


if __name__ == '__main__':
    unittest.main()
