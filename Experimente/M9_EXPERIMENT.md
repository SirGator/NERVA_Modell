# M9-v2: Kombination von Rekurrenz und STDP-Zeitskala

Überarbeiteter Versuchsplan vom **27.10.2026**, vor den wissenschaftlichen M9-Messungen.
Revision v2 ergänzt 20 Bestätigungsseeds und kontrollierte Gewichtsrücksetzungen.
Die [ursprüngliche v1-Planung](M9_V1_EXPERIMENT.md), ihre Manifeste und ihr
technischer Kurzlauf bleiben erhalten; die wissenschaftlichen v1-Reihen wurden
nicht gestartet.
**Status: Pilot und Bestätigung auf 20 neuen Seeds abgeschlossen. Kein
funktionaler Lernnachweis.** [Ergebnisse und Grenzen](../../Messungen/M9_EXPERIMENT.md).
Der folgende Versuchsplan wurde vor den wissenschaftlichen Reihen festgelegt;
die gespeicherten Manifeste blieben unverändert.

## Frage und Hypothese

[M8](M8_EXPERIMENT.md) zeigt getrennt zusätzliche Spike-Weiterleitung bei
vierfachen Core-Gewichten und stärkere erfahrungsbezogene Gewichtszuordnung bei
längeren STDP-Zeitkonstanten. Keine Einzelbedingung erfüllt die Lernkriterien.
M9 prüft, ob beide Eingriffe gemeinsam funktionale Handlungswirkungsvorhersagen
ermöglichen. Eine Verbesserung ist eine Hypothese, kein erwartetes Ergebnis.

## Vorab festgelegte Matrix

| Fall | Alle Core→Core-Gewichte | STDP-Tau für LTP und LTD |
|---|---:|---:|
| `reference` | ×1 | 20 ms |
| `weight4` | ×4 | 20 ms |
| `tau100` | ×1 | 100 ms |
| `weight4_tau100` | ×4 | 100 ms |

Vierfache Gewichte betreffen Erregung **und Hemmung**. Längere Zeitkonstanten
verbreitern Potenzierung **und Depression**. Die bestehende Nearest-Neighbor-
Pair-STDP bleibt unverändert: Amplituden 0,0001/0,00012, Paarfenster 100 ms,
Gewichtsgrenzen 0–2. Es wird keine neue neuronale Regel eingeführt.

Fest bleiben: 32 Core-Neuronen, pixelbasierter 2-Kanal-Diagnosesensor,
deterministische Kodierung, drei Aktionskopien mit Gewicht 0,9, Core-Membran-Tau
20 ms und 50 ms Aktions-/Wirkungsabstand. **Homöostase und Strukturentwicklung
sind aus.** Anfangskanten, Zellpositionen, Polaritäten, Verzögerungen und sämtliche
peripheren Gewichte stimmen pro Seed über die vier Fälle überein; ausschließlich
Core-Gewichte werden gegebenenfalls skaliert. Der sensorische Ratenverstärker
wird aus dem Referenznetz des Seeds berechnet und in allen Bedingungen beibehalten.

Alle Fälle enthalten die vier Arme `before`, `aged_frozen`, `paired` und
`scrambled` sowie normale und umgekehrte Tastenbelegung. Die ausgeführten Tasten
bleiben beim Belegungswechsel identisch. Der neuronale Actor erhält nur aktuelle
Bilder und eigene ausgeführte Tasten; Klassen, Zukunftsbilder und Auswertung
bleiben außerhalb des Actors.

## Seeds und Messumfang

- Pilot: **600–602**, alle vier Bedingungen.
- Bestätigung: **610–629** (20 neue Seeds), alle vier Bedingungen unverändert, keine Auswahl oder
  Nachjustierung anhand des Piloten.
- Je Fall/Seed: **256 Trainingstrials, 128 Kalibriertrials und 128 Trials je
  Online-Prüfphase**. Die Trialzahlen sind im wissenschaftlichen Runner fest.
- Technischer Kurzlauf: eigener Seed **692**, standardmäßig 8/8/8 Trials,
  ausdrücklich keine wissenschaftliche Messreihe.

