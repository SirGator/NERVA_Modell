# MLD-02: vorregistrierter Kausalatlas des Lernversagens

Dieses Dokument konkretisiert den eingefügten Versuchsplan vor den Messläufen.
Ein darin zusätzlich erwähnter vollständiger Markdown-Anhang lag nicht vor.
Offene Zahlen, Stimuli, Grenzen und Entscheidungen werden deshalb hier festgelegt,
versioniert und im Laufordner vor Ausführung kryptografisch gebunden.

## Forschungsfragen und festgelegter Umfang

Untersucht werden lokale Lerninformation, funktionale Synapsenauswahl,
Gewichtsvorzeichen, Änderungsstärke, tatsächliche Beitragskonflikte,
Homöostase-/Konkurrenzinteraktion und elektrische Weiterleitung. Sensorische
Eingangsoptimierung wird ausdrücklich von einem Kandidaten für internes
Assoziationsgedächtnis getrennt.

Bekannte technische Kontrollseeds: **17027, 23003**. Vorab gewählte neue Seeds:
**31013, 31019, 31033**. Vor der Registrierung wird ihre Abwesenheit aus den
vorhandenen Manifesten und Kontrollplänen geprüft. Beide bekannten Seeds werden
zuerst ausgeführt und technisch geprüft. Erst danach startet unverändert die
Bestätigungskohorte; keine Auswahl anhand ihrer Ergebnisse.

Je Seed: 256 Core-Zellen, acht Trainingswiederholungen, 50-ms-Bildfenster,
elf Sekunden Ruhe vor jedem Abrufbild. Kein biologischer Parameter wird anhand
der neuen Seeds optimiert. Die zwei Szenarioabschnitte `atlas` und `interaction`
werden vollständig wiederholt und auf identische deterministische Ausgaben
geprüft. Fünf Seeds ergeben zehn Records und zwanzig vollständige Ausführungen.

## Acht vollständige Netzwerkgruppen im Atlas

| Gruppe | Eingriff | Was bleibt gleich? |
| --- | --- | --- |
| F | Gewichtslernen eingefroren | Native zelluläre Chemie, Eingang und elektrische Dynamik |
| B | Akkumulierende Drei-Faktor-STDP | Bisheriger Ausgangsablauf |
| G | Natürlicher Lernfaktor × **64** | Räumliche/zeitliche Verteilung und Vorzeichen des Faktors |
| C | Lernfaktor konstant +1 | Alle übrigen chemischen Rezeptorwirkungen |
| P | Unmittelbare native Nearest-Neighbor-Pair-STDP | Native Chemie bleibt aktiv; die Regel verwendet ihren Lernfaktor nicht |
| L+ | Nur positive angeforderte Beiträge | Signed Eligibility, Zeitregel, Chemie und übrige Dynamik |
| L− | Nur negative angeforderte Beiträge | Signed Eligibility, Zeitregel, Chemie und übrige Dynamik |
| D | B-Regel mit anderen vergleichbaren Bildern | Netzaufbau, Seed, Dichte, Intensität, Wiederholungen und Reihenfolge |

64 ist ein vorab fester Diagnosefaktor, motiviert durch die Größenordnungen des
vorherigen Versuchs; es ist kein neues Suchraster. Die Filter greifen an
**einzelnen angeforderten Deltas vor Restakkumulation, Gewichtsbegrenzung und
f32-Publikation** an. Gegenläufige Eligibility wird weiterhin erzeugt und zerfällt
normal. Geblockte positive/negative Masse wird eigens bilanziert. Filter ändern
weder direkt Neuronenzustände noch chemische Konzentrationen.

