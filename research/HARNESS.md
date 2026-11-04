# Gemeinsamer Forschungs-Harness

Stand **04.11.2026**. Der Harness ergänzt den bestehenden Research-Runner um
gemeinsame Versuchsplanung, fünf Analyseverfahren, ein Verhaltensarchiv und
Forschungsgedächtnis. Alle neuronalen Versuche laufen weiterhin im vorhandenen
Rust-Adapter `m0_selection`. Die Python-Schicht verwendet ausschließlich die
Standardbibliothek. NERVAs neuronale Regeln werden nicht geändert.

## Start

Vom Projektverzeichnis `DSVLM`:

```bash
# Vollständiges Protokoll prüfen und speichern, ohne neuronale Simulationen:
./NERVA_Modell/research/run.sh research-plan NERVA_Modell/research/config/research_smoke.toml

# Alle fünf Verfahren und getrennte Abschlussvalidierung ausführen/fortsetzen:
./NERVA_Modell/research/run.sh research NERVA_Modell/research/config/research_smoke.toml

# Vorhandenen vollständigen Lauf verifizieren und Katalog/Karten neu erzeugen:
./NERVA_Modell/research/run.sh research-report NERVA_Modell/research/config/research_smoke.toml

# Größeres Einstiegsprotokoll: Morris und 4×4-Parameterkarte:
./NERVA_Modell/research/run.sh research NERVA_Modell/research/config/research_screening.toml
```

Ohne `--output` liegt das Archiv unter `research/results/<research.name>/`.
`--output /tmp/nerva-forschung` wählt einen anderen Ordner. Ein geänderter
Programmstand oder ein anderes Protokoll benötigt einen neuen Ordner.
`--binary <pfad>` verwendet eine vorhandene Research-Binärdatei.

Die bisherigen Befehle `plan/run/report` und `select-plan/select/selection-report`
bleiben eigenständig nutzbar. Ihre Auswahllogik wird nicht umgeschaltet.

## Gemeinsame Schnittstelle

```python
propose(context: ExperimentContext) -> ExperimentPlan
analyze(results: list) -> AnalysisReport
```

`contracts.py` definiert diese Verträge. Der Kontext enthält Basisparameter,
Designseed und ausschließlich Explorationsseeds. Ein Plan enthält vollständige
Parameterpunkte und Designmetadaten, etwa Morris-Kanten. Der Scheduler validiert
jeden Punkt über den nativen Runner vor dem ersten neuronalen Lauf. Dieselbe
effektive Konfiguration wird studienübergreifend wiederverwendet; explizite
Koordinaten der jeweiligen Studie bleiben bei der Analyse erhalten.

`planning/backend.py` verwendet die vorhandene `NativeBackend`-Studienausführung
und ergänzt verifizierte Messwerte je Seed/Kontrollgruppe sowie SHA-256 jedes
Roh-Records. Replayprüfung, Hard Gates und Bestätigungsentscheidung bleiben im
nativen Runner beziehungsweise im bestehenden unabhängigen
`evolution/evaluator.py`; die Analyseverfahren können sie nicht lockern.

| Teil | Aufgabe |
|---|---|
| `analysis/sensitivity.py` | Morris-Screening |
| `analysis/phase_mapping.py` | Zweidimensionale Zustandskarten |
| `analysis/causal.py` | Gepaarte 2×2-Mechanismeninterventionen |
| `analysis/robustness.py` | Gepaarte Parameter-Stresstests |
| `analysis/quality_diversity.py` | MAP-Elites auf einem endlichen Parameterraum |
| `planning/experiment_designer.py` | Morris-Trajektorien und faktorielle Designs |
| `planning/scheduler.py` | Prioritäten, Budgetreservierung und native Vorabvalidierung |
| `harness.py` | Ausführung, gemeinsame Ergebnisse, Einfrieren und Bestätigung |
| `research_memory/knowledge.py` | Evidenzverweise, SQLite und regelbasierte Folgeversuche |

## Konfiguration und Budget

Die [kleine vollständige Konfiguration](config/research_smoke.toml) zeigt alle
Verfahren; [research_screening.toml](config/research_screening.toml) konzentriert
sich auf die ersten beiden Prioritäten.