Die Pläne liegen vor der Ausführung unter
[`m9-v2-pilot/manifest.json`](../../Messungen/Experimente/m9-v2-pilot/manifest.json) und
[`m9-v2-confirm/manifest.json`](../../Messungen/Experimente/m9-v2-confirm/manifest.json).
Vorhandene Manifeste und abgeschlossene Einzelergebnisse werden nicht verändert.

## Primäre Endpunkte und Lernkriterien

1. `checkpoint_probe`: A und D jeweils auf einem eigenen exakten Replay des
   Trainingsendzustands.
2. `rest_checkpoint_probe`: dieselben getrennten Replayproben nach unabhängigen
   2 Sekunden neutralem Bild, ohne vorherige Online-Prüfung.

Beide Endpunkte werden separat beurteilt. Eine spätere negative Ruheprobe
verwirft einen positiven unmittelbaren Endpunkt nicht. Pro Belegung/Seed sind
dies nur **zwei Beispiele**, keine 128 unabhängigen Checkpointproben. Vor der
ersten Prognose liegen weiterhin 50 ms Startbild und 50 ms Aktionsintegration.
Visuelle Kalibrierung läuft ausschließlich auf einem eigenen Replayzweig.

Ein Lernnachweis je Bedingung/Endpunkt verlangt sämtliche bisherigen M6/M8-Kriterien:

- Mindestens zwanzig Seeds und mittlere visuelle Auslesbarkeit ≥80 % **in jeder
  Belegung separat**.
- Spikezähl-Vorhersage über Zufall und allen drei Kontrollarmen, in beiden
  Belegungen separat.
- Belegungswechsel und gleichzeitig richtige Antworten beider Belegungen besser
  als alle drei Kontrollarme.
- Für jeden erforderlichen gerichteten Kontrast positive untere Grenze des
  seedgepaarten 95-%-Bootstrapintervalls und einseitiger exakter gepaarter
  Vorzeichentest mit **p ≤ 0,05/(4×2) = 0,00625**. Gleichstände werden ausgelassen.

Die Familienkorrektur umfasst vier Bedingungen und zwei primäre Endpunkte.
Innerhalb eines Endpunkts sind sämtliche Kontrollkontraste gemeinsam erforderlich.
Die Bootstrapintervalle allein sind deskriptiv. Analysiert werden gepaarte Seeds,
nicht einzelne Trials als unabhängige Replikate. Der Pilot kann nichts bestätigen.

## Kontrollierte Gewichtskausalität

Zusätzlich zu den bisherigen Kontrollarmen wird an beiden primären Zeitpunkten
für jede Taste eine eigene, exakt wiedergegebene Instanz des eingefrorenen
Trainingszustands verwendet. Bei der Ruheprobe verstreichen zunächst unabhängig
2 Sekunden mit neutralem Bild. Danach, vor den 50 ms Startbild, erfolgen:

| Eingriff | Gewichte |
|---|---|
| `sham` | Trainierte Gewichte unverändert |
| `restore_core` | Nur Core→Core auf ihre bedingungsspezifischen Anfangswerte |
| `restore_all_plastic` | Sämtliche plastischen Verbindungen auf ihre Anfangswerte |

Alle drei Varianten erhalten ausschließlich neutrale Bilder und dieselben drei
Aktionskopien. Es wird kein Wirkungsbild gezeigt. Membranzustände, lokale Spuren,
Uhren und reisende Impulse werden nicht zurückgesetzt. Die Elterninstanz bleibt
unverändert; während der Proben bleiben alle Lern-/Entwicklungsregeln aus.

Der bereits auf einem separaten Zweig kalibrierte Spikezähl-Ausleser bleibt
für alle Eingriffe fest. Seine Parameter werden zusammen mit den tatsächlichen
Spikezeiten und den exakten Gewichtsänderungen gespeichert, damit der Audit
Vorhersagen unabhängig nachrechnen kann. `sham` muss genau die Vorhersagen und
Spikes der ursprünglichen Checkpointprobe reproduzieren. In eingefrorenen Armen
müssen alle Rücksetzungen ohne Gewichtsänderung identisch zu `sham` ausfallen.
Pro Fall/Seed entstehen zusätzlich **96 unabhängige Eingriffszweige**.

