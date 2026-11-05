# Homöostasevergleich: Bestätigung auf 24 neuen Seedpaaren

Nachfolgender Versuch: [Lokalisierung nach Zellpopulation vom 06.11.2026](HOMEOSTASIS_POPULATIONS_RESULTS_20261106.md).

Der vorregistrierte Vergleich vom **05.11.2026** ist vollständig ausgeführt.
Ein Verlust visueller Auslesbarkeit durch eingeschaltete Homöostase ist im
festgelegten Assay bei **17, 256 und 1.025 Core-Neuronen bestätigt**. Bei 4.096
Core-Neuronen besteht dieser vorab festgelegte Test nicht. Der separat gemessene
externe Vorhersagefehler ist dort höher; die schwache visuelle Kalibration
erlaubt dabei keine isolierte Aussage über interne Vorhersagefähigkeit.

## Vorab festgelegter Vergleich

[Protokoll](HOMEOSTASIS_CONFIRMATION.md) ·
[Konfiguration](config/homeostasis_confirmation.toml) ·
[Vorregistrierung](results/homeostasis_confirmation_20261105/preregistration.json).

Die 24 neuen Seeds sind für jeden An-/Aus-Vergleich gepaart. Sie überschneiden
sich mit keinem der 162 bekannten oder reservierten Seeds aus den 215 geprüften
lokalen Katalogdateien. Vier Größen und zwei primäre Endpunkte ergeben acht
Tests mit gemeinsamer Fehlerwahrscheinlichkeit 0,05 und strikter Schwelle
`p < 0,00625` je Test. Die Testeinheit ist der Netzwerkseed, nicht die einzelne
Bild- oder Aktionspräsentation.

Strukturentwicklung bleibt aus, STDP bleibt unverändert an. Der vorhandene
Homöostasemechanismus wird im bisherigen Umfang `All` ein-/ausgeschaltet.
Je Variante werden alle sechs Rückkopplungs-/Plastizitätsbedingungen ausgeführt;
die Primärtests verwenden den vorher festgelegten aktiven Arm. Es gibt keine
Finalistenauswahl, Ersatzseeds oder nachträglichen Ausschlüsse schwacher Netze.

## Primärer Endpunkt: visuelle Auslesbarkeit

Die Genauigkeit stammt aus der getrennten visuellen Prüfung des externen
Auslesers. Die Differenz ist **aus minus an**, in Prozentpunkten.

| Core-Neuronen | Homöostase aus | Homöostase an | Mittlerer Verlust | Positive / negative / gleiche Paare | Einseitiges p | Bestätigt |
|---:|---:|---:|---:|---:|---:|---|
| 17 | 73,21 % | 48,21 % | 25,00 Pp. | 19 / 2 / 3 | 0,0033054 | Ja |
| 256 | 76,79 % | 45,83 % | 30,95 Pp. | 22 / 0 / 2 | 0,000017941 | Ja |
| 1.025 | 61,90 % | 39,88 % | 22,02 Pp. | 20 / 1 / 3 | 0,00077194 | Ja |
| 4.096 | 33,33 % | 20,24 % | 13,10 Pp. | 13 / 0 / 11 | 0,41941 | Nein |

Gleichstände zählen wie vorregistriert im vollständigen n als Nichtgewinne.
Deshalb genügen bei 4.096 Neuronen 13 positive und elf gleiche Paare nicht für
den konservativen Test. Der deskriptive Rückgang ist weder ein bestätigter
Effekt nach diesem Kriterium noch ein Nachweis der Gleichwertigkeit.

Die unadjustierten 95-%-Bootstrapintervalle des mittleren Verlusts betragen
17,86–32,14, 24,40–38,10, 14,88–29,17 und 7,14–19,64 Prozentpunkte in derselben
Größenreihenfolge. Sie sind deskriptiv und ersetzen nicht die Entscheidung des
familienkorrigierten Vorzeichentests. Ein positives Intervall bei der größten
Größe ändert deren vorab festgelegte Entscheidung nicht.

## Getrennter Endpunkt: externer Vorhersagefehler

Mittlerer absoluter Positionsfehler in Gitterzellen. Die Differenz ist
**an minus aus**; positive Werte bedeuten einen höheren Fehler mit Homöostase.

