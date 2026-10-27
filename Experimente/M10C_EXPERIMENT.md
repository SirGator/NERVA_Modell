# M10-C: Wo verändern sich wirkungsbezogene Signalbeiträge?

Definition vom **27.10.2026**, nach Kenntnis von M9/M10/M10-B und vor Ausführung
dieser Zusatzanalyse. **Retrospektive explorative Bilanz vorhandener Trajektorien,
keine neue Bestätigung und keine neue neuronale Regel.**

## Fester Umfang und Bezug

Alle 20 M10-Traces der Kombination auf Seeds 610–629 werden verwendet,
einschließlich vier Armen, beiden Belegungen, beiden Zeitpunkten und drei
Gewichtseingriffen. Ein technischer Test nutzt ausschließlich den ersten Seed;
die Diagnose verwendet sämtliche Seeds. Quellmanifeste und Trace-Prüfsummen
werden vor Berechnung fixiert.

Die neuronale Relevanz stammt unverändert vom visuellen Spikezähl-Ausleser:
Einheitsvektor zwischen standardisierten visuellen Klassenmitteln, geteilt durch
die bisherige Spikezähl-Skalierung je Zelle. **Es werden keine eigenen visuellen
Klassenachsen für Eingangsimpulse oder Potenziale kalibriert.** Die Projektionen
beschreiben daher Antrieb in für den bestehenden Ausleser relevante Zellen,
nicht eigenständige Wirkungsprognosen dieser physikalischen Signaltypen.

## Messungen und Identitäten

Für jedes A-/D-Paar wird der vorzeichenkorrigierte Differenzbeitrag auf denselben
festen Neuronengewichten berechnet. Der Ausgangszustand vor Aktionsbeginn muss
für beide Antworten identisch sein. Gemessen werden:

- alle signierten Eingangsimpulse sowie die nach tatsächlicher Refraktärfilterung
  angenommenen Impulse;
- deren Leckgewichtung bis zum gemeinsamen Prognosezeitpunkt;
- linearer Antrieb einschließlich abgeklungenem Ausgangszustand;
- Abzug der tatsächlichen Potenzialwerte vor jedem beobachteten Spike-Reset;
- das tatsächliche Restpotenzial am Ende sowie die Spikezählmuster.

Die neutrale LIF-Bilanz lautet je Zelle:

`Endpotenzial = abgeklungener Anfangszustand + leckgewichtete Eingänge
− verworfene Refraktäreingänge − leckgewichtete Resetabzüge`.

Die Analyse folgt den **beobachteten Spikes und wiederkehrenden Eingängen**.
Sie erzeugt keine neuen Spikes und resimuliert kein alternatives Netzwerk ohne
Reset/Hemmung. Rekonstruiertes Endpotenzial und Refraktärzustand müssen zur Quelle
passen; f32-Toleranz 2×10⁻⁵. Die additive Bilanz wird zusätzlich algebraisch geprüft.

Eingänge werden nach Aktionskopie, rekurrenter Erregung, rekurrenter Hemmung,
visuellem Ursprung und sonstigen Quellen aufgeteilt, außerdem je Synapse
ausgewiesen. Gruppen- und Synapsensummen müssen die Gesamtprojektion reproduzieren.
Die Spikeprojektion muss exakt die M10-B-Richtung reproduzieren.

## Auswertung und Grenzen

Berichtet werden je Belegung, Zeitpunkt, Arm und Gewichtseingriff positive,
negative und verschwindende Richtungsbeiträge sowie Vorzeichenwechsel bei
Refraktärfilterung, Leckgewichtung und Übergang zu Spikezahlen. Numerische
Gleichheit verwendet 1e-12 relativ zur Summe absoluter Projektionsbeiträge.
Die Spike-Richtung verwendet unverändert die M10-B-Gleichstandsregel.

Physikalisch unterschiedliche Größen werden nicht als gleiche Stärken oder
Vorhersagegenauigkeiten behandelt. Besonders wichtig: **Das Endpotenzial liegt
nach Spikes/Resets und ist kein einfacher Zustand vor der Spikebildung.** Eine
Richtungsumkehr dieses Restwerts kann eine Folge erfolgreicher Spikebildung sein.

Vergleiche mit eingefrorenen, entkoppelten und zurückgesetzten Kontrollen bleiben
deskriptiv auf denselben bekannten Seeds. Es gibt keinen neuen Signifikanz- oder
Lernnachweis. Die Analyse lokalisiert Vorzeichenunterschiede relativ zum
bestehenden Ausleser; sie beweist weder allgemeine Informationslosigkeit noch
die Notwendigkeit eines bestimmten biologischen Zusatzmechanismus.

[Ausführung](m10c/README.md).
