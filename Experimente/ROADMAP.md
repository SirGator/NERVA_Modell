# NERVA: Forschungsfahrplan

Stand **27.10.2026**. Ziel ist zunächst ein kontrollierter Nachweis, dass lokale
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
| M10 | An welcher Stelle wird erfahrungsabhängige Information gespeichert, weitergegeben oder ununterscheidbar? | Nächster vorgeschlagener Schritt; Protokoll noch festzulegen |
| M11 | Unter welchen Bedingungen bleiben nachgewiesene Muster und ihre Funktion erhalten? | Vorgeschlagen; abhängig von messbarer Information und der M10-Diagnose |
| M12 | Können lokale Entwicklungsregeln funktionale Verschaltungen hervorbringen? | Vorgeschlagen; vorhandenes Wachstum/Pruning gezielt funktional prüfen |
| M13 | Ist für die geprüfte Aufgabe eine andere zeitliche Verarbeitung nötig? | Optional; erst nach konkreter Diagnose |

Diese Nummern definieren Forschungsrichtungen. M10–M13 sind keine bestehenden
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
Reichweite des Befunds. Ein konkretes Protokoll mit Endpunkten und Kontrollarmen
wird erst anhand der M9-Daten vorab festgelegt.

## M11: Stabilisierung und Gedächtnis

Zunächst die vorhandene Homöostase gezielt nach Population und Stärke untersuchen.
M5 zeigt bereits, dass Sensor-Homöostase Weiterleitung schwächen kann. Erhalt
relativer Gewichte, zeitliche Stabilität und funktionale Vorhersage werden
getrennt gemessen, auch unter weiterlaufender Plastizität und Interferenz.

Synaptische Konkurrenz und proportionale Skalierung sind mögliche gezielte
Erweiterungen, sofern die Diagnose sie begründet. Sie sind noch kein vollständiger
vorhandener Mechanismus. Inhibitorische Plastizität wäre ein eigener kontrollierter
Versuch, keine stillschweigende Erweiterung des M9-Modells.

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
