# Präzisionskorrektur: geprüfter Modellvergleich vom 10.11.2026

[Mechanismus](../../NERVA/docs/weight-accumulation.md) ·
[Versuchsprotokoll](PRECISION.md) · [Feste Konfiguration](config/precision_pilot.toml).

Verglichen werden historische f32-Direktupdates und synapsenlokale
f64-Akkumulation auf denselben initialen Netzwerken, Eingängen und bekannten
Seeds **17011 und 17027**. Der Pilot verwendet 256 Processing-Zellen,
acht Trainingswiederholungen und elf Sekunden Schwarzbildruhe vor jedem Abrufbild.
Neuromodulation bleibt an; Strukturentwicklung, Homöostase und Rückführung sind aus.

Zwölf Modellkontrollen und eine gemeinsame numerische Kalibrierung ergeben
13 Records, 26 Ausführungen mit vollständigem Replay, 48 Trainingsdurchläufe
einschließlich Rekonstruktion und 65.952 Modellschritte. Es gibt weder
Parameteroptimierung noch künstliche Gewichtsverstärkung im Modellvergleich.

## 1. Bleibt die kleine Änderungsmasse erhalten?

Ja, im kontrollierten nativen Mechanismustest. Über dieselben 100 ms:

| Veröffentlichung | Intervalle | Angefordert | Sichtbare Gewichtsänderung | Erhaltener Rest |
| --- | ---: | ---: | ---: | ---: |
| direkt | 1 | 0,00000862143229 | 0,00000864267349 | 0 |
| direkt | 1.000 | 0,00000862143593 | 0 | 0 |
| akkumuliert | 1 | 0,00000862143229 | 0,00000864267349 | −0,00000002124120 |
| akkumuliert | 1.000 | 0,00000862143593 | 0,00000864267349 | −0,00000002123757 |

Die negativen Restbeträge korrigieren das leichte Aufrunden bei der sichtbaren
Veröffentlichung. Sichtbare Änderung plus Restbetrag erhält die angeforderte
Masse im 1.000-Intervall-Fall bis auf etwa **8,94×10⁻²²**.
Die verbleibende leichte Differenz der angeforderten Massen entsteht bei
der unveränderten f32-Tagfortschreibung.

Der Kontrollvergleich zeigt somit tatsächlich aktive Library-Akkumulation,
nicht nur eine theoretische externe f64-Rechnung.

## 2. Wie viel numerischer Fehler verbleibt im Modell?

Der Audit bilanziert pro Synapse angeforderte Änderung, tatsächlich publizierte
Änderung, Restbetragsänderung, Begrenzung und numerischen Fehler.
Über die beiden gekoppelten Modellseeds gemittelt:

| Veröffentlichung | Angeforderte Bruttomasse | Publizierte Bruttomasse | Restbetrag-L1 | Rundungsfehler-L1 |
| --- | ---: | ---: | ---: | ---: |
| direkt | 0,000170889887 | 0,000080533326 | 0 | 0,000123493890 |
| akkumuliert | 0,000170889887 | 0,000170923769 | 0,000000589258 | 7,78×10⁻²⁰ |

In diesen beiden Läufen ist die angeforderte Bruttomasse zwischen den
Präzisionsvarianten gleich. Das ist ein beobachtetes Ergebnis; bei veränderter
Dynamik wäre es keine notwendige Eigenschaft. Keine Modellkante wird durch
Gewichtsgrenzen beschnitten.

Die mittleren absoluten Endänderungen je Verbindung steigen:

| Seed | Direktes Netto-L1 | Akkumuliertes Netto-L1 |
| --- | ---: | ---: |
| 17011 | 0,000000521541 | 0,000016421080 |
| 17027 | 0,000159204006 | 0,000319913030 |

Bruttomasse, Netto-Endänderung, Rundungsfehler und Restbetrag sind unterschiedliche
Messgrößen. Gegenläufige Updates können sich auf einer Synapse aufheben.
Rundungsfehler-L1 ist die Summe absoluter Veröffentlichungsfehler und nicht
pauschal ein Anteil verlorener positiver Masse. Sichtbare einzelne Schritte
können wegen Quantisierung größer sein als der aktuelle Einzelbeitrag.

Die beiden eingefrorenen Kontrollen haben weder sichtbare Gewichtsänderungen
noch Restbeträge. Die entkoppelten Kontrollen bleiben separat erhalten;
auch dort reduziert die Akkumulation die numerischen Veröffentlichungsfehler.

## 3. Ändern sich Spikes und gelernte Assoziationen?

Im geprüften Abruf **nein**:

