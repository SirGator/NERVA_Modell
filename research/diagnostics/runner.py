"""Budgeted, immutable diagnostic studies sharing the research contracts."""
from dataclasses import asdict
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
from contracts import ExperimentContext
from diagnostics import archive
from analysis.diagnostic_decoder import DiagnosticDecoder
from analysis.bottleneck_diagnostics import classify
from evolution import history
from evolution.mutation import canonical_json
from evolution.spec import require
from planning.diagnostic_controller import portfolio, advance
import harness


def prepare(spec):
    context = ExperimentContext(spec.name, {}, tuple(spec.training))
    method = DiagnosticDecoder(spec)
    plan = method.propose(context)
    root = Path(__file__).resolve().parents[1]
    sources = [root/'run.py', root/'harness.py', root/'contracts.py',
               root/'analysis/diagnostic_decoder.py', root/'analysis/bottleneck_diagnostics.py',
               root/'planning/diagnostic_controller.py', root/'evolution/history.py',
               root/'evolution/evaluator.py', root/'evolution/mutation.py', root/'evolution/spec.py',
               root/'planning/experiment_designer.py', *sorted((root/'diagnostics').glob('*.py'))]
    manifest = dict(protocol='NERVA-Diagnosis-v1', specification=spec.source, archive=str(spec.archive),
        plan=asdict(plan), source_hashes={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
        archive_index_sha256=hashlib.sha256((spec.archive/'raw-sha256.csv').read_bytes()).hexdigest(),
        python=sys.version, maximum_records=32*(len(spec.training)+len(spec.test)),
        maximum_frozen_models=20, new_neural_trials=0, input_scope='Previously examined M9-v2 archive, retrospective pilot')
    # Parse and validate every selected probe before output/model fitting.
    rows = archive.load(spec, plan.points)
    return manifest, method, rows


def plan(spec, root, source):
    manifest, _, _ = prepare(spec)
    with harness.output(root, manifest):
        if not (root/'source.toml').exists():
            history.atomic(root/'source.toml', source.encode())
    return manifest


def run(spec, root, source, report_only=False):
    manifest, method, rows = prepare(spec)
    with harness.output(root, manifest, report_only):
        if report_only:
            require((root/'diagnosis-report.json').exists(), 'diagnosis-report requires a completed diagnostic study')
        if not (root/'source.toml').exists():
            history.atomic(root/'source.toml', source.encode())
        if report_only:
            frozen = history.read(root/'frozen-decoders.json')
            require(frozen['training_seeds'] == spec.training, 'frozen decoder split differs')
            method.models = frozen['models']
        else:
            models = method.fit_models(rows)
            # Immutable before the first test accuracy is computed.
            history.immutable(root/'frozen-decoders.json', dict(training_seeds=spec.training,
                training_stage=archive.STAGES[0], test_labels_used_for_fitting=False, models=models))
        report = asdict(method.analyze(rows))
        diagnosis = asdict(classify(report['findings']['observations']))
        informative = report['findings']['observations']['core_action_information'] is True
        signal = report['findings']['observations']['learned_effect_information'] is True
        decision = advance('pilot', True, informative, signal, False)
        result = dict(protocol='NERVA-Diagnosis-v1', report=report, bottlenecks=diagnosis,
            controller=decision, research_tracks=portfolio(), original_model_modified=False,
            new_neural_trials=0, independent_confirmation=False)
        history.immutable(root/'diagnosis-report.json', result)
        history.immutable(root/'next-experiments.json', diagnosis['findings']['hypotheses'])
        catalog(root, manifest, result)
        lines = ['# '+spec.name, '',
            f'Retrospektiver Decoder-Pilot: {len(spec.training)} Trainingsseeds, {len(spec.test)} vollständig getrennte Testseeds; keine neuen neuronalen Simulationen.', '',
            'Skalierung und lineare Decoder sind vor der Testauswertung eingefroren. Beide Wirkungsmappings sind in beiden Partitionen vertreten.', '',
            '| Merkmale | Arm | Ziel | Training | Test sofort | Test nach Ruhe |', '|---|---|---|---:|---:|---:|']
        for score in report['findings']['scores']:
            lines.append(f"| {score['features']} | {score['arm']} | {score['target']} | {score['training']['mean']:.1%} | "
                         f"{score['test'][archive.STAGES[0]]['mean']:.1%} | {score['test'][archive.STAGES[1]]['mean']:.1%} |")
        lines += ['', '## Hinweise für die Diagnose', '']
        for stage, evidence in diagnosis['findings']['stages'].items():
            lines.append(f"- {stage}: `{evidence['status']}`")
        lines += ['', '## Gezielte Folgeversuche', '']
        for hypothesis in diagnosis['findings']['hypotheses']:
            lines.append(f"- `{hypothesis['id']}` → `{hypothesis['next_test']}`")
        lines += ['', '## Aussagegrenzen', '']+['- '+s for s in report['limitations']]
        lines += ['', 'Ein negativer linearer Decoderbefund beweist keine Informationsabwesenheit. Eine positive Decodierung ohne Vorteil gegenüber untrainierten und entkoppelten Kontrollen ist kein lernbedingter Zusatzgewinn.', '',
            'Modelle: frozen-decoders.json. Vollständige gepaarte Kennzahlen: diagnosis-report.json. Quellenprüfsummen und Partitionen: harness-manifest.json.', '']
        history.atomic(root/'diagnosis-summary.md', '\n'.join(lines).encode())
        stream = io.StringIO(); writer = csv.writer(stream)
        writer.writerow(['features','arm','target','training_accuracy','test_accuracy','retained_test_accuracy'])
        for s in report['findings']['scores']:
            writer.writerow([s['features'],s['arm'],s['target'],s['training']['mean'],
                             s['test'][archive.STAGES[0]]['mean'],s['test'][archive.STAGES[1]]['mean']])
        history.atomic(root/'decoder-scores.csv',stream.getvalue().encode())
        return result


def catalog(root, manifest, result):
    descriptor, name = tempfile.mkstemp(prefix='diagnosis-', suffix='.sqlite', dir=root)
    os.close(descriptor); temporary = Path(name)
    try:
        with sqlite3.connect(temporary) as db:
            db.executescript('''CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
                CREATE TABLE decoders(id TEXT PRIMARY KEY,metrics TEXT NOT NULL);
                CREATE TABLE bottlenecks(stage TEXT PRIMARY KEY,evidence TEXT NOT NULL);
                CREATE TABLE hypotheses(id TEXT PRIMARY KEY,status TEXT NOT NULL,next_test TEXT NOT NULL);''')
            db.execute('INSERT INTO metadata VALUES (?,?)',('manifest',canonical_json(manifest)))
            db.execute('INSERT INTO metadata VALUES (?,?)',('controller',canonical_json(result['controller'])))
            db.executemany('INSERT INTO decoders VALUES (?,?)',[(r['id'],canonical_json(r)) for r in result['report']['findings']['scores']])
            db.executemany('INSERT INTO bottlenecks VALUES (?,?)',[(k,canonical_json(v)) for k,v in result['bottlenecks']['findings']['stages'].items()])
            db.executemany('INSERT INTO hypotheses VALUES (?,?,?)',[(h['id'],'suspected',h['next_test']) for h in result['bottlenecks']['findings']['hypotheses']])
        os.replace(temporary,root/'diagnosis.sqlite')
    finally:
        temporary.unlink(missing_ok=True)
