import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from analysis.learning_history_followup import summarize_synapse


class FollowupArithmeticTests(unittest.TestCase):
    def history(self, integral=-0.25, factor=1., requested=-0.25, actual=-0.25):
        return {
            "selection": {"synapses": [52]},
            "synapse_summary": [{"synapse_id": 52, "weight_events": 1, "weight_before": 1., "weight_after": 1.+actual}],
            "events": [{"id": 0, "time_us": 10, "synapse_id": 52, "phase": "training", "stream": "learning", "mechanism": "three_factor",
                        "data": {"actual_delta": actual, "requested_delta": requested, "tag_integral_seconds": integral, "held_learning_signal": factor}}],
            "delayed_windows": [{"synapse_id": 52, "positive_tag_integral": 1., "negative_tag_integral": -3.,
                                 "positive_requested": 0.1, "negative_requested": -0.9}],
        }

    def test_negative_tag_with_positive_factor_is_not_a_factor_sign_reversal(self):
        history = self.history()
        original = copy.deepcopy(history)
        result = summarize_synapse(history, 52)
        self.assertEqual(result["published_delayed_sign_cases"], {"tag_negative__factor_positive__request_negative": 1})
        self.assertEqual(result["negative_tag_integral_share"], .75)
        self.assertEqual(result["negative_requested_share"], .9)
        self.assertEqual(history, original)

    def test_positive_tag_and_negative_factor_are_reported_as_a_different_case(self):
        result = summarize_synapse(self.history(integral=.25, factor=-1.), 52)
        self.assertEqual(result["published_delayed_sign_cases"], {"tag_positive__factor_negative__request_negative": 1})

    def test_inconsistent_product_or_missing_changes_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "product differs"):
            summarize_synapse(self.history(requested=.25), 52)
        history = self.history()
        history["synapse_summary"][0]["weight_events"] = 2
        with self.assertRaisesRegex(ValueError, "omitted weight"):
            summarize_synapse(history, 52)


if __name__ == "__main__":
    unittest.main()