Alle Gruppen starten vom gleichen initialen Netzwerk. F/B/G/C/P/L+/L− erhalten
identische generierte Eingaben. D ist die ausdrücklich unterschiedliche
Bildkontrolle. Aufgenommenes und unbeobachtetes Training jedes Arms müssen im
vollständigen Replayzustand übereinstimmen. Anschließend frieren alle Abrufe die
Plastizität ein. Im Atlas bleiben Entwicklung, Homöostase und Konkurrenz wie in
der Ausgangsassoziationsstudie ausgeschaltet; ihre Wechselwirkung ist Gegenstand
des getrennten, aktiven 2×2-Abschnitts. Die restliche konfigurierte Dynamik läuft
in L+/L− genauso weiter wie in B; es wird kein Teilnetz isoliert.

## Bilder, neue Cues und Referenzkonstruktion

A/B verwenden die bisherigen acht hellen Pixel je Vollmuster: sechs gemeinsame
Pixel `(1,1),(2,1),(1,2),(2,2),(1,3),(2,3)` und die beiden klassenspezifischen
Pixel A `(5,5),(6,5)` bzw. B `(5,1),(6,1)`. Die Trainingscues sind jeweils die
zwei klassenspezifischen Pixel. Labels gehen nicht in das neuronale Lernen ein.

D verwendet disjunkte Pixel bei gleicher Dichte: gemeinsame Pixel
`(1,5),(2,5),(1,6),(2,6),(3,5),(3,6)`, einzigartige A-Pixel `(5,3),(6,3)`,
einzigartige B-Pixel `(5,7),(6,7)`. Kein D-Trainingspixel liegt auf einem der
ursprünglichen A/B-Trainingspixel.

Die feste Abrufsequenz ist Schwarz, Voll-A, Voll-B, Original-Cue-A,
Original-Cue-B, neuer Cue A mit nur `(5,5)`, neuer Cue B mit nur `(5,1)`,
neuer Cue A mit nur `(6,5)`, neuer Cue B mit nur `(6,1)`. Die vier neuen
Cue-Varianten werden **nicht** zur Referenzkonstruktion verwendet.
Jeder Probe geht die gleiche Ruhe voraus; sie ist kein behaupteter harter
Zustandsreset. Spätere Antworten dürfen aufgrund vorheriger Gewichtswirkungen
unterschiedliche Vorgeschichten haben.

Die feste künstliche Sensor+Core-Referenz verwendet weiterhin
`sensory_and_recurrent`, Dosis 1, und die vertauschte Dosis als Negativkontrolle.
Zusätzlich wird eine ausschließlich rekurrente Referenzrichtung nach der bereits
vorhandenen Methode `recurrent_two_hop`, ebenfalls Dosis 1 und vertauscht, geprüft.
Es gibt keine weitere Dosis- oder Methodensuche. Die Richtungen werden nur aus
Vollmustern und Originalcues konstruiert. Fehlende rekurrente Referenz ist kein
Nachweis eines unmöglichen rekurrenten Pfads.

Referenzerfolg verlangt wie zuvor aktive und korrekte Originalcues, positive
Gewinne gegenüber Anfangsgewichten, Überlegenheit gegenüber der vertauschten
Kontrolle und keine gesteigerte cuefreie Core-Aktivität. Neue Cues werden separat
bewertet. Das Referenzgewicht darf bei Konstruktion an [0,2] begrenzt werden;
seine **tatsächlich installierte** Richtung ist die spätere Normierungsgrundlage.

## Gleicher vollständiger Zustand, unterschiedliche Gewichte

Alle folgenden Transplantationen rekonstruieren exakt den trainierten B-Zustand,
einschließlich Warteschlangen, Uhren, Rezeptorwirkungen, Tags und Restbeträgen.
Ein Gewichts-Rücktausch und `same_replay_state` prüfen die Isolierung.

Verglichen werden B/G/C/P/L+/L−/D, negiertes B, kumulierte positive bzw. negative
**tatsächlich ausgeführte B-Beiträge**, die feste Referenz, deren rekurrente
Variante sowie getrennte Sensor- und Core-Anteile von B und Referenz. Ein Core-
Anteil umfasst Verbindungen mit verarbeitender Quelle einschließlich Core→Motor;
der Sensoranteil hier Sensor→Core. Die positive Beitragssumme ist nicht dasselbe
wie der positive Anteil des endgültigen Nettodeltas und wird nicht damit ersetzt.

