#!/usr/bin/env python3
"""Read a Learning History record; filter indexed timelines without rerunning NERVA."""
import argparse
import json
from pathlib import Path


def select_history(history, *, neuron=None, synapse=None, phase=None, mechanism=None, branch=None, limit=200):
    if history.get("protocol") != "NERVA-Learning-History/1" or history.get("complete") is not True:
        raise ValueError("a complete NERVA-Learning-History/1 record is required")
    if limit < 1:
        raise ValueError("limit must be positive")
    filtered = any(v is not None for v in (neuron, synapse, phase, mechanism, branch))
    events = history["events"]
    indices = None
    for name, value in (("neuron_index", neuron), ("synapse_index", synapse)):
        if value is not None:
            selected = set(history[name].get(str(value), []))
            indices = selected if indices is None else indices & selected
    candidates = (events[i] for i in sorted(indices)) if indices is not None else iter(events)
    selected = [e for e in candidates if (phase is None or e["phase"] == phase)
                and (mechanism is None or e["mechanism"] == mechanism)
                and (branch is None or e["branch"] == branch)] if filtered else []
    edges = [s for s in history["synapse_summary"]
             if (synapse is None or s["synapse_id"] == synapse)
             and (neuron is None or neuron in (s["source_id"], s["target_id"]))]
    edges.sort(key=lambda s: (-s["requests_without_change"], -s["opposing_update_mass"], s["synapse_id"]))
    windows = [s for s in history["delayed_windows"]
               if filtered and branch in (None, "learned") and phase in (None, "training") and mechanism in (None, "three_factor")
               and (synapse is None or s["synapse_id"] == synapse)
               and (neuron is None or neuron in (s["source_id"], s["target_id"]))]
    selection = history["selection"]
    available = ((neuron is None or neuron in selection["neurons"])
                 and (synapse is None or synapse in selection["synapses"]))
    return {"summary": history["summary"], "selection": selection,
            "detail_available_for_selection": available,
            "events": selected[:limit], "matching_events": len(selected),
            "events_truncated": len(selected) > limit,
            "delayed_windows": windows[:limit], "matching_windows": len(windows),
            "windows_truncated": len(windows) > limit,
            "synapses": edges[:limit], "matching_synapses": len(edges),
            "synapses_truncated": len(edges) > limit,
            "ordering": history["ordering"], "limits": history["limits"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("record", type=Path, help="records/<trial>.json from a completed harness run")
    parser.add_argument("--neuron", type=int)
    parser.add_argument("--synapse", type=int)
    parser.add_argument("--phase", choices=["training", "recall", "recall_rest"])
    parser.add_argument("--mechanism")
    parser.add_argument("--branch", choices=["learned", "weight_reset"])
    parser.add_argument("--limit", type=int, default=200)
    args = parser.parse_args()
    try:
        for value in (args.neuron, args.synapse):
            if value is not None and not 0 <= value <= 2**64 - 1:
                raise ValueError("IDs must be u64 integers")
        record = json.loads(args.record.read_text(encoding="utf-8"))["record"]
        result = select_history(record["details"]["history"], neuron=args.neuron, synapse=args.synapse,
                                phase=args.phase, mechanism=args.mechanism, branch=args.branch, limit=args.limit)
        print(json.dumps({"job": record["job"], **result}, ensure_ascii=False, indent=2, allow_nan=False))
    except (OSError, ValueError, KeyError, TypeError, IndexError) as exc:
        parser.exit(1, f"learning-history: {exc}\n")


if __name__ == "__main__":
    main()
