> Historische Dokumentation. Die eigenständigen Experimentprogramme wurden entfernt; aufgeführte alte Startbefehle sind nicht mehr verfügbar. Aktuelle Ausführung: [Research-Harness](../../README.md).

# M11-v1: Synaptische Konkurrenz im Handlungs-Wirkungs-Test

Versuchsdefinition vom **28.10.2026**, vor Implementierung und Messungen.
**Status: Mechanismus und Runner implementiert; technischer Kurzlauf auf Seed
792 und Pilot 700–702 abgeschlossen. Die Bestätigung auf 710–729 ist noch
nicht ausgeführt.** Der maschinenlesbare Vertrag liegt unter
[`m11/protocol.json`](m11/protocol.json). Die Definition ist kein Messnachweis.
[Startbefehle und Audits](m11/README.md) ·
[Technischer Kurzlauf](../../../../Messungen/Experimente/m11-smoke/summary.md) ·
[Pilotergebnisse](../../../../Messungen/Berichte/M11_EXPERIMENT.md).

## Frage und einziger Eingriff

M9 und M10-E zeigen elektrisch wirksame Gewichtsänderungen ohne zuverlässige
Handlungswirkungsvorhersage. M11 prüft die Hypothese, dass Konkurrenz um ein
festes lokales Gewichtsbudget diese Änderungen funktional nutzbar macht.
Die bisherigen Daten belegen weder diese Hypothese noch einen fehlenden Mechanismus.

| Fall | Ausgangsmodell | Neue Konkurrenz |
|---|---|---|
| `reference` | M9-v2-Kombination `weight4_tau100` | Aus |
| `competition` | Identisches Ausgangsmodell | An |

`reference` bezeichnet hier die **M9-Kombination**, nicht den M9-Fall mit
einfachen Gewichten und 20-ms-STDP. Beide Varianten werden auf den neuen Seeds
ausgeführt; historische M9-Werte ersetzen keine neue Referenz.

Fest bleiben: 32 Core-Neuronen, `effect_2ch`, deterministische Kodierung,
alle anfänglichen Core-Gewichte ×4 (Erregung und Hemmung), beide STDP-Taus
100 ms, Paarfenster 100 ms, Amplituden 0,0001/0,00012, Gewichtsgrenzen 0–2,
Core-Membran-Tau 20 ms, drei Aktionskopieziele je Taste mit Gewicht 0,9 und
50 ms Aktions-/Wirkungsabstand. Homöostase und Strukturentwicklung bleiben aus.
Der sensorische Gain stammt wie bei M9 aus dem **unskalierten Referenznetz**
des Seeds und wird für beide Varianten unverändert verwendet.

Knoten, Kanten, Anfangsgewichte, Polaritäten, Positionen, Verzögerungen und
Zufallsfolgen stimmen pro Seed überein. Vor dem Lernen gibt es keine
Normalisierung. Die Konkurrenz verwendet weder Wirkungsklassen noch Ausleser,
Belohnung, zukünftige Bilder oder die Identität einer Taste.

## Budget und Normalisierung

Für jedes Core-Zielneuron j ist I_j die beim Aufbau gespeicherte Menge seiner
aktivierten, plastischen Core→Core-Synapsen mit **erregendem Quellneuron**.
Auch hemmende Zielneuronen dürfen solche Eingänge besitzen. Sensorische,
motorische und Aktionskopie-Kanten sowie hemmende Quellen gehören nicht zum Pool.
Die Mitgliedschaft bleibt fest, da Wachstum, Pruning und Umschalten von Kanten
im Lernlauf ausgeschaltet sind.

Das Budget ist **B_j = Summe der Anfangsgewichte in I_j**, nach der ×4-Skalierung.
Die f32-Gewichte werden nach Synapsen-ID sortiert in f64 summiert. Beide
Belegungen und alle Arme erhalten dasselbe Budget. Es gibt keinen zusätzlichen
Budgetfaktor und keine Optimierung anhand von Ergebnissen. Anfangsgewichte
liegen hier zwischen 0,72 und 1,12; das Budget ist damit in den Grenzen 0–2
erreichbar. Leere Pools haben Budget null und werden übersprungen.