Primäre gemeinsame Stärke ist die sichtbare L2-Norm der verifizierten Referenz.
Sekundär wird eine gemeinsame zulässige Stärke verwendet: das Minimum aus dieser
Referenznorm und allen nichtnull Richtungsradien. Alle ausgeführten normierten
Arme verlangen nach f32-Publikation höchstens 1 % Normabweichung und Kosinus
≥ 0,999. Unzulässige, nullförmige oder numerisch untreue Arme bleiben explizit
unausgeführt; Normierung wird nicht durch Clipping nachträglich gedreht.
Wenn die Referenz bei der kleineren gemeinsamen Stärke ihre Funktion verliert,
belegt dieser Vergleich keine funktional wirksame Ersatzrichtung.

Zusätzlich wird die Referenz bei natürlicher B-Stärke getestet. Der tatsächliche,
unverstärkte B-Core-Anteil wird mit ursprünglichen Sensorprojektionen abgerufen.
D wird für den Bildspezifitätsvergleich auf genau dessen zulässige Stärke gebracht.
Ein fehlender vergleichbarer D-Arm bleibt fehlende Evidenz.

Für bis zu acht Referenzsynapsen in aufsteigender ID wird jeweils nur deren
Referenzdelta zurückgenommen. Alle anderen Referenzgewichte bleiben identisch.
Nur wenn dadurch der zuvor erfüllte Referenzerfolg scheitert, heißt diese Kante
**für genau diese Referenzkonfiguration notwendig**. Andere Referenzkanten sind
nicht automatisch notwendig; weitere Kanten bleiben bei mehr als acht explizit
ungeprüft. Diese Ablationen werden nicht als gleichnormierte Richtungsvergleiche
interpretiert.

## Lokale Nachrechnung und Information

Das vollständige B-Ereignisband enthält native verzögerte Integrationsgrenzen,
chemische Proben, synaptische Prä-Ankünfte und Post-Spikes. Native lokale Hooks
rechnen Original, Faktor +1, Gain 64, Pair-STDP und beide Vorzeichenfilter ohne
erneute Netzwerksimulation nach. Die Original-Nachrechnung muss Gewichte,
Eligibility, Prä-Traces und Restbeträge exakt reproduzieren.

Die Offline-G- und -C-Richtungen werden zusätzlich am selben B-Checkpoint
normkontrolliert abgerufen. Ihr Vergleich trennt die chemische Gewichtung auf
**derselben** Historie von unterschiedlichen späteren Trainingstrajektorien.
Die vollständigen Live-Gegenversuche bleiben erforderlich. Pair-STDP ist dabei
kein reiner Chemie-Gegenversuch: Aktualisierungsart und Zeitpunkt unterscheiden sich.

Für jede Kante der künstlichen Referenz werden Plastizitätszulassung, tatsächliche
Prä-Ankünfte, Post-Spikes und passende positive/negative Nearest-Neighbor-Paare
im nativen 100-ms-Fenster gezählt. Gleichzeitige Prä-Ankünfte kommen wie in der
Runtime vor Post-Hooks. Regelverbot, fehlende Ereignisse und gegenläufige
angeforderte Masse sind getrennte Befunde. Die Auswertung behauptet keine
fehlende Information nur aus einer unveränderten f32-Gewichtszahl.

Alle Synapsen erhalten Statistiken. Native Detailhistorien konzentrieren sich auf
die acht größten B-Nettodeltas plus bis zu acht Referenzkanten, gemeinsam für die
Gruppen; die Endpunkte werden mit beobachtet. Gewichtsereignisse, Restbeträge und
unerfüllte Anforderungen werden getrennt gespeichert. Die elektrischen Proben
bilanzieren Ankünfte, Refraktärzeit, subthreshold Batches, kontinuierliche
Potenzialänderungen, Schwellenabstand und echte Spikefolgen. Gespeicherte
Potenziale und lokale Zeitstempel werden nicht als kontinuierlich gleichzeitige
Messung ausgegeben.

