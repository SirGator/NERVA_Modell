# Integrierter Hippocampus-/Neuromodulations-Prototyp (09.11.2026)

Der ausführbare Prototyp erweitert MAINs vorhandenes räumliches Nervensystem.
Cortex-, DG-, CA3- und CA1-ähnliche Profile sind räumliche Einflüsse auf dieselbe
Neuronendynamik; es gibt keinen programmierten Gedächtnisalgorithmus.
Die Parameter sind synthetische Modellparameter, keine physiologische Kalibrierung.

## Vorab festgelegter Funktionspilot

`./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/hippocampal_probe.toml`

Der bestehende Harness prüft zwei feste Seeds (42, 73), 128 Processing-Zellen,
8 Wiederholungen beider Bilder und fünf Kontrollen: Cortical Core, weiche
Arealprofile mit Chemie, harte Profile mit Chemie, weiche Profile ohne Chemie,
weiche Profile mit Chemie und angehaltenem Gewichtlernen. Strukturentwicklung,
Homöostase und Aktionsrückmeldung sind in allen Pilotarmen abgeschaltet.
Die aktiven Produktstandards bleiben separat ausführbar.

Zwei 8×8-Helligkeitsbilder teilen sechs von acht weißen Pixeln. Je ein Teilbild
behält nur die zwei eigenen, im anderen Bild schwarzen Pixel. Das Modell erhält
ausschließlich Bilder über den bestehenden deterministischen 120-Hz-Sensor.
Kein Label, Score, Modulatorpuls oder Sollaktion wird von diesem Prüfer eingespeist.
Beide Vollbilder erscheinen achtmal, in einer vorab seedabhängig balancierten
Reihenfolge, jeweils mit einem schwarzen 50-ms-Frame danach.

Vor und nach Training werden Gewichte eingefroren und vier native Core-Zählvektoren
gemessen: Vollbild A, Vollbild B, Teilbild A, Teilbild B. Vor jedem Bild liegen
elf Sekunden Schwarzbild, verarbeitet in 220 regulären 50-ms-Schritten. Das übersteigt die zehn Sekunden Aufbewahrung der
anfänglichen Chemiepulse; autonome Aktivität kann jedoch neue Pulse erzeugen.
Der Prüfer protokolliert die Core-Spikes der gesamten Ruhe und prüft unveränderte
Lernparameter während jeder Probe. Alle Rohzählvektoren bleiben erhalten.

Separation ist 1−Cosinus(Vollbild A, Vollbild B). Der Ergänzungskontrast eines
Teilbildes ist dessen Cosinusähnlichkeit zum eigenen Vollbild minus Ähnlichkeit
zum anderen Vollbild. Erwerb ist der Nachher-/Vorherunterschied. Das ist eine
feste externe Messung; die Ähnlichkeitsrechnung steuert keine Neuronen.
Schweigen erhält keine Ähnlichkeitsgutschrift und kann das Kandidatengate nicht bestehen.

Das explorative Kandidatengate verlangt bei beiden Seeds aktive Reaktionen,
positive Ergänzungskontraste und positiven Erwerb für beide Bilder sowie für
beide Bilder einen Erwerbsvorteil über Cortical-, Frozen- und No-Chemistry-Kontrolle.
Die Separation darf gegenüber Cortical nicht sinken. Beide Ergänzungskontraste
und die Separation dürfen gegenüber Sharp nicht sinken. No-Chemistry behält
denselben Drei-Faktoren-Modus; ausschließlich die chemische Wirkung entfällt.
Exakter Replay ist Pflicht. Dieses Gate ist kein Nachweis isolierter Gewichtserinnerung:
STP und Chemiezustände werden nicht transplantiert, und es gibt noch keine
unabhängige Bestätigung oder Gedächtnisaufgabe in einer externen Welt.
`functional_memory_demonstrated` bleibt deshalb ausdrücklich falsch.

## Biologische Motivation und Modellgrenzen

Die starke frequenzabhängige Fazilitation an Moosfasersynapsen motiviert den
lokalen Kurzzeitmechanismus, ohne seine Zahlen zu kalibrieren:
[Salin et al., 1996](https://doi.org/10.1073/pnas.93.23.13304).
Die signierte zeitlich abklingende Eligibility ist eine abstrahierte lokale
Variante eines Drei-Faktoren-Ansatzes:
[Izhikevich, 2007](https://www.izhikevich.org/publications/dastdp.pdf).
Dessen globale Belohnungsarchitektur wird nicht übernommen. MAINs Quellen
setzen Chemikalien durch eigene Spikes frei; Rezeptoren wirken lokal und verschieden.
Eine experimentelle Motivation für synapsenspezifische dopaminerge Modulation
liefert [Otmakhova & Lisman, 1996](https://pmc.ncbi.nlm.nih.gov/articles/PMC6579102/).
Daraus folgt weder ein biologisch validiertes MAIN-Modell noch dessen Gedächtnisleistung.