Nach einem Ereigniszeitpunkt, an dem STDP tatsächlich mindestens ein Gewicht
eines Pools geändert hat, wird dieser Pool **einmal** normalisiert: nach allen
Pre-/Post-STDP-Hooks desselben Zeitpunkts, vor der Spike-Weiterleitung. Die
Normalisierung findet auch nach LTD ohne nachfolgenden postsynaptischen Spike
statt. Sie wird nicht zwischen einzelnen gleichzeitigen Synapsenhooks ausgeführt.
Zielneuronen und Synapsen werden nach ID bearbeitet.

Seien v_i die vorläufigen, bereits auf 0–2 begrenzten STDP-Gewichte:

1. Solange das Budget mit den positiven v_i erreichbar ist, gilt
   **w_i = min(2, lambda × v_i)**, mit lambda so gewählt, dass die Summe B_j ist.
   Ein deterministives Active-Set-Verfahren sättigt Kanten bei 2 und verteilt
   das verbleibende Budget proportional auf die übrigen positiven Gewichte.
   Nullgewichte bleiben in diesem Fall null.
2. Reichen die positiven Kanten selbst bei Gewicht 2 nicht aus, werden sie auf
   2 gesetzt. Das Restbudget verteilt sich gleichmäßig auf die Nullgewichte.
   Bei einem vollständig leeren Gewichtsvektor erhält jede Kante B_j / |I_j|.
   Diese ausdrücklich festgelegte Wiederbelebung verwendet keine neuen Kanten.
3. Berechnung in f64, Speicherung in f32; ein Rundungsrest wird in aufsteigender
   Synapsen-ID innerhalb 0–2 korrigiert. Zulässig ist ein absoluter Budgetfehler
   von höchstens **10⁻⁶ + 10⁻⁶ × B_j**. Nichtendliche Werte, unerreichbare Budgets,
   geänderte Poolmitgliedschaft oder größere Fehler brechen den Lauf mit Audit ab.

Ohne Sättigung bleibt das Verhältnis positiver Gewichte erhalten. Sättigung
und der definierte Nullfall sind explizite Ausnahmen. Ein Pool mit nur einer
Kante kann unter festem Budget keine relativen Unterschiede lernen; seine
Häufigkeit wird berichtet, nicht durch neue Verbindungen verändert.

Die Konkurrenz ist eine Lernregel und folgt dem Lernschalter: aus in `before`,
`aged_frozen`, Kalibrierung, allen Prüfungen, Ruhe- und Resetproben. Ohne
tatsächliche STDP-Gewichtsänderung gibt es keinen Normalisierungsaufruf.
Reisende Impulse behalten ihre Emissionsamplitude; neue Emissionen verwenden
die normalisierten Gewichte. Jede STDP- und jede Konkurrenzänderung wird mit
eigener Ursache protokolliert. Das Budget ist eine postsynaptisch lokale
Modellannahme, kein Nachweis einer biologisch exakten Normalisierung.

## Kontrollarme und Messumfang

Beide Varianten verwenden unverändert `before`, `aged_frozen`, `paired` und
`scrambled`, jeweils mit normaler und umgekehrter Tastenbelegung. Der Actor
erhält nur Bilder und ausgeführte Tasten. Der Belegungswechsel verändert die
erfahrene Handlungswirkung, nicht die ausgeführte Tastenfolge.

| Stufe | Seeds | Training / Kalibrierung / Online-Prüfung | Bedeutung |
|---|---|---|---|
| Technischer Kurzlauf | 792 | 8 / 8 / 8 | Software und Invarianten |
| Pilot | 700–702 | 256 / 128 / 128 | Vollständiges Protokoll technisch prüfen |
| Bestätigung | 710–729 | 256 / 128 / 128 | 20 neue Seeds, unveränderte Bedingungen |

