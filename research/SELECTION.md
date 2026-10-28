# Externe Selektion: S1/S2-Prototyp

Stand **28.10.2026**. Die Forschungsanwendung erzeugt Konfigurationskopien,
testet sie, verwirft ungeeignete Kandidaten, erhält mehrere Eliten und erzeugt
daraus Nachkommen. Neuronale Läufe bleiben in Rust; die externe Evolution ist
in `research/evolution/` mit Python-Standardbibliotheken implementiert.

Der Suchprozess verändert weder Rust-Quellen noch das laufende Originalmodell.
Jeder Kandidat beginnt mit einem frisch konstruierten Netzwerk. Vererbt werden
Parameter und vorhandene Mechanismusschalter, keine trainierten Gewichte.

## Start

Vom Projektverzeichnis `DSVLM`:

```bash
# Eingaben/Baseline prüfen und Protokoll samt Budget unveränderlich festlegen:
./NERVA_Modell/research/run.sh select-plan NERVA_Modell/research/config/selection_m0.toml

# Suche und einmalige Abschlussvalidierung; unterbrochene Läufe fortsetzen:
./NERVA_Modell/research/run.sh select NERVA_Modell/research/config/selection_m0.toml

# Vorhandenen vollständigen Lauf prüfen und abgeleitete Daten neu erzeugen:
./NERVA_Modell/research/run.sh selection-report NERVA_Modell/research/config/selection_m0.toml

# Kleiner technischer Lauf mit zwei Generationen:
./NERVA_Modell/research/run.sh select NERVA_Modell/research/config/selection_smoke.toml --output /tmp/meine-nerva-selektion
```

Ohne `--output` liegen die Ergebnisse unter `NERVA_Modell/research/results/`
und dem konfigurierten Namen. In Git werden diese generierten Daten ignoriert.
Veränderte Konfigurationen oder Programmstände erfordern einen neuen Ordner.
Ein negativer Validierungsbefund ist ein reguläres Ergebnis, kein CLI-Fehler.

## Was mutiert werden kann

S1 umfasst ausgewählte vorhandene STDP-, Homöostase- und Membranparameter.
S2 ergänzt **STDP an/aus** und **Homöostase an/aus**. Architektur, Testaufgabe,
Seedplan, Erfolgskriterien und Rust-Code sind keine mutierbaren Eigenschaften.

Der konfigurierte Anfangswert jedes mutierbaren Parameters steht ausdrücklich
unter `[base]`. Numeric-Achsen brauchen Grenzen und entweder Multiplikatoren
oder Werte; Integer-Achsen benötigen Integer-Werte. Schalter erhalten boolesche
Werte. Beispiel:

```toml
[selection]
name = "m0_external_selection"
scenario = "m0_selection"
generations = 4
elite_size = 2
children_per_parent = 4
max_candidates = 32
mutation_seed = 42
training_seeds = [7, 19, 41, 73]
validation_seeds = [151, 191, 229, 277, 307, 347]
retention_delay_us = 2000000

[base]
training_repetitions = 24
"learning.a_plus" = 0.04
"learning.a_minus" = 0.045
"mechanisms.stdp" = true
"mechanisms.homeostasis" = false

[[mutations]]
name = "learning.a_plus"
factors = [0.8, 1.2]
min = 0.005
max = 0.12

[[mutations]]
name = "mechanisms.homeostasis"
values = [false, true]
```

Die [vollständige Beispielkonfiguration](config/selection_m0.toml) enthält
auch `a_minus`, den STDP-Schalter und alle Mindestanforderungen.

Aus jedem Elternteil entstehen Einzeländerungen an jeweils einer Achse.
Bei mehr Alternativen als `children_per_parent` begrenzt eine fest spezifizierte
SplitMix64-Auswahl die Nachkommen. Derselbe Suchseed, Elternteil und dieselbe
Generation erzeugen dieselben Vorschläge. Numerische Werte werden auf die
verwendete f32-Präzision normalisiert; Grenzen werden nicht überschritten.

