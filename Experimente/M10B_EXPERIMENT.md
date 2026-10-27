# M10-B: Wirkungsspezifität oder gemeinsamer Decoderbias?

Auswertungsdefinition vom **27.10.2026**, nach Kenntnis von M9/M10 und vor
Ausführung dieser Zusatzdiagnose. **Retrospektive explorative Auswertung der
vorhandenen Daten, keine neue unabhängige Bestätigung.**

**Status: vollständig ausgewertet und auditiert.**
[Ergebnisse](../../Messungen/M10B_EXPERIMENT.md).

M10 zeigt, dass A und D neuronal verschieden aktiv sind, aber häufig derselben
visuellen Klasse zugeordnet werden. M10-B trennt zwei Möglichkeiten: Die
Differenz kann bereits in die richtige Wirkungsrichtung zeigen, während ein
gemeinsamer Offset beide Antworten auf derselben Seite der Klassengrenze hält;
oder die Differenz selbst passt nicht zur visuellen Wirkungsrichtung.

## Feste Daten und Auswertung

Alle vier M9-Bedingungen, Seeds 610–629, vier Arme, beide Belegungen und beide
primären Zeitpunkte werden verwendet. Pro Zustand werden `sham`, `restore_core`
und `restore_all_plastic` verglichen. Die 80 Quell-JSONs und das Quellmanifest
werden vor Auswertung über SHA-256 fixiert. Keine Fälle oder Merkmale werden
anhand des neuen Ergebnisses ausgewählt.

Verwendet werden ausschließlich die gespeicherten visuellen Kalibrierparameter
`mean`, `scale` und die beiden Klassenmittelpunkte sowie vorhandene
Aktions-Spikezählvektoren. Es gibt keinen neuen neuronalen Lauf, keine Änderung
der Lernregeln und kein Anpassen eines Auslesers an Aktionslabels.

## Definierte Messgrößen

Die Aktionsvektoren werden mit dem ursprünglichen visuellen Mittelwert und der
ursprünglichen Skalierung standardisiert. Der Einheitsvektor vom negativen zum
positiven visuellen Klassenmittelpunkt definiert die feste Wirkungsachse. Der
Projektionsnullpunkt liegt genau zwischen diesen Mittelwerten.

- Ursprüngliche Einzelantworten und Genauigkeiten werden exakt rekonstruiert.
- Der Mittelwert beider A-/D-Projektionen beschreibt ihren gemeinsamen Offset.
- Die Differenz der Projektionen, mit der tatsächlichen D-Wirkung vorzeichen-
  korrigiert, misst die Richtung der Wirkungszuordnung. Zusätzlich wird der
  Kosinus zur visuellen Achse gespeichert.
- Eine diagnostische Paar-Zentrierung zieht den gemeinsamen Offset ab. Die
  Rangfolge entscheidet, welches der beiden bekannten balancierten Beispiele
  zur positiven beziehungsweise negativen Klasse passt. Richtige Rangfolge
  zählt 1, falsche 0, Projektionsgleichstand 0,5.
- Besonders gezählt werden Paare mit richtiger Richtung, die der ursprüngliche
  Ausleser trotzdem gleich klassifiziert.

Numerischer Gleichstand: Betrag des signierten Kontrasts höchstens
`1e-12 × max(1, |scoreA|, |scoreD|, ||zD-zA||)`. Entartete visuelle Achsen werden
explizit ausgewiesen und in der Rangfolge als Gleichstand behandelt.

## Kontrollen, Inferenz und Grenzen

Gekoppeltes Training wird mit Zufall, `before`, `aged_frozen` und `scrambled`
verglichen, außerdem mit beiden Gewichtsrücksetzungen bei unverändertem
Ausleser. Die Vergleiche erfolgen je Belegung und Zeitpunkt separat mit
deskriptiven seedgepaarten Bootstrapintervallen. Es gibt keinen neuen
Signifikanz- oder Bestätigungsschwellwert aus diesen bekannten Seeds.

**Die Paar-Rangfolge ist keine unabhängige Einzelvorhersage:** Sie benutzt beide
Aktionsantworten aus identischen Zuständen und die Zusatzinformation, dass eine
positive und eine negative Wirkung vorliegen. Eine Verbesserung dieser Diagnose
allein bedeutet deshalb kein funktional gelerntes Verhalten des Netzwerks.
Auch höhere Werte in ungepaarten oder eingefrorenen Kontrollen würden gegen
eine Interpretation als spezifischen Lernerfolg sprechen.

[Ausführung](m10b/README.md). Der Audit prüft Quellprüfsummen, ursprüngliche
Vorhersagen und die algebraische Gleichheit zwischen Zentroidabständen und
Projektionsscores. Alle Originaldaten bleiben erhalten.
