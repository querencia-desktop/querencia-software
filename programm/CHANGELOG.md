# Änderungen

## 0.1.4 – 2026-10-21

### Geändert
- Dunkle Darstellung (QD-081, CSS-Welle dunkel für Thema 0.2): Software folgt
  „Dunkle Darstellung bevorzugen“ beim Start und beim Wechsel im laufenden
  Programm. Hell bleibt pixelgleich mit 0.1.3.
- Beim Start meldet das Programm einmal `gtk-interface-color-scheme`; GTK 4.22
  lädt das dunkle Thema-CSS sonst nur bei einem Wechsel, ein unter „dunkel“
  gestartetes Programm bliebe hell.
- Programm-CSS: Farben, die nur Werte von Querencia-Klassisch wiederholen
  (Menüleiste, Menü, Fenstergrund von Kopf- und Statuszeile, Schrift der
  Statuszeile), kommen aus dem Thema. Eigene Farben (Seitenleiste, Liste,
  Detailseite, Fortschritt, Ränder, Netzwerkzustand, gedimmte Texte,
  Paketliste im Dialog) haben einen Block `@media (prefers-color-scheme: dark)`
  mit den Werten der dunklen Fassung.
- Der eigene `GtkCssProvider` ist an `gtk-interface-color-scheme` gebunden;
  ohne Bindung greift der Dunkelblock nicht (isoliert gemessen).

## 0.1.3 – 2026-10-20

### Hinzugefügt
- Eigene Erreichbarkeitsprüfung (QD-067, Befund B2 der Pflichtprobe): Auf
  Servern mit netplan/systemd-networkd, auf denen NetworkManager installiert
  ist, die Verbindung aber nicht verwaltet, meldet PackageKit „offline“.
  Die Oberfläche prüft deshalb in drei Stufen: PackageKit `NetworkState`,
  dann GNetworkMonitor (GLib), zuletzt ein HTTP-`HEAD` auf die APT-Quelle
  der Distribution (aus `/etc/apt/sources.list` bzw. `sources.list.d/`,
  Zeitlimit 3 s). Offline gilt nur, wenn keine Stufe online sagt.
- Statuszeile rechts: „Netzwerk: online (PackageKit | Netzwerk verfügbar |
  Archiv erreichbar)“ bzw. „Netzwerk: offline: <Host> nicht erreichbar
  (<Grund>)“; die Kurzinfo nennt jede gefragte Stufe und die geprüfte Quelle.
  Knopf „Erneut prüfen“; im Offline-Zustand prüft das Programm alle 30 s
  neu, im echten Betrieb zusätzlich 2 s nach jeder Netzänderung, die GLib meldet.
- Nichts wird gesperrt. Sagen alle Stufen offline, fragt das Programm vor
  dem Installieren/Aktualisieren nach einer frischen Prüfung („Keine
  Netzwerkverbindung“ mit der geprüften Quelle, Knöpfe „Abbrechen“ und
  „Trotzdem versuchen“ – Pakete aus dem apt-Zwischenspeicher gehen auch
  ohne Netz). Entfernen läuft immer ohne Rückfrage.
- PackageKit „offline“, Quelle erreichbar (NetworkManager installiert,
  verwaltet das Netzwerk nicht): Statuszeile „online (Archiv erreichbar),
  PackageKit: offline“ in Warnfarbe, Kurzinfo mit Ursache und Abhilfe
  („netplan-Renderer auf NetworkManager stellen oder NetworkManager
  entfernen“). Der Versuch läuft; den `no-network`-Fehler erklärt der Dialog
  „PackageKit meldet kein Netzwerk“. Software ändert nichts am System.
- Kommt das Netz zurück, verschwindet die veraltete Meldung „Keine
  Netzwerkverbindung“ aus der Statuszeile.
- Diagnose auf der Konsole: `querencia-software --netz-pruefen` fragt alle
  drei Stufen (Exit 0 online, 1 offline); `QS_APT_QUELLE=<uri>` setzt die
  geprüfte Quelle. Attrappen für die GUI-Prüfung:
  `QS_NETZ_ATTRAPPE=<pk>,<monitor>,<archiv>` (je `online`/`offline`) oder
  `@<datei>` mit demselben Inhalt, bei jeder Prüfung neu gelesen.
- 35 neue Texte in `po/de.po` (234 insgesamt).

## 0.1.2 – 2026-10-20

### Geändert
- Dunkle Auswahlschrift (CSS-Welle, passend zum GTK-Thema
  Querencia-Klassisch, Entscheidung 20.10.): Menüleiste, Menüeinträge, die
  gewählte Kategorie der Seitenleiste und die gewählte Zeile der
  Programmliste (Name, Zusammenfassung, Zustand) schreiben `#1a1a1a` statt
  `#ffffff` auf Orange. Kontrast 6,2:1 (`#f07746`) bis 8,5:1 (`#f6a179`);
  mit Weiß waren es 2,8:1 bzw. 2,0:1. Farben der Flächen und alles Übrige
  unverändert; Texte und Katalog unverändert.

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