Pro Stufe laufen stets **beide** Varianten: 2, 6 beziehungsweise 40 Fall-/Seed-
Ergebnisse. Seedlisten und Trialzahlen stehen vor Ausführung fest. Kurzlauf und
Pilot liefern keinen Bestätigungsnachweis. Weder ein guter noch ein schlechter
Pilotwert wählt Varianten oder Seeds aus; allein technische Fehler müssen vor
der Bestätigung behoben und dokumentiert werden. Änderungen an Regel, Budget,
Endpunkten oder Kriterien erfordern eine neue Protokollversion und unbenutzte
Bestätigungsseeds. Bisherige Aufzeichnungen bleiben erhalten.

## Primäre Endpunkte und Gewichtskausalität

Die zwei primären Endpunkte entsprechen M9-v2:

- `checkpoint_probe`: A und D auf getrennten exakten Replays des Trainingsendes.
- `rest_checkpoint_probe`: unabhängige Replays nach 2 Sekunden neutralem Bild.

Kalibrierung des visuellen Spikezähl-Auslesers findet ausschließlich auf einem
eigenen Replayzweig statt. Je trainiertem Modell bleibt dieser Ausleser für alle
Eingriffe fest; er wird nach einem Reset nicht angepasst. A und D sind nur zwei
Antworten je Seed, Belegung und Endpunkt, keine 128 unabhängigen Replikate.

Je Arm, Belegung, Endpunkt und Taste entstehen unabhängige Eingriffszweige:
`sham`, `restore_core` und `restore_all_plastic`. Rücksetzung auf die jeweiligen
Anfangsgewichte erfolgt **nach eventueller Ruhe, vor der 50-ms-Baseline**, wie
in M9-v2. Dies unterscheidet sich ausdrücklich vom Eingriffszeitpunkt in M10-E.
Zellzustände, lokale Spuren, Uhren und reisende Impulse bleiben erhalten.
Anschließend bleiben STDP, Konkurrenz und andere Lernregeln aus; kein Reset
wird wieder normalisiert. Es wird kein Wirkungsbild gezeigt.

`sham` muss die ursprüngliche Checkpointprobe exakt reproduzieren. Rücksetzungen
ohne geänderte Gewichte müssen identisch zu `sham` bleiben. Unabhängige Proben
ohne Aktionskopie bleiben als Hintergrundkontrolle erhalten. Die Resetwirkung
zeigt einen Gewichtsbeitrag unter festem Zustand und Ausleser, keine alleinige
Speicherung oder Hinreichendheit der Gewichte.

## Vorab festgelegte Entscheidung

Auswertungseinheit ist der **Seed**. Alle Vergleiche sind seedgepaart; beide
Belegungen und einzelne Aktionsfenster werden nicht als zusätzliche unabhängige
Replikate gezählt. Bootstrap: 4.000 gemeinsame Resamples ganzer Seeds,
Zufallsseed 1234, deskriptive 95-%-Intervalle. Keine Auswahl des besten Zeitpunkts.

Die Familie umfasst zwei bedingungsspezifische Lernbehauptungen × zwei primäre
Endpunkte, also vier. Jeder erforderliche gerichtete Kontrast muss eine positive
mittlere Differenz, eine positive untere Bootstrapgrenze und einen einseitigen
exakten gepaarten Vorzeichentest mit **p ≤ 0,05/4 = 0,0125** erfüllen.
Gleichstände werden ausgelassen; ohne Unterschiede gilt p = 1. Alle Kriterien
einer Behauptung sind gemeinsam erforderlich, nicht alternative Erfolgswege.

**Lernen je Variante und Endpunkt** verlangt mindestens 20 Bestätigungsseeds:

1. Mittlere visuelle Auslesbarkeit im gekoppelten Arm mindestens 80 % in jeder
   Belegung separat.
