# M10-D: Rekurrente Erregung und Hemmung getrennt blockieren

Definition vom **27.10.2026**, vor Pilot und Hauptauswertung. Diagnose auf
bekannten M9-Seeds, keine unabhängige Bestätigung. Keine neue Lernregel.

## Umfang

Kombination Core-Gewichte ×4 / STDP-Tau 100 ms. Technischer Pilot auf dem
vorhandenen M9-v2-Kurzlauf Seed 692 mit dessen kleinen Trialzahlen. Danach
alle 20 M9-Bestätigungsseeds 610–629 mit unveränderten 256 Trainings-,
128 Kalibrier- und 128 Online-Testtrials. Pilot und Diagnose werden getrennt
archiviert. Alle vier Arme (vor Training, gealtert eingefroren, gekoppelt,
entkoppelt), beide Belegungen und beide unabhängigen Zeitpunkte werden geprüft.

## Vier Varianten

- `sham`: alle Signalwege aktiv.
- `block_exc`: erregende Core→Core-Übertragung blockieren.
- `block_inh`: hemmende Core→Core-Übertragung blockieren.
- `block_both`: beide rekurrenten Core→Core-Anteile blockieren.

Aus identisch wiederholten eingefrorenen Checkpoints zunächst gegebenenfalls
2 Sekunden ruhen und dann die identische neutrale 50-ms-Baseline durchlaufen.
Erst unmittelbar vor der Aktion werden die ausgewählten Synapsen deaktiviert
und alle bereits unterwegs befindlichen Ankünfte dieser Kanten aus der Queue
entfernt. Das ist eine Übertragungssperre, keine Gewichtsnullung. Nur die
Enabled-Flags und ausgewählte Queueeinträge ändern sich; Gewichte, Identitäten,
Neuronenzustände und Zeit bleiben erhalten. Der Zweig endet nach der 50-ms-Probe.

Aktionskopien, Motor-Rückkopplung und alle nicht-Core→Core-Signalwege bleiben
aktiv. `block_both` ist daher kein ausschließlich direkt gespeistes Netzwerk.
Keine Wirkungsbilder während der Aktionsproben. Kalibrierung erfolgt auf dem
unveränderten Trainingscheckpoint; kein Fit oder Refit nach dem Eingriff.

## Vorab festgelegte Messung und Interpretation

Je Seed, Arm, Belegung, Zeitpunkt und Variante werden ursprüngliche balancierte
Einzelvorhersage, M10-B-Paar-Rangfolge/Wirkungsrichtung, Projektionskontrast,
Spikezahlen, aktive Core-Zellen und höchste Spikezahl einer Zelle erfasst.
Zusätzlich Unterschiede zu `sham` in Spikezählmustern und Entscheidungen.
Mehr Aktivität allein ist kein Lernerfolg; erhöhter Aktivitätsgrad muss bei
scheinbar besseren Richtungen gemeinsam berichtet werden.

Alle Varianten werden vollständig berichtet. Vergleiche zu `sham` und zu den
drei Kontrollarmen werden pro Belegung/Zeitpunkt über dieselben Seeds gepaart;
deskriptive 95-%-Bootstrapintervalle mit 4.000 Resamples und festem Zufallsseed
1234. Kein neuer Signifikanzschwellwert, keine Auswahl von Seeds, Signalwegen
oder Auslesern anhand der Ergebnisse. Bekannte Seeds werden nicht nachträglich
als neue Bestätigung dargestellt. Ein vielversprechender Befund braucht einen
separaten vorab festgelegten Versuch auf neuen Seeds.

Das vollständige ursprüngliche M9-Ergebnis und alle sechs historischen CSVs
müssen reproduziert werden. Shamproben müssen den ursprünglichen eingefrorenen
Proben entsprechen. Alle Baselines und Ausgangspotenziale/Refraktärzustände
müssen zwischen Varianten identisch sein. Die Liste gesperrter Kanten wird
anhand ursprünglicher Topologie/Polarität unabhängig geprüft; während der
Aktion darf keine ihrer Ankünfte auftreten. Neutrale LIF-Trajektorien werden
aus den Ereignissen rekonstruiert, Toleranz 2e-5; feste Ausleserprognosen aus
gespeicherten Parametern nachgerechnet. Aktionskopie-Ankünfte bleiben neun.

Der Eingriff untersucht die Notwendigkeit dieser Übertragungen für die Antwort
unter dem festen Zustand/Ausleser. Er beweist nicht die Eignung einer dauerhaften
Entfernung, die Suffizienz einer Lernregel oder eine allgemeine Informationslosigkeit.

[Ausführung](m10d/README.md).
