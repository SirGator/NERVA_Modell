from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from contingency_b_factor_analysis import contrasts, plans


class BFactorTest(unittest.TestCase):
    def test_plans_restore_only_changed_enabled_persistent_and_partition_b_births(self):
        a = [[0, 64, 65, .5, 1000, True], [1, 65, 66, .2, 1000, False],
             [2, 66, 67, .7, 1000, True]]
        b = deepcopy(a)
        b[0][3] = .9
        b[1][3] = .4
        b += [[3, 64, 70, .6, 1000, True], [4, 64, 71, .8, 1000, True]]
        result = plans({'a': {'end_topology': a}, 'b': {'end_topology': b}}, [{70}, {72}])
        self.assertEqual(result['restore_old']['weight_updates'], [[0, .9, .5]])
        self.assertEqual(result['block_terminal']['blocked_edges'], [3])
        self.assertEqual(result['block_nonterminal']['blocked_edges'], [4])
        self.assertEqual(result['restore_and_block']['blocked_edges'], [3, 4])
        self.assertEqual(result['restore_and_block']['published_topology'][0][3], .5)
        self.assertFalse(result['restore_and_block']['published_topology'][3][5])
        self.assertEqual(result['intact']['published_topology'], b)
        self.assertEqual(b[0][3], .9)

    def test_factorial_contrasts_are_simple_count_differences(self):
        values = dict(intact=10, restore_old=7, block_new=6, restore_and_block=5,
                      block_terminal=8, block_nonterminal=9)
        self.assertEqual(contrasts(values), dict(restore_with_new=3, restore_without_new=1,
                         block_with_b_weights=4, block_with_a_weights=2, interaction=2,
                         block_terminal=2, block_nonterminal=1))