`[research]` enthält Namen, `training_seeds`, `validation_seeds`, `design_seed`
und `max_trials`. Beide Seedmengen müssen eindeutig und disjunkt sein. `[base]`
setzt gemeinsame Parameter einschließlich `retention_delay_us` (Default zwei
Sekunden). `[requirements]` und `[validation]` verwenden dieselben Grenzen und
Prüfungen wie die [externe Selektion](SELECTION.md).

Jede `[[studies]]` hat eine eindeutige `id`, eine `method`, optional `priority`
und verfahrensspezifische `options`. Höhere Priorität kommt zuerst, bei gleicher
Priorität entscheidet die ID. Ein vollständiges Design wird entweder eingeplant
oder mit Begründung zurückgestellt. Ein Morris-Pfad wird nicht mitten in einer
Trajektorie abgeschnitten.

`max_trials` zählt Studienjobs einschließlich aller vier Kontrollen. Vor der
Exploration werden die Kosten von Finalist und Baseline auf den
Bestätigungsseeds reserviert. Ein Studienjob enthält zwei Endpunkte mit
vollständiger Wiederholung, also vier native Trainingsdurchläufe; jeder davon
enthält zusätzlich die bestehenden internen eingefrorenen Proben. Beide
Budgetzahlen stehen im Manifest. Auch die Baseline gehört zum Budget.

Gemeinsame Konfigurationen sparen Jobs. MAP-Elites reserviert konservativ bis
zu `max_evaluations` zusätzliche Konfigurationen. Da seine spätere Auswahl vom
Archiv abhängt, rechnet der Scheduler deren mögliche Wiederverwendung in
nachfolgenden Studien nicht vorab als Ersparnis an. Der tatsächliche Aufwand
kann deshalb unter dem geplanten Maximum liegen. Ungültige Punkte werden auch
in budgetbedingt zurückgestellten Designs abgewiesen.

## Verfahren und Interpretation

