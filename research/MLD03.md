# NERVA MLD-03: Schwelle, Integrationszeit und Homöostasepopulation

## Festlegung vor der Messung

Dies ist ein gepaarter kausaler Folgeversuch auf den **bereits beobachteten**
MLD-02-Seeds 17027, 23003, 31013, 31019 und 31033. Es gibt keine neue
Bestätigungskohorte und keinen populationsstatistischen Signifikanzanspruch.
Die Anregung stammt aus der nachträglichen Auswertung des vollständigen
MLD-02-Archivs. Fehlende numerische Vorgaben werden hier vor der Messung festgelegt.
MLD-02-Rohdaten, Vorregistrierung und Ergebnisbericht bleiben erhalten.

Zwei Abschnitte benutzen das native vollständige Netzwerk mit 256 Core-Zellen,
deterministischen 8×8-Pixelsensoren, acht Trainingswiederholungen und 11 Sekunden
Ruhe vor jedem Probe-Bild. Bildfolge und neue Fragmente entsprechen [MLD-02](MLD02.md):
Schwarz, Voll-A, Voll-B, Original-Cue-A/B, vier vorher definierte Einzelpixel-Cues.
Die Ruhe ist keine harte Zustandslöschung. Zielmuster sind jeweils die Vollbild-
Core-Antworten des festen Referenzzustands und werden nicht pro Eingriff neu gewählt.

## A: Erregbarkeit gegen Gewichtswirkung

Alle Arme werden aus exakt demselben vollständig trainierten B-Checkpoint
deterministisch rekonstruiert und auf vollständige Replay-Gleichheit geprüft.
Lernen, Entwicklung und Homöostase sind danach eingefroren. Gewichtswechsel
erhalten Restbeträge, Zellzustand und bestehende Ereigniswarteschlangen;
bereits reisende Eingaben behalten ihre bei Aussendung erfasste Amplitude.

Fünf Gewichtsmuster: anfängliche Gewichte (Reset), natürliches B, B nur auf
Core→Core bei zurückgesetzten übrigen Kanten, G (Lernfaktor ×64), D (andere
disjunkte gleich dichte Bilder). Es gibt keine nachträgliche Gewichtsverstärkung
und keinen künstlich erfolgreichen Referenzarm in diesem Test.

| Feste Bedingung | Core-Schwellenabstand über Ruhe × | Core-Membranzeitkonstante × |
| --- | ---: | ---: |
| native | 1 | 1 |
| threshold_09999 | 0,9999 | 1 |
| threshold_099 | 0,99 | 1 |
| threshold_090 | 0,9 | 1 |
| tau_half | 1 | 0,5 |
| tau_double | 1 | 2 |

Das sind zwei vorab festgelegte Einzelparameterkurven, kein vollständiges
Schwelle×Zeitkonstante-Faktorielles. Sensor- und Motorparameter bleiben nativ.
Eine längere passive Zeitkonstante verändert auch die Wirkung bestehender
intrinsischer Ströme; der Test isoliert keine einzelne Leitungslatenz.
Bestehende Refraktärzeiten, Chemie und Schwellenadaptation bleiben aktiv.
Die tatsächlichen Basis-/Effektivschwellen und Zeitkonstanten werden protokolliert.
Beim Eingriff werden Zellen mit den alten Parametern bis zur aktuellen Runtime-
Grenze entwickelt; die Runtime erneuert danach intrinsische Spikevorhersagen.
Die Identitätseinstellung ist ein echter No-op.

Jedes Gewichtsmuster wird bei **derselben** Erregbarkeit gegen den eigenen
Reset-Gewichtsarm verglichen. Die Aktivitätsänderung des Reset-Arms gegenüber
`native` wird gesondert ausgewiesen. Ein Eingriff, der nur mehr Spikes erzeugt,
erfüllt kein Lernkriterium.

Vorab festgelegte funktionale Kriterien mit Epsilon 10⁻⁹:

- Originaler Erfolg: beide Original-Cues haben positive feste Zielmarge und
  positiven Gewinn gegenüber Reset, aktive Core-Antwort, getrennte aktive
  Vollbildziele und keine Zunahme der Schwarz-Core-Antwort gegenüber Reset.
- Neue Cues: dasselbe Kriterium auf allen vier neuen Fragmenten; Gesamt-Erfolg
  verlangt alle sechs Cues gemeinsam.
- Diagnostischer Gedächtniskandidat: B und B-Core bestehen gemeinsam alle sechs
  Kriterien; D besteht sie nicht und B-Core übertrifft D bei allen sechs Gewinnen.
  Das wäre eine durch **künstlich veränderte Erregbarkeit** sichtbar gemachte
  Gewichtswirkung, kein Nachweis natürlich erfolgreichen Lernens.
- Erster Spikeeffekt: erste Bedingung in der obigen festen Reihenfolge, in der
  B gegenüber dem zugehörigen Reset eine andere Core-Spikefolge hat.
  Dies ist ein Treffer auf diesem Raster, keine minimale notwendige Schwelle.
  Original-/neue Cue-Gewinne und Motorfolgen werden zusätzlich vollständig berichtet.

## B: Homöostase auf derselben entwickelten Struktur

Die identische native einsekündige Entwicklungsphase aus MLD-02 wird je Fork
rekonstruiert: D-Voll-A, D-Voll-B, Schwarz, Schwarz, fünfmal, mit Lernen,
Entwicklung und Homöostase auf allen Populationen. Tatsächliche Formationen oder
Entfernungen sind Voraussetzung. Erst danach wird Entwicklung eingefroren.
Vor Populationseingriffen müssen alle vollständigen Zustände gleich sein.
Der Versuch diagnostiziert damit die **anschließende Trainingsphase**, nicht
den Ursprung bereits während Entwicklung entstandener Anpassungen.

