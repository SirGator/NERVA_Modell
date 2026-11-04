"""Research controller: shared execution, independent verification, fresh holdout."""
import contextlib
import csv
from dataclasses import asdict
import fcntl
import html
import io
from evolution import evaluator, history
from evolution.spec import require
from planning.scheduler import prepare
from research_memory import knowledge


@contextlib.contextmanager
def output(root, manifest, report_only=False):
    require(not report_only or (root/"harness-manifest.json").exists(), "research-report requires an existing run")
    root.mkdir(parents=True, exist_ok=True)
    with (root/".harness.lock").open("a+") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError("research output is locked") from None
        path = root/"harness-manifest.json"
        if not path.exists():
            require(not any(p.name != ".harness.lock" for p in root.iterdir()), "nonempty research output has no matching manifest")
        history.immutable(path, manifest)
        yield


def plan(spec, root, backend, source_text):
    manifest, _ = prepare(spec, backend)
    with output(root, manifest):
        source = root/"source.toml"
        if not source.exists():
            history.atomic(source, source_text.encode())
    return manifest


def run(spec, root, backend, source_text, report_only=False):
    manifest, methods = prepare(spec, backend)
    with output(root, manifest, report_only):
        if report_only:
            require((root/"final-result.json").exists() and (root/"frozen-finalist.json").exists(),
                    "research-report requires a completed run")
        if not (root/"source.toml").exists():
            history.atomic(root/"source.toml", source_text.encode())
        results, reports = {}, {}

        def evaluate(candidate):
            cid = candidate["id"]
            if cid not in results:
                cost = (len(results)+1)*len(spec.training)*4+manifest["reserved_validation_trials"]
                require(cost <= spec.max_trials, "research trial budget exhausted")
                result = backend.evaluate(candidate, spec.training, root/"training"/cid,
                                          "train", spec.requirements, report_only)
                history.immutable(root/"evaluations"/f"{cid}.json", result)
                results[cid] = result
            # A shared native configuration can have distinct explicit parameter
            # representations (e.g. omitted defaults). Keep design coordinates.
            return dict(results[cid], parameters=candidate["parameters"])

        baseline = manifest["baseline"]
        evaluate(baseline)
        for study in manifest["studies"]:
            if not study["scheduled"]:
                continue
            method = methods[study["id"]]
            observations, order = [], []
            if study["plan"]["method"] == "quality_diversity":
                remaining = set(range(len(study["candidates"])))
                for _ in range(method.limit):
                    index = method.next_index(observations, remaining)
                    remaining.remove(index)
                    order.append(index)
                    observations.append(evaluate(study["candidates"][index]))
            else:
                for index, candidate in enumerate(study["candidates"]):
                    order.append(index)
                    observations.append(evaluate(candidate))
            report = asdict(method.analyze(observations))
            report.update(study=study["id"], evaluation_order=order,
                          evidence=[r["id"] for r in observations], seeds=list(spec.training))
            history.immutable(root/"reports"/f"{study['id']}.json", report)
            reports[study["id"]] = report
            if report["method"] == "phase_mapping":
                write_map(root, study["id"], report)
            print(f"RESEARCH {study['id']} method={report['method']} points={len(observations)}", flush=True)

        eligible = [r for r in results.values() if r["evaluation"]["eligible"]]
        winner = min(eligible, key=lambda r: (-r["evaluation"]["quality"], r["evaluation"]["resource"], r["id"])) if eligible else None
        frozen = dict(winner=winner["id"] if winner else None,
                      parameters=winner["parameters"] if winner else None,
                      training_only=True, training_candidates=sorted(results),
                      policy="Maximum gated training quality, then minimum event cost, then stable ID")
        # Neither held-out results nor their derived memory are read before this.
        history.immutable(root/"frozen-finalist.json", frozen)
        final = dict(protocol="NERVA-Harness-v1", held_out_quality_improved=False,
                     validation_status="not_run_no_eligible_candidate", frozen=frozen,
                     training_candidates=len(results), validation_used_for_selection=False,
                     original_model_modified=False)
        if winner:
            by_id = {}
            for role, candidate in [("candidate", winner), ("baseline", baseline)]:
                cid = candidate["id"]
                if cid not in by_id:
                    result = backend.evaluate(candidate, spec.validation, root/"validation"/cid,
                                              "validate", spec.requirements, report_only)
                    history.immutable(root/"validation-evaluations"/f"{cid}.json", result)
                    by_id[cid] = result
                final[role] = dict(id=cid, evaluation=by_id[cid]["evaluation"])
            decision = evaluator.validate(final["candidate"]["evaluation"], final["baseline"]["evaluation"], spec.validation_policy)
            final.update(validation_status="completed", decision=decision,
                         held_out_quality_improved=decision["held_out_quality_improved"])
        history.immutable(root/"final-result.json", final)
        entries = knowledge.hypotheses(reports, final)
        suggestions = knowledge.followups(reports)
        history.immutable(root/"knowledge.json", entries)
        history.immutable(root/"next-experiments.json", suggestions)
        knowledge.catalog(root, manifest, results, reports, entries, final)
        lines = ["# "+spec.name, "", f"{len(results)} Trainingskandidaten; {len(reports)} abgeschlossene Analysen.", "",
                 "| Studie | Methode | Status |", "|---|---|---|"]
        for study in manifest["studies"]:
            lines.append(f"| {study['id']} | {study['plan']['method']} | {'explorativ' if study['scheduled'] else 'Budget: zurückgestellt'} |")
        lines += ["", f"Abschlussvalidierung: {final['validation_status']}; Qualitätsverbesserung: {final['held_out_quality_improved']}.", "",
                  "Ein eingefrorener Finalist gegen Baseline auf getrennten Seeds derselben M0-Aufgabe. Die Abschlussvalidierung bestätigt keine Morris-Rangfolge, Phasengrenze oder Mechanismushypothese.", "",
                  "Rohdaten und Einzelkataloge: training/ und validation/. Analysen: reports/. Verhaltenskarten: maps/. Abfragbare Forschungshistorie: research.sqlite.", "",
                  "next-experiments.json enthält regelbasierte Vorschläge. Sie werden nicht automatisch ausgeführt; weitere Bestätigungen benötigen neue, unbenutzte Seeds.", ""]
        history.atomic(root/"research-summary.md", "\n".join(lines).encode())
        return final


