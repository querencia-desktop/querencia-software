# Änderungen

## 0.1.1 – 2026-10-20

### Hinzugefügt
- Englische Oberfläche über GNU gettext (Muster Querencia Editor 0.2.0):
  englische msgids, Deutsch aus `po/de.po` (199 Texte, Wortlaut wie 0.1.0),
  Sprache nach `LANGUAGE`/`LANG`. Kataloge neben der Binary unter
  `share/locale/`. Übersetzt werden auch die Texte der Bausteine
  (Kategorien, PackageKit-Statustexte, Fehlerdialoge von SW-4) in der
  Oberfläche; die Bausteine selbst bleiben unverändert.
- Starter-Vorlage `data/org.querencia.Software.desktop.in` (englisch), die
  deutsche Fassung kommt aus `po/de.po`.
- Prüfgerüst: Modi `i18n` (je Sprache), `glyphen` (Statuszeile nach
  Ansichtswechseln) und `anlauf`; `QS_GPU=1` prüft mit dem GPU-Renderer wie live.

### Geändert
- Deutsche Mehrzahlformen richtig: „1 Programm“, „Folgendes Paket wird mit
  entfernt (1):“, „seit 1 Minute“ (vorher immer Mehrzahl).
- Lizenzvermerk: Copyright „Querencia Desktop contributors“
  in README und Kopf der LICENSE.

### Behoben
- Statuszeile behielt nach dem Laden des Katalogs und nach Ansichtswechseln
  Glyphenreste des vorigen Texts (Werkstatt QREL-05 W.5). Ursache: Die
  GPU-Renderer von GTK 4.22 zeichnen beim Textwechsel einen zu kleinen
  Bereich neu. Die Statusleiste wird jetzt bei jedem neuen Text vollständig
  neu gezeichnet.

## 0.1.0 – 2026-10-18

Erste Fassung (SW-3): Oberfläche nach SCHNITTSTELLEN.md, Pflichtumfang v1,
gegen Attrappen. Programmsymbol von SW-5 als Fenstersymbol und im
Info-Dialog, graues Ersatzsymbol für Anwendungen ohne Symbol. 23 Unit-Tests,
isolierte GUI-Prüfung 67/67 bei 1024×768 und 1920×1080 (je zwei Läufe).

Ein erster Stand mit Platzhalter-Fenstersymbol (Commit `7ac279f`) trägt den
Tag `software-ui/0.1.0-vorab`; der Besitzer hat den Platzhalter abgelehnt.
