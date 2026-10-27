# NERVA: Forschungsfahrplan

Stand **28.10.2026**. Ziel ist zunächst ein kontrollierter Nachweis, dass lokale
Lernregeln einen erfahrenen Zusammenhang verändern und das Netzwerk diese
Änderungen anschließend funktional nutzt. Eine sichtbare Gewichtsänderung oder
mehr Aktivität allein genügt nicht. Die Aussagen gelten jeweils für die geprüfte
Aufgabe und die untersuchten Bedingungen; universelle Minimalität folgt daraus nicht.

## Forschungsstand und Reihenfolge

M7 weist erfahrungsabhängige Gewichtsänderungen und elektrische Wirkungen nach.
M8 zeigt getrennt stärkere rekurrente Weiterleitung und stärkere zeitliche
Gewichtszuordnung, aber keinen belastbaren funktionalen Lernnachweis.

| Meilenstein | Forschungsfrage | Stand |
|---|---|---|
| M9 | Ermöglichen stärkere Rekurrenz und längere STDP-Zeitkonstanten gemeinsam funktionales, gewichtsabhängiges Lernen? | Abgeschlossen; auf 20 neuen Seeds kein funktionaler Lernnachweis. [Ergebnis](../../Messungen/M9_EXPERIMENT.md) |
| M10 | An welcher Stelle wird erfahrungsabhängige Information gespeichert, weitergegeben oder ununterscheidbar? | Retrospektive Diagnose abgeschlossen: Gewichte wirken auf Eingänge/Potenziale und teilweise auf Spikes, selten auf die Entscheidung. [Ergebnis](../../Messungen/M10_EXPERIMENT.md) |
| M11 | Macht lokale synaptische Konkurrenz vorhandene Gewichtsänderungen funktional nutzbar? | Vorgeschlagen: eine Änderung, zwei Varianten, technischer Pilot und 20 neue Seeds; noch nicht implementiert oder ausgeführt |
| M12 | Können lokale Entwicklungsregeln funktionale Verschaltungen hervorbringen? | Vorgeschlagen; vorhandenes Wachstum/Pruning gezielt funktional prüfen |
| M13 | Ist für die geprüfte Aufgabe eine andere zeitliche Verarbeitung nötig? | Optional; erst nach konkreter Diagnose |

Diese Nummern definieren Forschungsrichtungen. M11–M13 sind keine bestehenden
Messreihen und keine automatisch festgelegten Implementierungsaufträge.

## M9: vorhandene Mechanismen kombinieren

[Vollständiger vorab festgelegter Plan](M9_EXPERIMENT.md) ·
[Ausführung](m9/README.md).

Vier Bedingungen: Core-Gewichte ×1/×4 und gemeinsame STDP-Zeitkonstanten
20/100 ms. Alle übrigen Einstellungen bleiben fest, einschließlich ausgeschalteter
Homöostase und Strukturentwicklung. Stärkere Gewichte betreffen auch Hemmung;
längere STDP-Zeitkonstanten verbreitern auch Depression.

Ein kleiner technischer Test prüft Software und Audits. Danach bleiben die vier
Bedingungen im Pilot und auf **20 neuen Bestätigungsseeds** unverändert. Die
Bestätigung wird weder nach guten Pilotfällen ausgewählt noch nachjustiert.

Primär sind Vorhersagen aus unabhängigen Trainingscheckpoints, sofort und nach
2 Sekunden Ruhe. Kontrollen: vor Training eingefroren, mit Erfahrung gealtert
eingefroren und mit entkoppelter Aktions-/Wirkungserfahrung trainiert; beide
Tastenbelegungen. Zusätzliche Replayzweige vergleichen trainierte Gewichte mit
Core-Rücksetzung und vollständiger Rücksetzung plastischer Gewichte.

Ein vollständiger Nachweis verlangt den Verhaltensvorteil gegenüber Zufall und
Kontrollen sowie einen kontrollierten Verlust nach Gewichtsrücksetzung.
Gewichtskausalität wird bei festem Ausgangszustand und festem Ausleser geprüft;
das beweist weder ausschließlich synaptische Speicherung noch autonome Motorik.

