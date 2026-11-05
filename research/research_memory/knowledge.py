"""Derived SQLite knowledge, never a substitute for immutable raw evidence."""
import os
import sqlite3
import tempfile
from pathlib import Path
from evolution.mutation import canonical_json


def hypotheses(reports, final, scenario="m0_selection"):
    entries = []
    for study, report in reports.items():
        findings, method = report["findings"], report["method"]
        if method == "sensitivity":
            claim = f"{findings['ranking'][0]} has the largest sampled Morris mu_star for {findings['metric']}"
        elif method == "phase_mapping":
            claim = "Sampled activity states: "+", ".join(sorted({c["state"] for c in findings["cells"]}))
        elif method == "causal":
            claim = f"STDP/homeostasis interaction in {findings['metric']}: paired mean {findings['interaction']['mean']}"
        elif method == "robustness":
            claim = f"All declared stress configurations meet hard requirements: {findings['all_stress_gates_passed']}"
        elif method in ["size_environment","plasticity_diagnostics"]:
            claim = "Measured size/feedback learning gains: "+str([
                dict(size=c["processing_neurons"],parameters=c["parameters"],gains=c["learning_gains"]) for c in findings["cells"]])
        else:
            claim = f"Eligible behavioral archive covers {findings['coverage']:.3f} of declared cells"
        entries.append(dict(id="study:"+study, status="unresolved", claim=claim,
                            evidence="reports/"+study+".json",
                            scope=f"{scenario} exploration; independent confirmation of these findings is pending"))
    if final["validation_status"] == "completed":
        entries.append(dict(id="confirmation:held_out_quality_gain", status="supported" if final["held_out_quality_improved"] else "unresolved",
            claim=f"Frozen finalist improves control-adjusted {scenario} quality over baseline under the declared validation policy",
            evidence="final-result.json", scope="Held-out seeds of the same task; failure to confirm does not refute the claim"))
    return entries


def followups(reports, scenario="m0_selection"):
    suggestions = [
        dict(source='research_strategy',method='diagnostic_decoder',track='A',priority=200,
             config='config/diagnosis_m9.toml',reason='Diagnose the unresolved M9 representation/readout bottleneck before further M0 optimization'),
        dict(source='research_strategy',method='continual_sequences',track='B',priority=190,
             config='config/diagnostic_continual_smoke.toml',reason='Check acquisition and pattern overlap in a small controlled continual-learning assay'),
    ] if scenario == "m0_selection" else []
    for study, report in reports.items():
        findings = report["findings"]
        if report["method"] == "sensitivity":
            suggestions.append(dict(source=study, method="phase_mapping", priority=100,
                reason="Map the highest ranked parameter ranges", parameters=findings["ranking"][:2]))
        elif report["method"] == "phase_mapping":
            if len({c["state"] for c in findings["cells"]}) > 1 or any(c["state"] == "mixed" for c in findings["cells"]):
                suggestions.append(dict(source=study, method="phase_mapping", priority=90,
                    reason="Refine sampled regions with different or seed-dependent state labels"))
        elif report["method"] == "causal":
            suggestions.append(dict(source=study, method="causal", priority=80,
                reason="Preregister one mechanism contrast and confirm it on fresh seeds"))
        elif report["method"] == "quality_diversity":
            suggestions.append(dict(source=study, method="robustness", priority=70,
                reason="Stress-test representatives of occupied behavioral cells",
                candidates=[c["candidate"] for c in findings["cells"]]))
        elif report["method"] in ["size_environment","plasticity_diagnostics"]:
            suggestions.append(dict(source=study, method=report["method"], priority=100,
                reason="Inspect activity/readout gates, then preregister one paired size or mechanism contrast on fresh seeds"))
    return sorted(suggestions, key=lambda s: (-s["priority"], s["source"]))


def catalog(root, manifest, results, reports, entries, final):
    descriptor, name = tempfile.mkstemp(prefix="research-", suffix=".sqlite", dir=root)
    os.close(descriptor)
    temporary = Path(name)
    try:
        with sqlite3.connect(temporary) as db:
            db.executescript("""
                CREATE TABLE metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE candidates(id TEXT PRIMARY KEY, result TEXT NOT NULL);
                CREATE TABLE reports(study TEXT PRIMARY KEY, method TEXT NOT NULL, report TEXT NOT NULL);
                CREATE TABLE hypotheses(id TEXT PRIMARY KEY, status TEXT NOT NULL, claim TEXT NOT NULL, evidence TEXT NOT NULL, scope TEXT NOT NULL);
            """)
            db.executemany("INSERT INTO metadata VALUES (?,?)", [("manifest", canonical_json(manifest)), ("final", canonical_json(final))])
            db.executemany("INSERT INTO candidates VALUES (?,?)", [(cid, canonical_json(r)) for cid, r in sorted(results.items())])
            db.executemany("INSERT INTO reports VALUES (?,?,?)", [(study, r["method"], canonical_json(r)) for study, r in sorted(reports.items())])
            db.executemany("INSERT INTO hypotheses VALUES (?,?,?,?,?)", [(e["id"], e["status"], e["claim"], e["evidence"], e["scope"]) for e in entries])
        os.replace(temporary, root/"research.sqlite")
    finally:
        temporary.unlink(missing_ok=True)