Jeder Vorschlag wird vom nativen Runner validiert. Ungültige Kombinationen,
unveränderte Mutationen, bekannte effektive Konfigurationen und erschöpfte
Budgets werden mit Grund protokolliert. Effektive Konfigurationen erhalten
SHA-256-Identitäten. Bereits bewertete Kandidaten werden über Generationen
hinweg wiederverwendet; bekannte Vorschläge erzeugen zusätzliche Abstammungskanten.

Das Budget begrenzt eindeutige **bewertete** Trainingskandidaten, Generationen,
Eliten und Nachkommen. Der Beispielplan erlaubt höchstens 29 Trainingskandidaten
und 512 Studienjobs einschließlich der maximal zwei Validierungsstudien.
Ein Studienjob enthält unmittelbare/verzögerte Endpunkte und vollständige
Wiederholung: höchstens 2.048 native Trainingsdurchläufe. Jeder native M0-
Trainingsdurchlauf enthält zusätzlich seine beiden eingefrorenen internen Proben.

## Eigener kontrollierter M0-Adapter

Die Selektion verwendet `m0_selection`, nicht die unveränderte feste G1–G4-
Auswertung des bisherigen `m0_sequence`-Adapters:

| Kontrollname | Eingangsfolge | STDP | Homöostase |
|---|---|---|---|
| `none` | Geordnet | Kandidat | Kandidat |
| `disable_stdp` | Dieselbe geordnete Folge | Aus | Kandidat |
| `shuffle_input` | Gemischt, gleiche Häufigkeiten | Kandidat | Kandidat |
| `disable_homeostasis` | Dieselbe geordnete Folge | Kandidat | Aus |

Damit bleibt beispielsweise bei einem Kandidaten mit Homöostase auch die
eingefrorene oder gemischte Kontrolle mit derselben Homöostase ausgestattet.
Mechanismusschalter sind echte Runtime-Einstellungen, keine nur umbenannten
Null-Lernraten. Die zusätzlichen Schalter werden über die Referenzversuchs-API
`M0Experiment::run_controlled` auf Versuchskopien angewendet. Die bisherigen
`run`-/`run_profiled`-Gruppenregeln bleiben exakt erhalten und werden gegen
die neue Schnittstelle getestet. Es gibt keine neue biologische Lernregel.

Jeder Kontrolllauf untersucht eine unmittelbare und eine zusätzlich verzögerte
eingefrorene Probe. Beide stammen aus deterministisch wiederholtem Training
mit derselben Konfiguration und demselben Seed. Jeder Endpunkt wird vollständig
wiederholt und mitsamt nativen Ereignisfolgen verglichen. Im Archiv stehen beide
Konfigurationen, Zustände/Metriken und Ereignis-Digests.

Die Verzögerung prüft **nur Erhalt während zusätzlicher Ruhe bei eingefrorenen
Gewichten**, mit dem vorhandenen Zellzustand. Sie untersucht kein Vergessen bei
fortlaufender Plastizität, keine Interferenz und keine unabhängigen Aufgaben.
Die nativen Gruppenlabels in Rohdetails bezeichnen den zugrunde liegenden
Eingangsgenerator; die tatsächlichen Mechanismen stehen ausdrücklich im
effektiven Konfigurationssatz des `m0_selection`-Jobs.

## Mindestanforderungen vor Bewertung

Voreinstellungen unter `[requirements]`:

| Anforderung | Prüfung |
|---|---|
| Stabilität | Unmittelbare/verzögerte Probe in sämtlichen Kontrollen stabil, keine persistent aktiven Zellen |
| Reproduzierbarkeit | Vollständige native Wiedergabe beider Endpunkte; eingefrorene Gewichte bleiben unverändert |
| Nichtkollaps | Nicht alle Zellen still; endliche Raten |
| Gedächtnis | Verzögerter Kandidaten-Score mindestens 1 bei jedem Seed, kein Scoreverlust gegenüber sofort |
| Ressourcen | Protokollierter Ereigniszähler höchstens 100.000 pro Studienjob, Proberate höchstens 200 Hz |

Abweichende Grenzen sind ausdrücklich konfigurierbar. Nicht bestandene Gates
setzen `eligible = false` und `quality = null`. Ablehnungsgründe werden je
Seed/Kontrolle gespeichert. Ein großer Lernscore hebt diese Anforderungen
nicht auf. Ausführungs- und Integritätsfehler stoppen den Lauf; sie werden
nicht still als schlechte wissenschaftliche Ergebnisse aussortiert.