**Morris:** `axes` enthalten numerische `min/max`-Grenzen, `trajectories`
liegt zwischen 2 und 64, `levels` ist gerade und liegt zwischen 4 und 20.
Jede Trajektorie verändert jeden Parameter genau einmal; die normierte
Schrittweite beträgt `p/(2*(p-1))`. Der Nenner der Auswertung berücksichtigt
die tatsächlich gespeicherten f32-Parameter. Gemeinsame Seeds reduzieren
Unterschiede durch unterschiedliche Ausgangsnetze. Ausgewertet werden
`mu`, `mu_star` und die Stichprobenstreuung `sigma` über Trajektorien der
seedgemittelten Zielmetrik (`metric`). `sigma` trennt Interaktionen und
Nichtlinearität nicht. Es gibt keine Sobol-Indizes, optimierte Trajektorien
oder Konfidenzintervalle. Kontinuierliche Achsen müssen vom nativen Adapter
als Fließkommaparameter akzeptiert werden.
Methodenreferenz: [SALib Morris-Dokumentation](https://salib.readthedocs.io/en/stable/api.html).

**Phasenkarte:** genau zwei `axes` mit diskreten `values`. Jeder Seed erhält
einen Zustand: `collapse` bei vollständiger Stille bzw. Null-Proberate,
`overactive` bei persistent aktiven Zellen bzw. einer Rate über `max_rate_hz`,
sonst `unstable` bei verletztem Stabilitätskriterium und andernfalls `stable`.
Unterschiedliche Seedzustände ergeben `mixed`. Karten enthalten Scores, Raten
und Gate-Status. SVG und CSV zeigen gemessene Punkte ohne Interpolation.
Die Farben sind operationalisierte Zustandsklassen, kein Nachweis
physikalischer Phasen oder genauer Übergangsgrenzen.

**Kausalvergleich:** vier frisch konstruierte Konfigurationen für
STDP an/aus × Homöostase an/aus mit identischen Seeds. Der Bericht enthält
gepaarte Effekte beider Mechanismen unter beiden Bedingungen und die
Differenz der Differenzen als Interaktion. Die Schalter sind die vorhandenen
Runtime-Schalter. Erfasst werden totale Interventionseffekte einschließlich
nachgelagerter Veränderungen. Die deskriptiven Kontraste sind kein
Bestätigungsnachweis eines isolierten biologischen Mechanismus.

**Robustheit:** ein Basislauf und einzelne Änderungen an den konfigurierten
`axes.values`, jeweils mit denselben Seeds. Standardmetrik ist
`retained_sequence_score`. Berichtet werden seedgepaarte Veränderungen und
Hard Gates. Es sind Parameter-Stresstests auf frischen Netzwerken, keine
Störungen während einer bereits trainierten Episode. Beispielsweise können
sensorische Gewichte, Zeitparameter oder die eingefrorene Retentionsdauer
variiert werden. Ein vom nativen Adapter nicht messbarer Versuch, etwa ohne
auslösenden Cue-Spike, stoppt als Ausführungsfehler; er wird nicht in einen
erfundenen Messwert umgewandelt.

**MAP-Elites:** `axes.values` definieren einen endlichen, vollständig
vorvalidierten Raum mit höchstens 4.096 Punkten. `max_evaluations` begrenzt
besuchte Punkte. Zwei `descriptors` mit `metric/min/max/bins` bestimmen die
Verhaltenszellen; Defaults sind mittlere Proberate und verzögerter Score.
Nur Kandidaten mit bestandenen Mindestanforderungen dürfen Zellen besetzen.
Pro Zelle gewinnt höhere kontrolladjustierte Lernqualität, dann geringerer
Ereignisaufwand, dann die stabile Kandidaten-ID. Werte außerhalb der Grenzen
werden mit Grund ausgeschlossen; die obere Grenze gehört zur letzten Zelle.
Ein gleichverteilt gewählter Archiv-Elternteil erzeugt einen unbesuchten
Nachbarn mit genau einer veränderten Parameterachse. Ohne Archiv oder freie
Nachbarn wird aus den übrigen Punkten gezogen. Es werden keine trainierten
Gewichte vererbt. Referenz: [Mouret und Clune, 2015](https://arxiv.org/abs/1504.04909).

## Exploration, Bestätigung und Forschungsgedächtnis

Auswahl und Analyse sehen ausschließlich `training_seeds`. Vor dem ersten
Zugriff auf Bestätigungsergebnisse wird `frozen-finalist.json` unveränderlich
geschrieben. Ein geeigneter Finalist wird nach maximaler Trainingsqualität,
minimalem Aufwand und stabiler ID festgelegt. Danach prüft die bestehende
Bestätigungsregel ihn einmal gegen die Baseline auf `validation_seeds`.
Ist kein Kandidat geeignet, wird keine Bestätigung ausgeführt. Negative
wissenschaftliche Ergebnisse beenden einen technisch erfolgreichen Lauf regulär.

Diese Abschlussprüfung untersucht **nur den Qualitätsgewinn des Finalisten**.
Sie bestätigt weder Morris-Rangfolgen noch Zustandsgrenzen oder kausale
Wechselwirkungen. Dafür müssen nach der Exploration konkrete Kontraste und
Erfolgskriterien mit neuen, bisher unbenutzten Seeds vorab festgelegt werden.
Über mehrere Protokolle hinweg müssen solche Seedmengen gesondert verwaltet
werden; der Harness prüft deren Trennung innerhalb eines Protokolls.

`knowledge.json` speichert konkrete deskriptive Befunde mit Evidenzpfaden als
`unresolved`. Nur eine bestandene Abschlussprüfung setzt die eng begrenzte
Qualitätsgewinnhypothese auf `supported`. Eine fehlgeschlagene Bestätigung
bedeutet nicht automatisch Widerlegung. Die Speicherschicht speichert keine
frei erfundenen Bestätigungen aus explorativen Vergleichen.

Der erste Forschungscontroller ist regelbasiert: Er priorisiert konfigurierte
Studien, steuert die archivabhängige MAP-Elites-Suche und erzeugt
`next-experiments.json`. Die Vorschläge nutzen Morris-Rangfolgen, unterschiedliche
Zustandsklassen und besetzte Verhaltenszellen. Sie dienen als Eingang für das
nächste explizit konfigurierte Protokoll und werden nicht selbst ausgeführt.
Es gibt keinen Sprachmodell-Agenten oder selbstständig generierte neuronale
Aufgaben. S1/S2-Mutationsraten des bisherigen Selektionsbefehls bleiben fest.

Interferenz, neue Aufgaben, injizierte Störungen, Gedächtnisverlust bei
fortlaufender Plastizität und zusätzliche Zeitmuster-Metriken benötigen weitere
Szenarioadapter beziehungsweise echte Messendpunkte. Die vorhandene eingefrorene
Retentionsprobe wird nicht als Ersatz für diese Fähigkeiten ausgegeben.

## Archiv und Wiederaufnahme

| Pfad | Inhalt |
|---|---|
| `harness-manifest.json` | Protokoll, vorvalidierte Designs, Budgetentscheidungen, native/Python-Quellprüfsummen |
| `source.toml` | Erstes TOML-Original |
| `training/<id>/` | Unveränderte native Studien inklusive JSON/CSV/SQLite und Replaynachweisen |
| `evaluations/<id>.json` | Gates, Messwerte je Seed/Kontrolle und SHA-256 der Roh-Records |
| `reports/<study>.json` | Methodenbefund, Grenzen, Reihenfolge und Evidenzverweise |
| `maps/<study>.svg`, `.csv` | Gemessene zweidimensionale Zustandskarten |
| `frozen-finalist.json` | Festgelegter Kandidat vor Bestätigungszugriff |
| `validation/`, `validation-evaluations/` | Native Bestätigungsstudien und verifizierte Bewertungen |
| `final-result.json`, `research-summary.md` | Eng begrenzte Abschlussentscheidung und Übersicht |
| `knowledge.json`, `next-experiments.json` | Forschungsbefunde und regelbasierte Vorschläge |
| `research.sqlite` | Nach vollständigem Lauf atomar erzeugter Forschungskatalog |

JSON-Protokolle verwenden die vorhandenen SHA-256-Hüllen und unveränderlichen
Schreibprüfungen. Eine Betriebssystemsperre schützt den gemeinsamen Ausgabeordner.
Unterbrochene Läufe rekonstruieren ihre deterministischen Designs und setzen
fehlende native Jobs fort. Die MAP-Elites-Auswahl wird dabei aus denselben
verifizierten Explorationsergebnissen wiederhergestellt. Vollständige Studien
werden vor Wiederverwendung nochmals vom nativen Runner geprüft.
`research-report` verlangt einen abgeschlossenen Lauf und führt keine neuen
neuronalen Trials aus. Nach einem Abbruch sind bereits gespeicherte
Einzelstudien und Berichte lesbar; der Gesamtkatalog entsteht erst am Ende.

Beispiel einer Abfrage über Python `sqlite3` oder ein SQLite-Werkzeug:

```sql
SELECT id, status, claim, evidence FROM hypotheses;
SELECT study, method FROM reports;
SELECT id, json_extract(result, '$.evaluation.eligible') FROM candidates;
```

## Verifikation

Der tatsächlich ausgeführte kleine Beispiellauf vom **04.11.2026** umfasst
18 unterschiedliche Trainingskandidaten und alle fünf Analyseverfahren.
144 Trainingsjobs und 16 Bestätigungsjobs ergeben 160 Studienjobs bzw.
640 native Trainingsdurchläufe inklusive Endpunkten und vollständiger
Wiederholung. Das vorab geplante Maximum liegt bei 168 Studienjobs.
Finalist und Baseline erreichen auf den beiden Bestätigungsseeds jeweils
Qualität 3,0; die Differenz beträgt null und
`held_out_quality_improved = false`. Dieses kleine technische Protokoll
liefert keinen Verbesserungsnachweis. Die anschließende erneute Auswertung
verwendet ausschließlich gespeicherte Trials. Die lokalen Ergebnisse stehen
unter `results/m0_research_smoke/`, einschließlich `research-summary.md`,
`maps/states.svg` und `research.sqlite`.

```bash
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/research/tests -p 'test_*.py' -v
cargo test --offline --manifest-path NERVA_Modell/research/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/research/Cargo.toml --all-targets -- -D warnings
```

Die neuen Tests prüfen bekannte lineare Morris-Effekte, kontrollierte
Interaktionen, Seedunterschiede in Zustandskarten, MAP-Elites-Elternwahl und
Gates, Budgetreservierung, Eingabeprüfung vor Versuchsbeginn, Ausgabe-Locks,
getrennte Bestätigung, Unabhängigkeit der Exploration von Bestätigungsergebnissen,
Wiederaufnahme und manipulierte Berichte/Rohdaten. Der native End-to-End-Test
führt alle fünf Methoden auf echten M0-Versuchen aus.