| Seed | Core-Spikes zwischen Präzisionsvarianten geändert | Motor-Spikes geändert | Gewichtseffekt auf Ergänzung A/B |
| --- | --- | --- | --- |
| 17011 | nein | nein | direkt 0/0; akkumuliert 0/0 |
| 17027 | nein | nein | direkt 0/0; akkumuliert 0/0 |

Die innerhalb jedes Arms vollständig rekonstruierten Checkpoints stimmen vor
dem Eingriff mit und ohne Audit überein. Der Gewichtsreset verändert nur
sichtbare Gewichte; die im eingefrorenen Zustand ungenutzten Restbeträge
bleiben erhalten. Nach Rücktausch muss der gesamte Replayzustand wieder
identisch sein.

Die acht vorab festen Tests auf den gewichtsbedingten Ergänzungsgewinn haben
jeweils null positive Seedpaare und `p = 1`. Vorteile gegenüber Direktupdates,
eigenem eingefrorenem Arm und eigener entkoppelter Kontrolle sind damit
nicht nachgewiesen. Die zwei bekannten Seeds sind keine unabhängige Bestätigung.

Die numerische Korrektur funktioniert, reicht in diesem festen Modellversuch
aber allein nicht zu veränderten Abruf-Spikes oder sinnvoller gelernter
Assoziation. Das schließt Wirkungen bei anderen Reizen, Größen oder Seeds
nicht aus.

## Technische Umsetzung und Referenztreue

Die Korrektur liegt zuerst in NERVA und ist ausdrücklich wählbar:
`WeightUpdateMode::Accumulated`. Der Modelleintritt
`LearningRule::ThreeFactorAccumulated` verwendet denselben Mechanismus.
Bisherige Aufrufe behalten die direkten Updates.

Das sichtbare Gewicht bleibt f32; die Synapse hält einen zusätzlichen
signierten f64-Rest. Freeze erhält den Rest ohne Nachlernen gefrorener Zeit.
An Gewichtsgrenzen entsteht keine versteckte Sättigungsschuld.
Autoritative Gewichtsersetzungen löschen den Rest; eingefrorene diagnostische
Gewichtsinterventionen erhalten ihn für den kontrollierten Rücktausch.

Die unveränderte CPU-Referenzdatei besteht ihren Test bytegenau.
Die Direktarme dieses Piloten haben außerdem exakt dieselben initialen
Netzwerk-Digests, trainierten Endgewichte und Abrufzählvektoren wie die
früheren `three_on`-Arme auf beiden Seeds. Der ursprüngliche Modellvergleich
bleibt damit als Referenz nachvollziehbar.

## Prüfung und Reproduktion

Der vollständige Projektprüfaufruf besteht: **437 Rust-Tests**,
**64 Python-Harness-Tests**, **10 Dashboard-Tests**, Rustformat,
Clippy mit Warnungen als Fehlern und identische Dashboard-Server.
VM- und GPU-Hardwaretests wurden nicht erneut gestartet.

Eine unabhängige Prüfung rekonstruiert lokale Massenbilanzen, Restbeträge,
Gewichtsdifferenzen und zählbasierte Assoziationswerte aus den archivierten
Daten. Die maximale Nachrechnungsdifferenz beträgt **2,72×10⁻²⁰**.
Nicht gespeicherte individuelle Auditereignisse werden dabei nicht
rekonstruiert. Die erneute reine Auswertung lässt den Rohdatenbeleg bytegleich.

```bash
./NERVA_Modell/research/run.sh control report NERVA_Modell/research/config/precision_pilot.toml
python3 NERVA_Modell/research/results/precision_pilot_20261110_v2/verify_results.py
```

Die 13 Roh-Records belegen **8.456.232 Bytes**.
Native Binärdatei, Controller, Konfiguration und Rust-Quellen sind archiviert.
CSV/SQLite enthalten die zwölf Modellkontrollen; die numerische Kalibrierung
liegt separat in JSON und den SQLite-Metadaten.

| Artefakt | SHA-256 |
| --- | --- |
| Gesamtplan | `12a4a3244fc22194e3bfd4fd562eefcf365e42afd6143072d36b86315e347699` |
| Rohdatenbeleg | `486c84f67275b8a1bc7d92e89a3b5a50b067b2fa488ddf524a442df46e569530` |
| Quellenarchiv | `deec583c599767072b53fa2ba5fd2ca5bb68b32f5bcf576606a2f78c95a5fb91` |

[Automatischer Bericht](results/precision_pilot_20261110_v2/CONTROL_REPORT.md) ·
[Alle Messwerte](results/precision_pilot_20261110_v2/control-metrics.csv) ·
[Gesamtplan](results/precision_pilot_20261110_v2/control-plan.json) ·
[Rohdatenbeleg](results/precision_pilot_20261110_v2/receipt.json) ·
[Unabhängige Prüfung](results/precision_pilot_20261110_v2/verification.json).