## Qualität, Aufwand und Vielfalt

Die Lernqualität ist der kleinste mittlere Scorevorteil gegenüber eingefrorener
und gemischter Kontrolle, jeweils sofort und nach Ruhe. Das sind vier
Vergleiche über denselben Seedplan. Diese Zahl beschreibt nur die Lernleistung
innerhalb der M0-Aufgabe; sie ersetzt keine Stabilitäts- oder Gedächtnisprüfung.

Der zweite Vergleichswert ist der mittlere Ereigniszähler des Kandidaten.
Er ist ein **deterministischer Aufwandindikator**, kein vollständiges Hardware-
oder Laufzeitkostenmaß. Er zählt die nativen Trainings-/Probeereignisprotokolle
der zwei Endpunkte; interne Wiederholungen sind nicht als gesamte Rechenkosten
enthalten. Host-Laufzeiten bleiben separat in den nativen Records erhalten
und bestimmen keine Auswahl.

Geeignete Kandidaten werden in Paretofronten nach Qualität und Aufwand
ausgewählt. Innerhalb einer Front beginnen wir mit hoher Qualität und geringem
Aufwand, danach erhält ein großer normierter Parameterabstand zu bereits
ausgewählten Eliten Vorrang. Boolesche Mechanismen zählen als verschiedene
Genotypen. Dadurch können mehrere unterschiedliche Kandidaten erhalten bleiben,
ohne alle Kriterien in eine gewichtete universelle Fitnesszahl zu pressen.

Die historisierten Frontnummern umfassen die für die Elitebesetzung
verarbeiteten Fronten. Nicht ausgewählte Kandidaten und ihre vollständigen
Kennzahlen bleiben trotzdem im Katalog. Bei leerer geeigneter Population oder
erreichtem Kandidatenbudget endet die Suche regulär.

## Trainingsdaten und Abschlussvalidierung

Trainings- und Validierungs-Seeds müssen eindeutig und disjunkt sein. Mutationen,
Paretoauswahl und Finalist nutzen **ausschließlich Trainingsergebnisse**.
Vor der ersten Validierung wird `frozen-finalist.json` unveränderlich geschrieben.
Der global nach Trainingsqualität, Aufwand und stabiler Identität gewählte
Finalist wird danach einmal auf dem Validierungsplan gegen die ursprüngliche
Ausgangskonfiguration bewertet. Sind beide identisch, wird dieselbe Studie
wiederverwendet. Es wird nach Kenntnis der Validierung kein Ersatzfinalist gewählt.

`held_out_quality_improved` verlangt:

- Mindestanforderungen für Kandidat und Baseline;
- positiven Qualitätsvorteil über `min_quality_gain` und positive untere Grenze
  eines seedgepaarten 95-%-Bootstrapintervalls der Qualitätsdifferenz;
- positive Kontrollvorteile und drei einseitige gepaarte Vorzeichentests:
  Kandidat gegen eingefroren, gegen gemischt und gegen Baseline;
- pro Test `p < alpha / 3`, standardmäßig `alpha = 0.05`;
- Ressourcenverhältnis innerhalb des vorab festgelegten Faktors, standardmäßig
  höchstens das 1,25-Fache der Baseline.

Kontrolltests verwenden konservativ die schwächere unmittelbare/verzögerte
Differenz pro Seed, der Baselinevergleich die entsprechende schlechteste
Kontrollmarge. Der Bootstrap resampelt gepaarte Seeds und berechnet die vier
mittleren Margen jedes Mal neu; 4.000 Resamples mit festem Seed 1234 im Beispiel.
Gleichstände zählen bei Vorzeichentests nicht als unabhängige Evidenz.