| Arm | Sensor-Homöostase | Core-Homöostase | Andere Populationen |
| --- | --- | --- | --- |
| off | aus | aus | aus |
| sensor | an | aus | aus |
| core | aus | an | aus |
| both | an | an | aus |
| all_bridge | an | an | an |

Der Zusatzarm `all_bridge` verbindet das Ergebnis mit der All-Population aus
MLD-02; Sensor+Core allein schließt Motorzellen nicht ein. Nach Entwicklung ist
Konkurrenz in sämtlichen Armen aus. Native Drei-Faktor-Gewichtsplastizität bleibt
während des Originalbild-Trainings an. Somit werden totale Populationseffekte
einschließlich Rückkopplung auf Gewichtslernen gemessen.

Die native Populationseinstellung steuert Uhren, Zeitanker und zulässige IDs.
Vollständige Homöostase-Aggregate je Neuron müssen ausschließlich innerhalb der
jeweiligen Population liegen; aktive Arme müssen tatsächliche Updates zeigen.
Die gemeinsame Topologie und die anfänglichen Gewichte werden nachgewiesen.
Jeder Endzustand wird zusätzlich mit ausschließlich zurückgesetzten Gewichten
verglichen. Das trennt Gewichtsbeiträge von übrigen Endzustandsunterschieden.

Primär berichtet werden je Cue Sensor-/Core-Spikezahlen, vollständige Spikefolgen,
feste Zielmargen und eigener Gewichtsgewinn. Sensor allein ist für eine bestimmte
Aktivitätsunterdrückung hinreichend, wenn seine Antwort die All-Bridge-Antwort
erreicht, während off sie nicht zeigt. Ein solcher Befund beweist keine universelle
Schädlichkeit oder alleinige Notwendigkeit der Sensor-Homöostase.
Core allein und beide werden parallel berichtet; der feste 2×2-Kontrast ist
`F_both − F_sensor − F_core + F_off` auf sämtlichen sechs Margen. Nichtzero
über Epsilon bedeutet eine deterministische Nichtadditivität in diesem Netz,
keinen populationsstatistischen Nachweis.

## Elektrische Messung und Grenzen

Alle Probe-Spikes werden als Neuron-ID und absoluter Zeitpunkt gespeichert.
Alle beobachteten Core-/Motor-Integrationsgrenzen enthalten Zeitpunkt, ID,
Potenzial vorher/nachher, Schwelle, Summeneingang, Spike-/Refraktärstatus und
Refraktärende. Die bestehenden vollständigen elektrischen Aggregate und
Potenzialbilanzen bleiben erhalten. Restereignisse werden wie bisher getrennt
gezählt, nicht als vollständige Rest-Timeline gespeichert.

Die unabhängige Auswertung rekonstruiert Spikezählungen aus Zeiten,
f32-Schwellenüberschreitungen aus Integrationsgrenzen, Cue-Margen aus Rohzählungen,
Gewinne und Gates, Populationstreue sowie den Interaktionsterm. Sie misst Größe
und RMS der Potenzialdifferenz vor Integration nur an eindeutigen gemeinsamen
Zeitpunkt-/Neuron-Paaren. Nach divergierenden Trajektorien fehlende oder
mehrdeutige Paare werden gezählt und ausgeschlossen. Diese Stichprobe ist kein
vollständiger kontinuierlicher Spannungsvergleich; der minimale Schwellenabstand
umfasst auch tatsächlich überschrittene Schwellen.

## Budget, Quellenbindung und Befehle

A benötigt 34 Trainingsrekonstruktionen pro Ausführung; B benötigt 12 gemeinsame
Entwicklungsrekonstruktionen und zehn Trainingsphasen, zusammen 22.
Mit vollständigem Replay und fünf Seeds sind es höchstens **560** Phasen.
Höchstens zwei Seeds laufen parallel, Wall-Budget 7200 Sekunden. Vorhandene
Auditgrenzen gelten weiter; Überlauf macht den Trial ungültig und wird nicht
still gekürzt. Keine zusätzlichen Dosen, Seeds oder Referenzziele werden anhand
der Ergebnisse gewählt.

```bash
# Von main/ aus:
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/mld03_smoke.toml --output /tmp/nerva-mld03-smoke
./NERVA_Modell/research/run.sh mld03 plan NERVA_Modell/research/config/mld03_20261112.toml
./NERVA_Modell/research/run.sh mld03 run NERVA_Modell/research/config/mld03_20261112.toml
./NERVA_Modell/research/run.sh mld03 report NERVA_Modell/research/config/mld03_20261112.toml
```

Vor Ausführung bindet `preregistration.json` mit SHA-256 Konfiguration, Seeds,
Protokoll, Binärdatei, Python-Programme, Quellenarchiv und native Manifeste.
Quellcode und dieses Protokoll werden vor dem Hauptlauf committed und gepusht.
Jeder native Trial hat vollständiges Replay; alle zehn Records werden anschließend
erneut nativ und unabhängig geprüft. Rohdaten und Prüfbelege werden unter
`research/results/mld03_20261112/` sowie in einem separaten geprüften ZIP gemäß
[ARCHIVING.md](ARCHIVING.md) gesichert. MLD-03-Records selbst enthalten die
vollständigen Probe-Zeitreihen; ein zusätzlicher Historienexport ist dafür nicht nötig.
