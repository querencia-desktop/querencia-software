# Querencia Software (Oberfläche)

Grafischer Software-Installer für Querencia Desktop, Teilprojekt SW-3:
Fenster, Ansichten und Dialoge. Rust + GTK 4 ohne libadwaita,
Server-Fensterrahmen von labwc, Menüleiste, deutsche Oberfläche,
App-ID `org.querencia.Software`. Verbindliche Schnittstellen:
`~/projects/querencia-software/SCHNITTSTELLEN.md`; Typen und Traits aus
`qs_typen` (SW-1).

**Umfang 0.1.0:** Menüleiste Datei/Bearbeiten/Ansicht/Hilfe; Seitenleiste mit
„Alle Programme“, 10 Kategorien, „Installiert“, „Aktualisierungen (n)“,
„Expertenliste“; Sofortsuche (300 ms); Liste mit Symbol 48, Name,
Zusammenfassung, Zustand und Knopf; Detailseite mit Symbol 64, Beschreibung,
Version, Größe, Lizenz, Homepage-Link; nicht modale Fortschrittsleiste mit
Abbrechen; Bestätigung vor dem Entfernen mit Simulationsliste; Dialoge für
belegte Paketsperre, fehlende Berechtigung und fehlenden polkit-Agenten;
Statuszeile; Info mit Querencia-Q. Tastatur: Strg+F, Escape, Eingabetaste,
F5, Strg+Q.

**Backends:** Stand 0.1.0 nur Attrappen (`src/attrappen.rs`, 30 Anwendungen,
simulierte Transaktionen 2 s). Austauschstellen: `INTEGRATION.md`.

| Datei | Inhalt |
|---|---|
| `src/schnittstellen.rs` | Re-Export der Traits aus `qs_typen` |
| `src/attrappen.rs` | `AttrappeQuelle`, `AttrappeKatalog` |
| `src/monitor.rs` | lokaler Übergangs-Transaktionsmonitor (SW-4 ersetzt ihn) |
| `src/logik.rs` | Ansichten, Filter, Sortierung, Suche, Fehlertexte; Unit-Tests |
| `src/fenster.rs` | Hauptfenster |
| `src/dialoge.rs` | Info, Bestätigung, Meldungen, Fenstersymbol |
| `src/style.css` | klassische Optik, Orange nur für Auswahl und Fortschritt |
| `werkzeug/` | isolierte GUI-Prüfung (TESTING.md) |
| `nachweis/` | Release-Binary mit SHA256, Prüf- und GUI-Protokolle |

Desktop-Datei, endgültiges Symbol und MetaInfo liefert SW-5.

Lizenz: GNU GPL, Version 3 oder neuer.