**Der vollständige M9-v2-Nachweis verlangt sowohl die Verhaltenskriterien als auch
Gewichtskausalität:** Im gekoppelten Arm muss `sham` in jeder Belegung die
Genauigkeit von `restore_all_plastic` übertreffen. Je Belegung sind eine positive
untere Grenze des seedgepaarten 95-%-Bootstrapintervalls und einseitig exaktes
Vorzeichen-p ≤0,00625 erforderlich, bei mindestens 20 Seeds. Die zusätzliche
Kausalitätsanforderung ist mit den Verhaltensanforderungen gemeinsam notwendig;
die Familie bleibt vier Bedingungen × zwei Endpunkte.

`restore_core` dient zunächst der deskriptiven Lokalisation. Ein Verhaltensvorteil
mit negativem Rücksetzungstest ist kein vollständiger M9-v2-Gesamtnachweis.
Ein positiver Test zeigt einen Beitrag der Gewichte zur getesteten Auslesung bei
festgehaltenem Ausgangszustand. Er beweist keine ausschließliche Speicherung in
Gewichten oder deren alleinige Hinreichendheit. Ein Verlust kann auch die
Passung zum festen Ausleser betreffen. Bereits reisende Impulse behalten ihre
Emissionsamplituden; kurzfristige zustandsabhängige Beiträge werden nicht entfernt.

## Mechanistische und sekundäre Diagnosen

An beiden primären Zeitpunkten gibt es unabhängige Proben ohne Aktionskopie.
Zusatzspikes außerhalb der direkt angeregten Zellen werden diesen Hintergrund-
und den eingefrorenen/entkoppelten Kontrollen gegenübergestellt. Zusatzspikes,
visuelle Auslesbarkeit und topologische Gewichtszuordnung allein zählen nicht
als Lernnachweis.

Vorab festgelegte deskriptive Kontraste, separat je Belegung und primärem Endpunkt:

- Kombination minus Referenz, minus `weight4` und minus `tau100`.
- Faktorielle Interaktion `Y11 − Y10 − Y01 + Y00`.
- Jeweils sowohl rohe Genauigkeit des gekoppelten Arms als auch dessen
  Lernvorteil gegenüber jedem Kontrollarm.

Diese seedgepaarten Bootstrapkontraste liefern keinen eigenständigen
Signifikanznachweis von Synergie. Ein funktionaler Lernnachweis verlangt nicht
zusätzlich einen signifikanten Kombinationsvorteil gegenüber jeder Einzelbedingung;
das sind unterschiedliche Fragestellungen.

Die anfängliche und trainierte visuelle Gewichtssumme sowie ihr erhaltener Anteil
werden pro Seed und Arm protokolliert. Dies beobachtet den in M8 sichtbaren
gegenläufigen Effekt verstärkter sensorischer Depression. Sekundär bleiben
Online-Probe, erste 16 Trials, sequenzielle Ruhe-/Interferenzprobe und
Zeitbin-Auslesung erhalten. Die Gewichte sind während sämtlicher Prüfungen
eingefroren; damit wird keine Stabilität bei weiterlaufender Plastizität belegt.

## Ausführung und Grenzen

[Startbefehle und Dateivertrag](m9/README.md). M9 verwendet die vorhandenen
M8/M6-Replay- und Messroutinen mit eigenem strengen Konfigurationsformat `NERVA-M9-v2`.
Der bisherige v1-Runner ist unter `m9/v1.py` weiter verfügbar.
M8 behält seine Einzelfaktorbedingungen und seine Familienkorrektur von 13×2.
Die Standardwerte des Live-Modells und die NERVA-Bibliothek werden nicht verändert.

Ein negatives Ergebnis betrifft diese Matrix und dieses Diagnoseprotokoll. Es
beweist weder, dass lokale Selbstorganisation unmöglich ist, noch dass ein
bestimmter zusätzlicher biologischer Mechanismus zwingend benötigt wird.
