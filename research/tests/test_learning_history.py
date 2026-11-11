import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from learning_history import select_history


class HistoryQueryTests(unittest.TestCase):
    def setUp(self):
        self.history = {
            "protocol": "NERVA-Learning-History/1", "complete": True,
            "summary": {"weight_events": 1}, "selection": {"neurons": [1, 2], "synapses": [8]},
            "events": [
                {"id": 7, "branch": "learned", "time_us": 10, "phase": "training", "mechanism": "stdp_post", "synapse_id": 8},
                {"id": 3, "branch": "weight_reset", "time_us": 20, "phase": "recall", "mechanism": "transmission", "synapse_id": 8},
            ],
            "neuron_index": {"1": [0, 1], "2": [0, 1]}, "synapse_index": {"8": [0, 1]},
            "synapse_summary": [{"synapse_id": 8, "source_id": 1, "target_id": 2,
                                 "requests_without_change": 3, "opposing_update_mass": 0.5}],
            "delayed_windows": [{"synapse_id": 8, "source_id": 1, "target_id": 2}],
            "ordering": "test", "limits": "test",
        }

    def test_index_positions_are_distinct_from_stable_event_ids(self):
        original = copy.deepcopy(self.history)
        result = select_history(self.history, neuron=1, synapse=8, phase="recall", branch="weight_reset")
        self.assertEqual([e["id"] for e in result["events"]], [3])
        self.assertEqual(result["delayed_windows"], [])
        self.assertEqual(self.history, original)

    def test_limits_are_explicit_and_unselected_neurons_have_no_detail(self):
        result = select_history(self.history, synapse=8, limit=1)
        self.assertEqual(result["matching_events"], 2)
        self.assertTrue(result["events_truncated"])
        self.assertEqual(len(result["events"]), 1)
        result = select_history(self.history, neuron=99)
        self.assertFalse(result["detail_available_for_selection"])
        self.assertEqual(result["events"], [])

    def test_aggregate_mode_and_incomplete_records(self):
        result = select_history(self.history)
        self.assertEqual(result["events"], [])
        self.assertEqual(result["synapses"][0]["synapse_id"], 8)
        self.history["complete"] = False
        with self.assertRaises(ValueError):
            select_history(self.history)


if __name__ == "__main__":
    unittest.main()