Diese begrenzte Prüfung eines vorab eingefrorenen Finalisten auf anderen Seeds
derselben Aufgabe ist **kein allgemeiner wissenschaftlicher Verbesserungsnachweis**
für NERVA, keine Validierung auf anderen Aufgaben und kein Ergebnis von M9/M11.
Parameterwahl auf Trainingsdaten und spätere menschliche Wiederverwendung der
Validierungsdaten müssen bei weiterführender Forschung berücksichtigt werden.
Ein ressourcengünstigerer Kandidat mit gleicher Qualität wird beschrieben,
aber nicht als Qualitätsgewinn ausgegeben. Es erfolgt keine automatische
Übernahme in das Originalmodell.

## Historie, Wiederaufnahme und Datenbank

| Pfad | Inhalt |
|---|---|
| `selection-manifest.json` | Normalisierte Suchdefinition, Budget, Ausgangskandidat, native Binär- und Python-Quellprüfsummen |
| `selection-plan.json` | Optional vorab gespeicherte Planansicht |
| `candidates/`, `evaluations/` | Effektive Konfigurationen und Trainingsbewertungen |
| `generations/` | Vorschläge, Eltern, Mutationen, Ablehnungen, Population und Eliten |
| `training/<id>/` | Native JSON-/CSV-/SQLite-Studie auf Trainingsseeds |
| `frozen-finalist.json` | Festgelegter Kandidat vor Validierungszugriff |
| `validation/<id>/`, `validation-evaluations/` | Finalist/Baseline auf zurückgehaltenen Seeds |
| `final-result.json`, `selection-summary.md` | Validierungsbefund und Grenzen |
| `selection.sqlite` | Kandidaten, Generationen, Abstammung, Eliten, Validierung und Metadaten |

Suchprotokolle sind SHA-256-geprüfte JSON-Hüllen. Bestehende Inhalte müssen
bei Wiederaufnahme exakt übereinstimmen. Native Studien prüfen zusätzlich ihre
eigenen unveränderten Manifeste und Records. Die Suche rekonstruiert ihre
deterministischen Generationen aus verifizierten Studien; fehlende Trials
werden fortgesetzt. Ein OS-Lock schützt den Suchordner. Die SQLite-Momentaufnahme
wird nach jeder abgeschlossenen Generation sowie am Ende atomar aufgebaut.

`selection-report` benötigt vollständige gespeicherte Studien und berechnet
keine neuen neuronalen Trials. Die Datenbank der zuletzt abgeschlossenen
Generation ist auch nach einem Abbruch unmittelbar lesbar. Unfertige oder
manipulierte Archive werden nicht als vollständige Resultate ausgegeben.

Beispielabfrage:

```sql
SELECT id, json_extract(evaluation, '$.eligible') AS eligible,
       json_extract(evaluation, '$.quality') AS quality,
       json_extract(evaluation, '$.resource') AS resource
FROM candidates;
```

## Verifikation und tatsächlich ausgeführtes Beispiel

Der vollständige Beispiel-Lauf vom 28.10.2026 umfasst **vier Generationen und
24 unterschiedliche Trainingskandidaten**. Es bleiben zwei Eliten pro Generation.
Finalist und Baseline wurden anschließend auf sechs disjunkten Validierungsseeds
geprüft. Beide erreichen dort Qualität **3,0** und Ereignisaufwand **2.726**.
Die Qualitätsdifferenz und ihr Bootstrapintervall betragen null;
`held_out_quality_improved = false`. Es wird keine Verbesserung behauptet.

Tests prüfen synthetisch einen über Generationen verbesserten Kandidaten,
getrennte Validierungszugriffe, unveränderte Trainingsauswahl bei anderem
Validierungsergebnis, echte Mechanismusschalter, verzögerte Proben, leere
Populationen, Budgets, Paretoauswahl, Wiederaufnahme und Integritätsfehler.

```bash
cargo test --offline --manifest-path NERVA/Cargo.toml
cargo test --offline --manifest-path NERVA_Modell/research/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/research/Cargo.toml --all-targets -- -D warnings
python3 -W error::ResourceWarning -m unittest discover -s NERVA_Modell/research/tests -p 'test_*.py' -v
```

S3 (Architektur/Entwicklungsregeln), S4 (Codeänderungen), automatische
Testaufgabenerzeugung und ein KI-Forschungsagent sind keine implementierten
Suchmöglichkeiten dieses Prototyps.
