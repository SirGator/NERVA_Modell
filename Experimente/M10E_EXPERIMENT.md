# M10-E: Core-Gewichtsbeitrag mit Rekurrenzsperren kreuzen

Definition vom **27.10.2026**, vor Pilot und Hauptdiagnose. Explorative
Ursachenanalyse auf bekannten Seeds, keine unabhängige Bestätigung und keine
neue neuronale Regel.

## Fester Umfang

M9-Kombination ×4 Core-Gewichte / STDP-Tau 100 ms. Technischer Pilot auf dem
vorhandenen Kurzlauf Seed 692 (8/8/8 Trials), danach alle 20 bekannten Seeds
610–629 (256 Trainings-, 128 Kalibrier-, 128 Online-Testtrials). Vier Arme,
beide Belegungen und beide unabhängigen Zeitpunkte bleiben erhalten.

Zwei Gewichtszustände werden mit den vier M10-D-Sperren gekreuzt:

| Gewichtszustand | Alle aktiv | Erregung blockiert | Hemmung blockiert | Beide blockiert |
|---|---|---|---|---|
| Trainierte Gewichte | ✓ | ✓ | ✓ | ✓ |
| Core-Gewichte auf bedingungsspezifische Anfangswerte zurückgesetzt | ✓ | ✓ | ✓ | ✓ |

Das ergibt 256 einzelne Aktionsfenster pro Seed, 5.120 in der Hauptdiagnose.
Alle Varianten werden vollständig ausgeführt und berichtet; kein Nachjustieren
nach dem Pilot. Der Pilot prüft ausschließlich die technische Ausführung.

## Gleicher Zustand vor der Aktion

Erst nach derselben neutralen Baseline, unmittelbar vor der Aktion, werden
Core→Core-Gewichte zurückgesetzt und danach gegebenenfalls Core→Core-Signale
blockiert. Nur die ausgewählten Gewichte, Enabled-Flags und gesperrten
Queueeinträge ändern sich. Zellzustände, Refraktärzeiten, lokale Spuren,
Uhrzeit, andere Gewichte, Aktionskopien und der visuelle Ausleser bleiben erhalten.
Kein Refit oder Weiterlernen während der Probe.

**Unterschied zu M9/M10:** Dort fand die Gewichtsrücksetzung vor der Baseline
statt. Hier erfolgt sie danach, damit der Anfangszustand der Aktionsprobe in
allen acht Varianten identisch ist. Rücksetzungszweige sind deshalb keine
direkten Replikate der früheren Rücksetzungszweige. Die vier unveränderten
Gewichtszweige müssen hingegen die archivierten M10-D-Proben reproduzieren.

Bereits unterwegs befindliche, nicht gesperrte Eingänge behalten die Amplitude
vom Emissionszeitpunkt. Gewichtsreset löscht keine gespeicherten Zellzustände
oder vorherige Einflüsse. Die Diagnose isoliert die Notwendigkeit aktiver
Core-Gewichte unter diesem Zustand, nicht das gesamte Gedächtnis. Motorische
und andere nicht-Core→Core-Signalwege bleiben aktiv. Wirkungsbilder werden
während der Aktionsprobe nicht gezeigt.

## Messungen und Kontraste

Unverändert: ursprüngliche Einzelvorhersage, M10-B-Paar-Rangfolge, signierter
Wirkungskontrast, Spikezahlen und aktive Zellen. Zusätzlich je identischem
Paar/Zeitpunkt/Sperre die Unterschiede zwischen trainiert und zurückgesetzt:
Spikezählmuster (L1 und Änderungsindikator), Spikezeiten/Ereignisse,
Vorhersagewechsel und synchrones Restpotenzial (L∞).

Der funktionale Gewichtseffekt ist `trainiert − zurückgesetzt`, getrennt je
Seed, Arm, Belegung, Zeitpunkt und Sperre. Die Wechselwirkung ist
`Gewichtseffekt unter Sperre − Gewichtseffekt ohne Sperre`. Weiter werden die
Gewichtseffekte im gekoppelten Arm mit vor Training, gealtert eingefroren und
entkoppelt verglichen. 95-%-Bootstrapintervalle: 4.000 seedgepaarte Resamples,
Seed 1234, separat je Belegung/Zeitpunkt. Alle Kontraste sind deskriptiv,
ohne neuen Signifikanz- oder Lernschwellwert. Keine Auswahl von Signalwegen,
Seeds oder Auslesern anhand dieser bekannten Daten.

Eine Potenzial-, Spike- oder Aktivitätsänderung beweist allein keinen
Lernfortschritt. Ein positiver funktionaler Kontrast müsste in beiden Belegungen
und gegenüber Kontrollen bestehen und anschließend auf neuen Seeds mit einem
eigenen vorab festgelegten Protokoll bestätigt werden.

## Prüfungen

Vollständiges ursprüngliches M9-Ergebnis und sechs historische CSVs unverändert.
Alle vier trainierten Zweige entsprechen M10-D. Die Baseline sowie die
Aktions-Anfangspotenziale und Refraktärzeiten sind in sämtlichen acht Varianten
identisch. Zurückgesetzte Kanten/Werte werden gegen Anfangs- und Trainingsgewichte
geprüft; Sperrlisten gegen Topologie/Polarität. Keine gesperrte Ankunft während
der Aktion, neun erhaltene Aktionskopie-Ankünfte. Ereigniszähler und neuronale
LIF-Endzustände werden unabhängig rekonstruiert, Potenzialtoleranz 2e-5.

Wenn alle tatsächlich geänderten Kanten zugleich gesperrt sind, müssen beide
Gewichtszweige identische Aktionsereignisse und Zustandsverläufe erzeugen. Das
gilt auch bei leerer Änderungsliste und dient als interner Negativkontrollfall.

[Ausführung](m10e/README.md).