| Core-Neuronen | MAE aus | MAE an | Mittlere Änderung | Positive / negative / gleiche Paare | Zweiseitiges p | Änderung bestätigt |
|---:|---:|---:|---:|---:|---:|---|
| 17 | 1,3750 | 1,3854 | +0,0104 | 7 / 16 / 1 | 0,15159 | Nein |
| 256 | 1,0781 | 1,0000 | −0,0781 | 13 / 9 / 2 | 0,83882 | Nein |
| 1.025 | 0,9375 | 1,0000 | +0,0625 | 17 / 3 / 4 | 0,063915 | Nein |
| 4.096 | 0,7917 | 1,0000 | +0,2083 | 22 / 0 / 2 | 0,000035882 | Ja |

Das 95-%-Bootstrapintervall für die größte Größe beträgt deskriptiv
0,1719–0,2344 Gitterzellen. Dies bestätigt eine Änderung des **aufgezeichneten
End-to-End-Auslesefehlers** unter den festgelegten Bedingungen. Es belegt keine
isolierte Verschlechterung neuronaler Vorhersagerepräsentationen.

Beide aktiven visuellen Ausleser bestehen das frühere 0,5-Gate nur in 8/24,
9/24, 4/24 beziehungsweise **0/24** Paaren. Gerade bei 4.096 Neuronen ist die
Kalibration daher ein wesentlicher Interpretationsengpass. Die Vorhersage-
genauigkeit und die weiteren Kontrollarme stehen deskriptiv im Roharchiv;
für sie wurden keine zusätzlichen Bestätigungstests eingeführt.

## Kontrollnachweise und Aussagegrenzen

Alle 1.152 Records bestehen die vorher festgelegten technischen Prüfungen:
vollständige Wiederholung, eingefrorene Messparameter, gepaarte Anfangsnetze
und Bild-/Zeitfolgen, richtige Vorhersagezeitordnung und Schalteraudits.
Wachstum und Pruning bleiben aus; Anfangs- und Endsynapsenzahl stimmen in jedem
Lauf überein. Die eingefrorenen Kontrollendpunkte sind über die beiden
Homöostasestände identisch. Funktional schwache Netze bleiben vollständig in
den Tests enthalten.

Die Befunde betreffen die bestehende Homöostase über alle Modellpopulationen
im diagnostischen Positionssensor-Assay. Ob Sensorzellen, Core-Zellen oder
deren Zusammenspiel den Effekt verursachen, wurde hier noch nicht getrennt.
Auch Auswirkungen auf andere Aufgaben, autonome Rohpixelverarbeitung oder
allgemeine Kognition sind damit nicht bestätigt.

## Umfang, Wiederaufnahme und Archiv

**8 Varianten × 24 Seeds × 6 Kontrollen = 1.152 Studienjobs** mit vollständiger
Wiederholung ergeben **2.304 Modellausführungen**. Kalibration und Aktionsproben
sind darin enthalten. Der bestehende neuronale Programmstand und seine lokalen
Lernregeln wurden für diesen Vergleich nicht verändert.

[Archiv](results/homeostasis_confirmation_20261105/):

- [Vorregistrierung einschließlich Seed-Audit](results/homeostasis_confirmation_20261105/preregistration.json).
- [Vollständige Kontraste mit allen Seedpaaren](results/homeostasis_confirmation_20261105/confirmation.json).
- [Kompakte Auswertung](results/homeostasis_confirmation_20261105/confirmation.md)
  und [native Metriktabelle](results/homeostasis_confirmation_20261105/metrics.csv).
- [SHA-256-Prüfnachweis](results/homeostasis_confirmation_20261105/verification-receipt.json).

Die anschließende erneute Auswertung mit `confirm-homeostasis report` startete
keine neuronalen Trials. Die Vorregistrierung, die Analyseentscheidung und alle
1.152 Rohdateien behielten ihre Prüfsummen. Der SQLite-Katalog enthält 1.152
Trials und besteht `PRAGMA integrity_check`. Die gesamte Python-Prüfsuite mit
47 Tests besteht, einschließlich sechs neuer Prüfungen des Bestätigungsprotokolls.

Nativer Build-Fingerabdruck: `5b02ae55003aa897`.
SHA-256 der ausgeführten Release-Binärdatei:
`4d5604a469345ba03fb45af4d48f52edd6773e8fad9fb69d66217daff9beae08`.
Quell- und Binärdateiidentität sind bereits in der Vorregistrierung gebunden.
Die Originalbinärdatei (`reference-runner`), Quellen/Tests/Konfigurationen
(`reference-sources.zip`) und Verifikationslogs (`verification/`) sind zusätzlich
im Archiv gesichert; `source-snapshot.json` enthält die zugehörigen SHA-256-Werte.

```bash
# Vorhandenen vollständigen Lauf ohne neue neuronale Trials prüfen:
./NERVA_Modell/research/run.sh confirm-homeostasis report
```
