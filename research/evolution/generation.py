import json
from pathlib import Path
from . import evaluator, history, mutation, selection
from .population import InvalidCandidate
from .spec import require


def run(spec, root, backend, source_text, report_only=False):
    # Validate the baseline and bind all executable sources before output/training.
    baseline = backend.canonical(spec.base, spec)
    manifest = make_manifest(spec, backend, baseline)
    with history.output(root, manifest, report_only):
        if not (root/"source.toml").exists():history.atomic(root/"source.toml", source_text.encode())
        candidates, evaluations, generations = {}, {}, []
        elites = [baseline]
        stop = "generation_limit"
        for generation in range(spec.generations):
            proposals, population = [], {}
            if generation == 0:
                candidates[baseline["id"]] = baseline
                population[baseline["id"]] = baseline
                proposals.append(dict(parent=None, child=baseline["id"], status="baseline", mutation=None))
            for parent in elites:
                population[parent["id"]] = parent
                proposals.append(dict(parent=parent["id"], child=parent["id"], status="retained_parent", mutation=None))
                children, rejected = mutation.children(parent, spec, generation)
                proposals.extend(dict(parent=parent["id"], **item) for item in rejected)
                for child in children:
                    try:candidate = backend.canonical(child["parameters"], spec)
                    except InvalidCandidate as exc:
                        proposals.append(dict(parent=parent["id"], status="invalid_configuration", mutation=child["mutation"], reason=str(exc)));continue
                    cid = candidate["id"]
                    if cid not in candidates and len(candidates) >= spec.max_candidates:
                        proposals.append(dict(parent=parent["id"], child=cid, status="candidate_budget", mutation=child["mutation"]));continue
                    status = "duplicate_or_known" if cid in candidates else "new_child"
                    if cid in candidates:require(candidate["effective_config"] == candidates[cid]["effective_config"], "candidate identity collision")
                    else:candidates[cid] = candidate
                    population[cid] = candidates[cid]
                    proposals.append(dict(parent=parent["id"], child=cid, status=status, mutation=child["mutation"]))
            for candidate in population.values():
                cid = candidate["id"]
                history.immutable(root/"candidates"/f"{cid}.json", {k:v for k,v in candidate.items() if k != "evaluation"})
                if cid not in evaluations:
                    evaluation = backend.evaluate(candidate, spec.training, root/"training"/cid, "train", spec.requirements, report_only)
                    history.immutable(root/"evaluations"/f"{cid}.json", evaluation)
                    evaluations[cid] = evaluation
                candidate["evaluation"] = evaluations[cid]
                candidates[cid] = candidate
            selected, fronts = selection.select(list(population.values()), spec.elite_size, spec.mutations)
            checkpoint = dict(number=generation, proposals=proposals, population=list(population), elites=selected,
                pareto_fronts=fronts, eligible=sum(c["evaluation"]["eligible"] for c in population.values()),
                unique_candidates_evaluated=len(evaluations), selection_input="training_only")
            history.immutable(root/"generations"/f"generation-{generation:03}.json", checkpoint)
            generations.append(checkpoint); history.catalog(root, candidates, generations)
            print(f"GENERATION {generation+1} eligible={checkpoint['eligible']} elites={len(selected)} evaluated={len(evaluations)}", flush=True)
            elites = [candidates[cid] for cid in selected]
            if not elites:stop = "no_eligible_candidates";break
            if len(candidates) >= spec.max_candidates:stop = "candidate_budget";break
        eligible = [c for c in candidates.values() if c.get("evaluation", {}).get("eligible")]
        winner = min(eligible, key=lambda c: (-c["evaluation"]["quality"], c["evaluation"]["resource"], c["id"])) if eligible else None
        frozen = dict(winner=winner["id"] if winner else None,
                      parameters=winner["parameters"] if winner else None, training_only=True,
                      completed_generations=len(generations), stop_reason=stop)
        # This immutable file must exist before the first validation study is read/run.
        history.immutable(root/"frozen-finalist.json", frozen)
        final = dict(protocol="NERVA-Selection-v1", frozen=frozen, training_candidates=len(evaluations),
            held_out_quality_improved=False, validation_status="not_run_no_eligible_candidate",
            original_model_modified=False, validation_used_for_selection=False)
        if winner:
            by_id = {}
            for role, candidate in [("candidate", winner), ("baseline", baseline)]:
                cid = candidate["id"]
                if cid not in by_id:
                    result = backend.evaluate(candidate, spec.validation, root/"validation"/cid, "validate", spec.requirements, report_only)
                    history.immutable(root/"validation-evaluations"/f"{cid}.json", result)
                    by_id[cid] = result
                final[role] = dict(id=cid, parameters=candidate["parameters"], evaluation=by_id[cid])
            decision = evaluator.validate(final["candidate"]["evaluation"], final["baseline"]["evaluation"], spec.validation_policy)
            final.update(held_out_quality_improved=decision["held_out_quality_improved"], validation_status="completed", decision=decision)
        history.immutable(root/"final-result.json", final)
        history.catalog(root, candidates, generations, final)
        lines = ["# "+spec.name, "", f"Generationen: {len(generations)}. Eindeutige Trainingskandidaten: {len(evaluations)}.",
                 "", "Mindestanforderungen vor Paretoauswahl nach Lernvorteil und deterministischem Ereignisaufwand; Vielfalt nach Parameterabstand.",
                 "", f"Stopgrund: `{stop}`. Validierung: `{final['validation_status']}`.",
                 "", f"Vorteil auf zurückgehaltenen Seeds nach vorab festgelegter Prüfung: **{final['held_out_quality_improved']}**.",
                 "", "Der Finalist wurde vor der Validierung unveränderlich festgelegt. Validierung beeinflusst keine Mutation oder Eliteauswahl.",
                 "", "Gedächtnisprüfung: zusätzliche Ruhe bei eingefrorenen Gewichten. Andere Aufgaben, fortlaufende Plastizität und Interferenz sind nicht geprüft.",
                 "", "Dies ist ein Selektionsprototyp auf M0, kein allgemeiner Lernnachweis für NERVA und kein M9/M11-Ergebnis. Das Originalmodell wird nicht verändert.",
                 "", "[Ergebnis](final-result.json) · [Finalist vor Validierung](frozen-finalist.json) · [SQLite-Verlauf](selection.sqlite)."]
        history.atomic(root/"selection-summary.md", ("\n".join(lines)+"\n").encode())
        return final


def plan(spec, backend):
    baseline = backend.canonical(spec.base, spec)
    # Every mutation axis is explicit and bounded, but cross-axis combinations are
    # also validated natively as they are proposed during the search.
    return dict(protocol="NERVA-Selection-v1", name=spec.name, baseline=baseline, budget=spec.budget(),
                training_seeds=spec.training, validation_seeds=spec.validation,
                requirements=spec.requirements, validation_policy=spec.validation_policy,
                limits="Plan only. Adaptive candidates are generated using training results; validation stays unused until finalist freeze.")

def make_manifest(spec, backend, baseline):
    return dict(protocol="NERVA-Selection-v1", source=spec.source, budget=spec.budget(),
        backend=backend.identity(), baseline=baseline, ranking="Hard requirements, Pareto quality/event-count fronts, genotype diversity; no holdout input",
        validation="Freeze exactly one training-selected finalist before held-out evaluation; no re-selection after validation")