## M10: Signalweg lokalisieren

Die Diagnose folgt dem Weg:

`ausgeführte Aktion → Aktionskopie → Core-Aktivierung → lokale Gewichtsänderung
→ Wirkung auf nachgeschaltete Zellen → unterscheidbare Antwort`

Für jede Stufe werden geeignete vorhandene Messungen aus M5/M7/M8 wiederverwendet
und bei Bedarf um passive Aufzeichnungen ergänzt: Spikezeiten, Membranpotenziale,
synaptische Eingangsbeiträge und Gewichtsänderungen. Kontrollierte Eingriffe auf
identischen eingefrorenen Replayzuständen sollen mögliche Ursachen trennen.

Bei negativem M9 wird insbesondere geprüft, ob Information zu schwach gespeichert,
elektrisch nicht weitergegeben oder vom gewählten Ausleser nicht unterschieden
wird. Auch Zeitfenster, sensorische Depression und Anfangsverschaltung bleiben
mögliche Ursachen. Ein negatives Ergebnis benennt nicht automatisch einen
fehlenden biologischen Mechanismus.

Bei positivem M9 lokalisiert M10 die tragenden Verbindungen und prüft die
Reichweite des Befunds. Das konkrete Diagnoseprotokoll wurde anhand der M9-Daten
vor den zusätzlichen Replaymessungen festgelegt.

Die [abgeschlossene M10-Diagnose](../../Messungen/M10_EXPERIMENT.md) verwendet
alle 80 M9-Quellfälle und passive Kombinations-Replays auf denselben 20 Seeds.
In 160/160 gekoppelten Aktionsproben ändern Core-Gewichte Eingänge und synchrone
Potenziale, in 61/160 das Spikezählmuster und in 5/160 die feste Entscheidung.
Das lokalisiert begrenzte Übertragung an zwei Übergängen und bestätigt kein
funktionales Lernen. Wirkungsspezifität und die Passung von Aktions- zu visuellen
Zielmustern bleiben vor einer biologischen Erweiterung gezielt zu untersuchen.

Die anschließende [M10-B-Diagnose](../../Messungen/M10B_EXPERIMENT.md) entfernt
einen gemeinsamen A-/D-Projektionsoffset rein für die Auswertung. Die Kombination
erreicht dabei nur 42,5 % richtige Paar-Rangfolgen sofort und 45,0 % nach Ruhe.
Eine reine Biaserklärung reicht nicht aus; die Wirkungsausrichtung der
synaptischen Beiträge und ihrer Weiterverarbeitung bleibt zu lokalisieren.
Die Paar-Rangfolge ist kein neuer unabhängiger Einzelvorhersage- oder Lernnachweis.

Die [M10-C-Bilanz](../../Messungen/M10C_EXPERIMENT.md) zeigt anschließend
23/40 negative Eingangsprojektionen auf der festen neuronalen Auslesergewichtung
je Zeitpunkt. Filterung und Spikebildung ändern nur wenige Vorzeichen; die
Eingänge enthalten aber schon rekurrente Rückkopplung. Damit bleibt vor einer
neuen Lernregel ein kontrollierter Eingriff in rekurrente Beiträge identischer
eingefrorener Checkpoints sinnvoll. Dies ist eine Diagnose derselben 20 Seeds
und keine eigenständige Eingangs-/Potenzialklassifikation.

Die [M10-D-Eingriffe](../../Messungen/M10D_EXPERIMENT.md) sind ebenfalls auf
allen 20 bekannten Seeds abgeschlossen. Ohne rekurrente Hemmung verdoppelt sich
die Aktivität ungefähr, ohne beidseitigen Vorhersagevorteil. Ohne Erregung
verbleiben drei Spikes/Aktion und ebenfalls kein Vorhersagegewinn. Vor einer
neuen Lernregel kann ein gekreuzter Gewichtsreset-/Übertragungsversuch direkte
Aktionsantwort und Beitrag gelernter Core-Gewichte weiter trennen. Die
getrennten bisherigen Eingriffe belegen deren Wechselwirkung noch nicht.