## Aktiver 2×2-Vergleich auf gemeinsamer entwickelter Struktur

Ein eigener Abschnitt rekonstruiert eine **gemeinsame einsekündige native
Entwicklungsphase** mit Lernen, Homöostase und Entwicklung: zwanzig 50-ms-Fenster
im Zyklus D-Voll-A, D-Voll-B, Schwarz, Schwarz. Formationen und Entfernungen werden
gemessen; ohne tatsächliche Strukturänderung ist dieser technische Versuch ungültig.
Danach wird Entwicklung eingefroren. Vor jeder Verzweigung muss der vollständige
Zustand identisch sein; dies geht über gleiche Synapsenzahlen hinaus.

An diesem eingefrorenen Zustand werden bei K=1 die aktuellen Konkurrenzpools
und Budgets erfasst. Eine neue ausdrückliche Checkpoint-API erlaubt danach
zelluläre Homöostase auf **festem Graphen**; erneute Entwicklung ist verboten.
Die bisherigen Konstruktor-Defaults und Schutzregeln werden nicht abgeschaltet.
Die vier Trainingsarme sind H0/K0, H0/K1, H1/K0, H1/K1. Für beide K1-Arme müssen
Poolmitgliedschaft und Ausgangsbudgets exakt gleich sein. Die vorhandene
Konkurrenz umfasst aktivierte plastische erregende Core→Core-Kanten;
Sensorprojektionen werden nicht normalisiert. Homöostase nutzt die native
Population `All`. Alle Arme behalten
während des Trainings dieselbe Struktur.

Der primäre Interaktionsterm für jeden der sechs Cue-Kontraste ist
`F(1,1) − F(1,0) − F(0,1) + F(0,0)`. F bezeichnet die feste Zielmarge des
jeweiligen eingefrorenen Endzustands. Dies ist die Gesamtwirkung der beiden
Mechanismen einschließlich zellulärer Zustandsänderungen. Eigene Gewichts-Resets
weisen zusätzliche Gewichtsanteile aus; sie ersetzen nicht den Gesamtvergleich.
Konkurrenz- und lokale Gewichtsdeltas werden separat bilanziert, ebenso
Homöostaseänderungen je Neuron. Eine reine Zahl abweichender Endgewichte gilt
nicht als Mechanismeninteraktion.

## Vorab festgelegte Entscheidungsregeln

Alle funktionalen Vergleiche verwenden `epsilon = 1e-9`. Ein fehlender oder
unzulässiger Kontrollarm ergibt **nicht bewertet**, keinen negativen Nachweis.

| Frage | Kriterium und zulässige Aussage |
| --- | --- |
| Lokale Information | Nur an einer verifizierten Referenz: Regel schließt Kante aus oder keine zugelassenen Paare im festen B-Band; die aktuelle Regel kann sie unter dieser beobachteten Historie so nicht lernen. Notwendigkeit stammt ggf. aus Einzelablation. |
| Richtung | Vorhandene Paare/Anforderungen, aber zur Referenz entgegengesetztes Gewichtsvorzeichen; ein lokaler Engpass gegenüber dieser Referenz, keine universell falsche Richtung. |
| Stärke | G bei Faktor 64 hilft nicht und B scheitert bei verifizierter gleicher Referenzstärke, während E dort Original- und neue Cues verbessert: Stärke allein reicht in diesem geprüften Bereich nicht. |
| Beitragskonflikt | Gegenläufige Masse je Synapse ist `min(positive_actual, abs(negative_actual))`; Anteil ausgelöschter Masse ist `2*Summe(min)/Summe(abs)`. Schaden wird erst erwogen, wenn die reine positive Beitragsrichtung bei gleicher primärer Norm alle sechs Cues verbessert und B übertrifft, die negative Richtung aber nicht. Live-L+-Wirkung wird getrennt berichtet. |
| Chemische Verteilung | Offline-G und Offline-C auf derselben B-Historie unterscheiden sich bei gleicher gültiger primärer Norm in mindestens einem Cue-Kontrast über epsilon: Kopplung beeinflusst mehr als Gesamtstärke, ohne Confound verschiedener Spikehistorien. |
| Elektrische Weiterleitung | Gespeicherte Potenziale ändern sich, Core-/Motor-Abrufspikes nicht: Schwellen-/Timingengpass als Kandidat; elektrische Bilanzen grenzen Refraktärzeit und fehlende Schwellenüberschreitung ein, beweisen aber keine schädliche Einzelhemmung. |
| Nichtadditivität | Betrag des festgelegten 2×2-Kontrasts über epsilon auf einem Seed: deterministisch nichtadditive Wirkung in diesem Netz; Vorzeichenreplikation auf drei neuen Seeds separat. |
| Gedächtniskandidat | Unverstärktes B und unverstärktes B-Core bei zurückgesetzten Sensorprojektionen verbessern alle sechs Original-/neuen Cues; D-Live besteht nicht dieselben Gates, und ein gleichstarker D-Gewichtseingriff bleibt bei allen sechs Gewinnen unter B-Core. Sonst kein Gedächtniskandidat aus diesem Protokoll. |

