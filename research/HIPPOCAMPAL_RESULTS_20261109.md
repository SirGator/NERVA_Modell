# Hippocampus-/Neuromodulations-Prototyp: Ergebnis vom 09.11.2026

Die Implementierung wurde zuerst als unabhängige Library in `main/NERVA`
abgeschlossen und geprüft. Danach wurde `NERVA_Modell` über diese öffentlichen
APIs angebunden. Der echte VM-Test besteht; der feste Bildpilot liefert noch
keinen nachgewiesenen Gedächtnisgewinn.

## Technische Integration

Library: 299 Tests ohne Zusatzfeatures, 348 mit Entwicklung/Diagnostik/Visualisierung,
Clippy und Rustformat bestehen. Drei explizite GPU-Tests konnten auf der vorhandenen
RTX 3060 wegen fehlendem NVRTC nicht initialisieren; der CUDA-Rust-Build kompiliert.
[Library und Grenzen](../../NERVA/docs/neuromodulation.md).

Modell: 25 Tests plus eigener echter QEMU-Lauf bestehen. Zwölf aktive Schritte mit
128 Processing-Zellen erzeugten 1.342 Core-Spikes, 2.036 Motorimpulse, 486
Gewichtsänderungen, 177 aktive chemische Pulse und 38 STP-Synapsen. Alle 105
physischen Tastencodes wurden angenommen. 293 Kanten entstanden, 244 wurden
entfernt. Diese Größen zeigen Mechanismen und Gerätewirkung, keine Zielerfüllung.
[VM-Rohbericht](../../Messungen/Modell/hippocampal-engineering-20261109/report.json).

Der Standard-Cortical-Pfad wurde gegen die gesicherten Originalquellen verglichen:
32/128 Processing-Zellen, je vier gemischte Bild-/Tastenbeobachtungen, insgesamt
9.214.739 Bytes bisheriger Zustandsfelder und Schrittresultate stimmen exakt.
Ausgenommen wurden ausschließlich explizit neutrale neue Metadatenfelder.
[Prüfbeleg](../Modell/verification/cortical-default-compatibility.json).

## Fester Bildpilot

[Protokoll](HIPPOCAMPAL.md), [Konfiguration](config/hippocampal_probe.toml),
[Rohdaten/SQLite/Quellarchiv](results/hippocampal_probe_20261109/).
Zwei Seeds, fünf Kontrollen, zehn vollständige Versuche und jeweils ein zweiter
nativer Lauf mit identischen zurückgelieferten Zählungen, Netzwerkprüfsummen und
Scores. Vollständiger versteckter Hostzustand wurde in diesem Versuch nicht
zusätzlich bitweise verglichen. Die separaten Modelltests prüfen diesen Replaypfad.

| Seed | Kontrolle | Ergänzung A | Ergänzung B | Erwerb A | Erwerb B | Separation | Gewichtsupdates |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 42 | cortical | 0.505307 | 0.358569 | 0.043730 | -0.003245 | 0.273727 | 6465 |
| 42 | smooth | 0.446737 | 0.324355 | 0.000000 | 0.000000 | 0.290304 | 3405 |
| 42 | sharp | 0.393149 | 0.301972 | 0.000000 | 0.000000 | 0.264020 | 3152 |
| 42 | no_chemistry | 0.446737 | 0.324355 | 0.000000 | 0.000000 | 0.290304 | 0 |
| 42 | frozen | 0.446737 | 0.324355 | 0.000000 | 0.000000 | 0.290304 | 0 |
| 73 | cortical | 0.285687 | 0.262017 | 0.000000 | 0.000000 | 0.127641 | 6866 |
| 73 | smooth | 0.298828 | 0.320133 | 0.000000 | 0.000000 | 0.127137 | 10690 |
| 73 | sharp | 0.286042 | 0.247079 | 0.000000 | 0.000000 | 0.109920 | 14413 |
| 73 | no_chemistry | 0.298828 | 0.320133 | 0.000000 | 0.000000 | 0.127137 | 0 |
| 73 | frozen | 0.298828 | 0.320133 | 0.000000 | 0.000000 | 0.127137 | 0 |

Das vorab feste Kandidatengate wird bei keinem Seed bestanden. Im weichen
hippocampalen Arm stimmen sämtliche vier nativen Vorher-/Nachher-Zählvektoren
exakt überein; die Nachhervektoren stimmen auch mit Frozen und No-Chemistry überein.
Die bereits vor dem Training positiven Ergänzungskontraste sind deshalb kein
Erwerbsnachweis. Die weiche Variante übertrifft Sharp in diesen beiden festen
Piloten bei den gemessenen statischen Kontrasten, verbessert die Separation
gegenüber Cortical aber nicht bei beiden Seeds. Keine Generalisierung und kein
Beleg biologischer Funktion.

Die unabhängige Python-Nachrechnung aller Rohvektoren ergibt exakt dieselben
Scores (maximaler Skalarfehler 0). Alle Probe-Lernparameter blieben laut geprüftem
Produzenten unverändert; Source-, Archiv- und Binärhashes stimmten zum damaligen Prüfzeitpunkt.
[Unabhängige Prüfung](results/hippocampal_probe_20261109/verification.json),
[ausführbarer Prüfer](results/hippocampal_probe_20261109/verify_results.py).
Die abschließenden Harnessprüfungen umfassen 23 erfolgreiche Rust-Tests;
53 vorhandene Python-Harness-Tests bestanden ebenfalls.

Ein vorausgehender Produzentenfehler übergab elf Sekunden an die auf 50 ms
begrenzte Einzelbeobachtung. Dabei entstanden null Versuchsergebnisse. Er wurde
separat als ungültig archiviert und durch reguläre 50-ms-Ruheschritte samt
vollständigem Produzenten-/Replaytest behoben; er zählt nicht als negatives
Lernresultat. Das abschließende Protokoll und sein Quellarchiv wurden vor dem
ersten gültigen Bildpilot-Ergebnis festgeschrieben.

Entscheidung: Mechanismen als explizit wählbaren Prototyp behalten, historischer
Cortical Core bleibt Standard. Keine Übernahme als bewiesener Gedächtnisgewinn,
kein neues Parameterraster. Funktionelle Mustertrennung/-ergänzung, unabhängige
Bestätigung, eine Intervention auf den Speicherort und dendritische Verarbeitung
bleiben offen. Das ist eine empirische Grenze dieses kleinen Piloten, keine
Aussage über die grundsätzliche Möglichkeit lokaler Gedächtnismechanismen.

## Nachfolgende technische Prüfung

Der spätere [Diagnoselauf](results/hippocampal_weight_diagnostics_20261109/RESULTS.md) ergänzt ausschließlich lesende Gewichtsmessungen und bestätigt sämtliche bisherigen Ergebnisse exakt. Er prüft die hier archivierten Quellen und Rohdaten ohne Änderungen an ihnen. Der gegenwärtige Produzent ist dort separat archiviert.

Die frühere NVRTC-Lücke ist inzwischen mit einer isolierten temporären Laufzeit geschlossen: Alle vier [expliziten CUDA-Tests](../../NERVA/docs/benchmarks/neuromodulation-gpu-20261109.json) bestehen einschließlich des Zusammenspiels von Chemie, verzögertem Lernen und Kurzzeitplastizität. Der CPU-Standard bleibt bestehen.