2. Gekoppelte Spikezähl-Genauigkeit über 50 % und über jedem der drei
   Kontrollarme, separat in beiden Belegungen.
3. Belegungswechselrate (`flip_rate`) und gleichzeitig korrekte Antworten beider
   Belegungen (`dual_correct_rate`) jeweils über allen drei Kontrollarmen.
4. Gekoppeltes `sham` besser als `restore_all_plastic` in beiden Belegungen.

**Die Konkurrenzhypothese je Endpunkt** verlangt zusätzlich:

1. Gekoppelte Genauigkeit der Konkurrenz über der neuen Referenz, in beiden
   Belegungen separat.
2. Lernvorteil gegenüber jedem Kontrollarm größer als bei der Referenz:
   `(competition_paired − competition_control) − (reference_paired − reference_control)`
   positiv, für jeden Kontrollarm und jede Belegung.
3. Gekoppeltes `sham` der Konkurrenz besser als `restore_core` in beiden
   Belegungen. Damit ist die Wirkung der veränderten Core-Lernregel selbst
   erforderlich; ein alleiniger Effekt peripherer Plastizität genügt nicht.

Diese zusätzlichen Kontraste sind gemeinsam notwendige Teile der
Konkurrenzbehauptung, keine weiteren unabhängig auswählbaren Erfolgsbehauptungen.
Ein **erhaltener funktionaler Konkurrenzvorteil** wird nur berichtet, wenn
sämtliche Anforderungen **an beiden Endpunkten** erfüllt sind. Ein alleiniger
Sofortbefund wird ausdrücklich als solcher berichtet. Lernt nur die Referenz,
belegt das keinen Nutzen der Konkurrenz. Mehr Spikes, selektivere Gewichte oder
ein eingehaltenes Budget ersetzen keines dieser Verhaltenskriterien.

## Technische Prüfung, Archivierung und Abschluss

Vor dem ersten Lauf werden Manifest und alle effektiven Konfigurationen mit
Protokollhash und Quell-/Buildkennung gespeichert. Der spätere Runner muss
Referenzparität mit M9-v2, deterministische Replays, Poolgrenzen, Budgetfehler,
Sättigung, Nullfälle, gleichzeitige Ereignisse, LTD ohne Post-Spike und das
vollständige Einfrieren unabhängig prüfen. Gleichzeitige STDP-Hooks sehen
denselben Ablauf wie ohne Konkurrenz; normalisiert wird erst danach.

Rohdaten enthalten Spikezeiten, feste Ausleser, Anfangs-/Endgewichte sowie für
jede Normalisierung Zeit, Ziel, Pool-IDs, Budget, vorläufige und neue Gewichte,
Sättigungen und Restfehler. Berichtet werden auch leere/einkantige Pools,
visuelle Gewichtsverluste, Aktivitätsniveau und beide Endpunkte aller Arme.
Ein unabhängiger Audit muss Normalisierungen und Vorhersagen nachrechnen können.

Abgeschlossene Rohdateien werden nicht überschrieben. Infrastrukturabbrüche
dürfen denselben Job fortsetzen; neuronale Instabilität wird als Ergebnis
berichtet und nicht durch neue Seeds ersetzt. Fehlende oder ungültige Daten
verhindern einen vollständigen Bestätigungsnachweis. Danach endet M11 mit der
Entscheidung über diesen einen Eingriff; keine automatische Seed-Erweiterung,
kein nachträgliches Tuning und keine neue M10-Diagnoseserie.

S1/S2-Selektion ist nicht Teil dieses festen Vergleichs. M0-Ergebnisse ersetzen
keine Handlungswirkungsmessung. Der neue Bibliotheksmechanismus ist standardmäßig ausgeschaltet; die
Live-Modellparameter bleiben unverändert. Übertragung auf andere Aufgaben, Interferenz,
fortlaufende Plastizität und selbstständige Motorentscheidungen bleiben offen.