def write_map(root, study, report):
    """Portable SVG and CSV from measured grid cells, without interpolation."""
    findings = report["findings"]
    axes, cells = findings["axes"], findings["cells"]
    stream = io.StringIO()
    writer = csv.writer(stream)
    writer.writerow([axes[0]["name"], axes[1]["name"], "state", "sequence_score", "firing_rate", "eligible", "candidate"])
    for c in cells:
        writer.writerow([*c["coordinates"], c["state"], c["sequence_score"], c["firing_rate"], c["eligible"], c["candidate"]])
    folder = root/"maps"
    folder.mkdir(exist_ok=True)
    history.atomic(folder/f"{study}.csv", stream.getvalue().encode())
    palette = dict(stable="#6abf91", collapse="#8e9aaf", overactive="#e76f51", unstable="#e9c46a", mixed="#a78bce")
    width, height = max(520, 140+90*len(axes[0]["values"])), 170+55*len(axes[1]["values"])
    escape = lambda value: html.escape(str(value), quote=True)
    svg = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" role="img" aria-label="{escape(study)}">',
           '<rect width="100%" height="100%" fill="white"/>',
           f'<g font-family="sans-serif" font-size="12"><text x="15" y="20">{escape(study)}: sampled M0 states</text>',
           f'<text x="120" y="42">{escape(axes[0]["name"])}</text>',
           f'<text x="10" y="62">{escape(axes[1]["name"])}</text>']
    for i, value in enumerate(axes[0]["values"]):
        svg.append(f'<text x="{125+90*i}" y="84">{escape(value)}</text>')
    for j, value in enumerate(axes[1]["values"]):
        svg.append(f'<text x="10" y="{124+55*j}">{escape(value)}</text>')
    for c in cells:
        # Native f32 rounding may shift values relative to the TOML coordinates.
        indices = [min(range(len(a["values"])), key=lambda i: abs(a["values"][i]-v)) for a, v in zip(axes, c["coordinates"])]
        x, y = 120+90*indices[0], 100+55*indices[1]
        svg += [f'<rect x="{x}" y="{y}" width="85" height="50" fill="{palette[c["state"]]}"><title>{escape(c)}</title></rect>',
                f'<text x="{x+4}" y="{y+27}">{escape(c["state"])}</text>']
    svg.append('</g></svg>')
    history.atomic(folder/f"{study}.svg", "\n".join(svg).encode())