Drei neue Seeds liefern eine deskriptive Replikation, keinen konventionellen
5-%-Signifikanznachweis. Primäre Kriterien gelten als repliziert, wenn alle drei
bewertbaren neuen Seeds sie erfüllen. Gemischte und nicht bewertbare Befunde
werden getrennt ausgewiesen. Es gibt keine nachträgliche Auswahl erfolgreicher
Cues, Seeds, Normen oder Referenzdosen.

## Budget, Archiv und Befehle

Höchstens 80 Trainingsrekonstruktionen je Atlas-Ausführung; der Interaktions-
abschnitt benötigt zehn gemeinsame Entwicklungsrekonstruktionen und acht
anschließende Trainingsdurchläufe, zusammen 18. Mit Replay und fünf Seeds liegt
die obere Grenze bei **980** Durchläufen. Identische Gewichtsvektoren teilen
vollständige Beobachtungen. Höchstens zwei Seeds laufen parallel; zunächst nur
die bekannten, danach die drei neuen. Der feste Wall-Zeitrahmen beträgt 7.200 s.
Trainingsbänder sind auf zwei Millionen Quellzeilen begrenzt, Detailhistorien
auf 500.000 Zeilen je Arm; die jeweiligen Fenster- und elektrischen Auditbudgets
bleiben aktiv. Überläufe sind ungültige Trials und werden nicht still gekürzt.
Interaktionsbeispiele sind auf je 64 begrenzt und als Beispiele markiert;
vollständige Synapsen-/Neuronenaggregate werden erhalten.

```bash
# Technische Prüfung mit 16 Core-Zellen:
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/mld02_smoke.toml --output /tmp/nerva-mld02
# Lokale Vorregistrierung mit Quellenarchiv und unveränderlicher Binärdatei:
./NERVA_Modell/research/run.sh mld02 plan NERVA_Modell/research/config/mld02_20261112.toml
./NERVA_Modell/research/run.sh mld02 run NERVA_Modell/research/config/mld02_20261112.toml
./NERVA_Modell/research/run.sh mld02 report NERVA_Modell/research/config/mld02_20261112.toml
```

`preregistration.json` und SHA-256 binden Spezifikation, ausführbaren Runner,
Protokoll, unabhängigen Nachrechner, Quellenarchiv und alle Seed-Manifeste vor
Ausführung. Die Kohorten verwenden genau diese archivierten Programme. Der
unabhängige Python-Nachrechner kontrolliert native Cue-Margen/Gewinne, gesetzte
Gewichte, Norm/Richtung, lokale Ereignisgelegenheiten, Einzelablationen und den
Interaktionsterm. Rohdaten bleiben gemäß [ARCHIVING.md](ARCHIVING.md) unter
`research/results/mld02_20261112/`; Quellcode und kompakte Berichte werden
versioniert. Bestehende Messarchive werden nicht überschrieben.