Die [M10-E-Kreuzung](../../Messungen/M10E_EXPERIMENT.md) ist auf allen 20
bekannten Seeds abgeschlossen. Bei aktiver Rekurrenz verändert der Core-Reset
61/160 gekoppelte Spikezählmuster und 5/160 Vorhersagen; ohne rekurrente Erregung
verschwindet der Gewichtseinfluss. Ohne Hemmung wird er stärker, ohne beidseitigen
Vorhersagevorteil. Die Rücksetzung erfolgt nach identischer Baseline und ist
methodisch von den früheren Rücksetzungen davor zu unterscheiden.

**Planungsstand 28.10.2026:** M10 ist mit M10-E abgeschlossen. Der zunächst
vorgeschlagene Gewichtstausch wird vorerst zurückgestellt; weitere M10-
Zusatzdiagnosen sind nicht geplant. Die Befunde motivieren einen begrenzten
Lernversuch, beweisen aber keinen bestimmten fehlenden Mechanismus.

## M11: Synaptische Konkurrenz als begrenzter Lernversuch

Vorgeschlagen wird genau eine Änderung: Die eingehenden erregenden Core-Synapsen
jedes Neurons teilen sich ein festes Gewichtsbudget. Nach lokalen STDP-
Änderungen werden ihre relativen Stärken auf dieses Gesamtbudget normalisiert.
Die Hypothese ist, dass dadurch selektivere, funktional nutzbare Antworten
entstehen. Die bisherigen Diagnosen belegen diese Hypothese noch nicht.

Zwei Varianten vergleichen: bisheriges Netzwerk und dasselbe Netzwerk mit
Konkurrenz. Aufgabe, Ausleser und Kontrollarme bleiben gleich: eingefrorenes
Lernen, entkoppelte Erfahrung und beide Belegungen. Vor Umsetzung müssen die
Normalisierung, das Budget und die Erfolgskriterien konkret festgelegt werden.
Ein Nachweis verlangt bessere Vorhersagen als Zufall und Kontrollen in beiden
Belegungen, Erhalt nach Ruhe und einen Verlust des Vorteils nach Gewichtsreset.

Geplant sind ein technischer Pilot und anschließend ein unveränderter Vergleich
auf 20 neuen Seeds. Danach wird über diese Hypothese entschieden; eine weitere
Reihe von Zusatzdiagnosen ist nicht vorgesehen. Dies ist Forschungsplanung,
kein bereits implementierter Mechanismus und kein Ausführungsauftrag.

## M12: selbstständige Netzwerkentwicklung

Die Bibliothek besitzt bereits lokalen Wachstumsbedarf, aktivitätsabhängige
Partnersuche, exzitatorische Nutzenbewertung und Pruning. Technische M1-Tests
prüfen strukturelle Anpassung bei vorgegebener Aktivität; funktionales Lernen
unter sensorischen Erfahrungen ist damit nicht nachgewiesen.

M12 soll schwach und lokal vernetzte räumliche Anfangsnetze untersuchen und
Spontanaktivität, sensorische Erfahrung, Lernen, Wachstum und Pruning kontrolliert
kombinieren. Kontrollen mit fester Struktur und getrennten Mechanismen müssen
zeigen, ob Entwicklung die geprüfte Funktion verbessert. Ein-/Ausgangsgrenzen
und Anfangsbedingungen bleiben explizite Modellannahmen.

## M13: zeitliche Verarbeitung bei nachgewiesenem Bedarf

Erst wenn die bisherigen Diagnosen einen zeitlichen Engpass erkennen lassen,
Kurzzeitplastizität oder alternative lokale Lernregeln einzeln vergleichen.
Vorhandene Aktivitäts-, STDP- und Nutzen-/Eligibility-Spuren werden dabei von
synaptischer Fazilitation oder Depression unterschieden. Eine neue Regel braucht
eine konkrete Hypothese, Kontrollen und vorab festgelegte Endpunkte.

Evolution, neue Neuronentypen, Dopaminmodelle und komplexe Dendriten sind vorerst
keine Arbeitspakete. Die nächste Entscheidung richtet sich nach einem gemessenen
Engpass und der einfachsten kontrolliert prüfbaren Änderung.
