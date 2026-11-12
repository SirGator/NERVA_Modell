# Dokumentation: Forschung und Experimente

[Aktueller Forschungsstand](../../../Doku/FORSCHUNGSSTAND.md) ·
[Harness bedienen](../README.md) · [Zentrale Dokumentation](../../../Doku/README.md)

Protokolle legen Ablauf und Kriterien fest. Ergebnisberichte beschreiben die
tatsächlich ausgeführten Messungen und ihre Grenzen. Datierte Planungsdokumente
behalten ihren damaligen Stand.

## Neuromodulation und kausale Diagnose

[MLD-02](../MLD02.md): vorregistrierter Kausalatlas mit fünf Seeds, lokalen
Historien und aktivem Homöostase-/Konkurrenzvergleich.
[Ergebnisse und Historien vom 12.11.2026](../MLD02_RESULTS_20261112.md).

[Richtung × Stärke](../DIRECTION_STRENGTH.md): normkontrollierter Vierfeldervergleich
und rekurrente Kontrast-/Relaiskontrollen.
[Pilot vom 10.11.2026](../DIRECTION_STRENGTH_RESULTS_20261110.md).

[Richtungs- und Abrufpfadanalyse](../DIRECTION_PATH.md): künstliche Referenzen
mit und ohne Sensorprojektion, natürliche Lerndeltas und elektrische Bilanzen.
[Pilot vom 10.11.2026](../DIRECTION_PATH_RESULTS_20261110.md).

[Präzisionsvergleich](../PRECISION.md): native Direktupdates gegen
synapsenlokale Akkumulation; getrennte Masse-, Spike- und Assoziationsendpunkte.
[Geprüfter Pilot vom 10.11.2026](../PRECISION_RESULTS_20261110.md).

[Diagnosekette A–D](../CAUSAL_CHAIN.md): numerische Plastizität, künstlicher
A→B-Abruf, feste Wirksamkeitsdosen und lokale Prä-/Post-/Modulatorverfügbarkeit.
[Geprüfte Befunde vom 10.11.2026](../CAUSAL_CHAIN_RESULTS_20261110.md).

Der aktuelle Einstieg für gepaarte Vergleiche ist das
[Forschungskontrollzentrum](../CONTROL.md). Es kombiniert die Chemie-/Lernregel-
Kontrollen mit kompakten Learning Audits und einem Gewichtsreset beim Abruf.
[Pilotbericht vom 10.11.2026](../HIPPOCAMPAL_CONTROL_RESULTS_20261110.md).

| Thema | Protokoll | Ergebnisse |
| --- | --- | --- |
| Hippocampale Gewebeprofile und fester Bildpilot | [Hippocampus](../HIPPOCAMPAL.md) | [Integration und Pilot](../HIPPOCAMPAL_RESULTS_20261109.md) |
| Diagnose der Gewichtsänderungen | [Gewichtsdiagnostik](../WEIGHT_DIAGNOSTICS.md) | [Gewichtsbefunde](../results/hippocampal_weight_diagnostics_20261109/RESULTS.md) |
| Bildvergleich mit 256 Processing-Zellen | [Konfiguration](../config/hippocampal_256_20261109.toml) | [256-Zellen-Pilot](../results/hippocampal_256_20261109/RESULTS.md) |
| Eligibility, lokaler Faktor, Gewicht und Ausgabewirkung | [Kausales Diagnoseprotokoll](../CAUSALITY_256.md) | [Kausale Befunde](../results/hippocampal_causality_256_20261109/RESULTS.md) |

## Forschungs-Harness

- [Planungsstand: Codeabgleich und Umsetzung](../NEXT_PHASE.md).
- [Erste feste M0-Referenzmessung](../PHASE1_BASELINE.md).
- [Sensomotorischer Größen-/Umweltadapter](../SENSORIMOTOR.md) und [erster Messbericht](../SENSORIMOTOR_RESULTS_20261105.md).
- [Frei skalierbare Größen und Diagnoseprüfungen](../SCALING.md) · [erster Diagnoselauf](../SCALING_RESULTS_20261105.md).
- [Vorregistrierter Homöostasevergleich auf 24 neuen Seeds](../HOMEOSTASIS_CONFIRMATION.md) · [Ergebnisse](../HOMEOSTASIS_RESULTS_20261105.md).
- [Homöostase nach Zellpopulation lokalisieren](../HOMEOSTASIS_POPULATIONS.md) · [Ergebnisse](../HOMEOSTASIS_POPULATIONS_RESULTS_20261106.md).
- [Forschungs-Harness und Methoden](../HARNESS.md): Planung, Interventionen, Zustandskarten und Forschungsgedächtnis.
- [Diagnose](../DIAGNOSTICS.md): Decoder, Kontrollen und Diagnosecontroller.
- [Externe Selektion](../SELECTION.md): Generationen, Auswahl und Abschlussvalidierung.
- [Stand und Folgeversuche vom 05.11.2026](../FOLLOWUP_20261105.md).
- [Research: Einstieg und Konfiguration](../README.md).

## Versuchsreihen

Die Protokolle beschreiben historische Forschungsstände; sie sind
keine Aussage, dass ein Versuch erfolgreich war. Ergebnisse und Manifeste
liegen separat im Ordner `Messungen/Experimente`.

- [Historische Übersicht und frühere Startbefehle](versuchsreihen/README.md) · [Roadmap](versuchsreihen/ROADMAP.md)
- [M3](versuchsreihen/M3_EXPERIMENT.md) · [M4](versuchsreihen/M4_EXPERIMENT.md) · [M5](versuchsreihen/M5_EXPERIMENT.md) · [M6](versuchsreihen/M6_EXPERIMENT.md)
- [M7](versuchsreihen/M7_EXPERIMENT.md) · [M8](versuchsreihen/M8_EXPERIMENT.md) · [M9](versuchsreihen/M9_EXPERIMENT.md) · [M9 V1](versuchsreihen/M9_V1_EXPERIMENT.md)
- [M10](versuchsreihen/M10_EXPERIMENT.md) · [M10B](versuchsreihen/M10B_EXPERIMENT.md) · [M10C](versuchsreihen/M10C_EXPERIMENT.md) · [M10D](versuchsreihen/M10D_EXPERIMENT.md) · [M10E](versuchsreihen/M10E_EXPERIMENT.md)
- [M11](versuchsreihen/M11_EXPERIMENT.md)
- Weitere Ansätze: [Agency](versuchsreihen/AGENCY_EXPERIMENT.md), [Rekurrenz](versuchsreihen/RECURRENT_EXPERIMENT.md), [Retention](versuchsreihen/RETENTION_EXPERIMENT.md), [Sensomotorik](versuchsreihen/SENSORIMOTOR_EXPERIMENT.md).

Die historischen Anleitungen unter `dokumentation/versuchsreihen` enthalten
keinen ausführbaren Experimentcode. Alte Startbefehle sind stillgelegt.
Aktuelle Versuche starten über `research/run.sh`; siehe
[Auflösung und integrierte Bausteine](versuchsreihen/MIGRATION.md).
