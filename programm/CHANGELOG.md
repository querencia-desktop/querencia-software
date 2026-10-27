# Änderungen

## 0.1.7 – 2026-10-26

### Hinzugefügt
- Oberfläche auf Italienisch, Spanisch und Französisch (`po/it.po`,
  `po/es.po`, `po/fr.po`, je 238 Texte) samt Desktop-Datei; Wortwahl nach
  GNOME Software der jeweiligen Sprache und der gemeinsamen
  Querencia-Begriffstabelle, Zugriffstasten je Menü eindeutig, Online-Hilfe
  auf querencialinux.org/it|es|fr/docs/installing-software.
- Katalog-Ergänzungen (Thunderbird, Firefox): Zusammenfassung, Beschreibung
  und Stichwörter auf Italienisch, Spanisch und Französisch.

## 0.1.6 – 2026-10-23

### Hinzugefügt
- Katalog-Ergänzungen: eigene AppStream-Beschreibungen für Pakete, die
  Querencia aus Ersatzquellen holt und die deshalb keinen Katalogeintrag haben.
  Erste Einträge: Thunderbird (Paket `thunderbird` aus der Paketquelle des
  Ubuntu-Mozilla-Teams) und Firefox (Paket `firefox` von Mozilla), beide unter
  „Internet“, mit eigenem Katalogsymbol bis zur Installation. Die Suche
  „thunderbird“ findet Thunderbird jetzt an erster Stelle. Format und Ort sind
  die von AppStream (`/usr/share/swcatalog/xml/querencia-ergaenzungen.xml`);
  das Programm liest denselben Aufbau zusätzlich neben der Binary
  (`share/swcatalog/`). Beschreibung und Anleitung für weitere Einträge:
  `docs/katalog-ergaenzungen.md`, Prüfwerkzeug
  `werkzeug/ergaenzungen-pruefen.sh`.
- `docs/quellen.md`: die Paketquellen für Thunderbird und Firefox (Zweck,
  Suite, Schlüssel, Pin).

### Geändert
- Snap-Hüllen (Ubuntus Übergangspakete wie `thunderbird 2:1snap1-…`, die nur
  den Snap nachinstallieren) gelten nicht als verfügbare Fassung: Ohne
  Ersatzquelle steht „Nicht verfügbar“ und der Knopf bleibt gesperrt;
  Aktualisierungen auf eine Hülle werden nicht angeboten, die Expertenliste
  zeigt sie nicht. Eine installierte Hülle bleibt sichtbar.

## 0.1.5 – 2026-10-22

### Hinzugefügt
- Hilfe-Menü: „Online-Hilfe“ öffnet die Anleitung „Software installieren“ auf
  querencialinux.org in der Sprache der Oberfläche (`/de/docs/…` bzw.
  `/en/docs/…`, die Adresse steht im Katalog). „Fehler melden“ öffnet
  `BUG_REPORT_URL` aus `/usr/share/querencia/release` (Versionsdatei des
  Metapakets ab Alpha 2.1); fehlt die Datei, die Auswahlseite für neue
  Meldungen im Repo `querencia-desktop/querencia`. Beides über
  `gtk::UriLauncher` im Standardprogramm der Sitzung.

### Geändert
- Info-Dialog einheitlich mit Dokumente, Archiv und Rechner: Hinweis „Alpha –
  nicht für den Alltag freigegeben.“, Website querencialinux.org als Link,
  „© 2026 Querencia Desktop contributors“, Lizenz.
- Fehlerdialog „Berechtigungsdialog fehlt“: Die Erklärung nennt keine interne
  Kennung mehr („In Querencia Desktop startet die Sitzung ihn als
  „querencia-polkit-agent“.“). Ein Test stellt sicher, dass kein Nutzertext in
  `po/` eine interne Kennung enthält.
- README: Die Bausteine liegen in eigenen, seit 22.10.2026 öffentlichen Repos.

## 0.1.4 – 2026-10-21

### Geändert
- Dunkle Darstellung (CSS-Welle dunkel für Thema 0.2): Software folgt
  „Dunkle Darstellung bevorzugen“ beim Start und beim Wechsel im laufenden
  Programm. Hell bleibt pixelgleich mit 0.1.3.
- Beim Start meldet das Programm einmal `gtk-interface-color-scheme`; GTK 4.22
  lädt das dunkle Thema-CSS sonst nur bei einem Wechsel, ein unter „dunkel“
  gestartetes Programm bliebe hell.
- Programm-CSS: Farben, die nur Werte von Querencia-Klassisch wiederholen
  (Menüleiste, Menü, Fenstergrund von Kopf- und Statuszeile, Schrift der
  Statuszeile), kommen aus dem Thema. Ansichtsgrund (Liste, Detailseite,
  Paketliste im Dialog) und Ränder nehmen die benannten Themenfarben
  `@theme_base_color` und `@borders`; sie folgen dem Thema ohne Weiteres und
  bleiben unter einem Thema ohne dunkle Fassung hell. Nur Farben ohne
  Themenwert (Seitenleiste, Fortschritt, Netzwerkzustand, gedimmte Texte,
  Zeilentrenner) haben einen Block `@media (prefers-color-scheme: dark)`; die
  Schrift auf Seitenleiste und Fortschritt ist dort fest gesetzt.
- Der eigene `GtkCssProvider` ist an `gtk-interface-color-scheme` gebunden;
  ohne Bindung greift der Dunkelblock nicht (isoliert gemessen).

## 0.1.3 – 2026-10-20

### Hinzugefügt
- Eigene Erreichbarkeitsprüfung (Befund B2 der Pflichtprobe): Auf
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
- Lizenzvermerk: Copyright „Querencia Desktop contributors“ in README und
  COPYRIGHT.

### Behoben
- Statuszeile behielt nach dem Laden des Katalogs und nach Ansichtswechseln
  Glyphenreste des vorigen Texts. Ursache: Die
  GPU-Renderer von GTK 4.22 zeichnen beim Textwechsel einen zu kleinen
  Bereich neu. Die Statusleiste wird jetzt bei jedem neuen Text vollständig
  neu gezeichnet.

## 0.1.0 – 2026-10-18

Erste Fassung (SW-3): Oberfläche nach schnittstellen.md, Pflichtumfang v1,
gegen Attrappen. Programmsymbol von SW-5 als Fenstersymbol und im
Info-Dialog, graues Ersatzsymbol für Anwendungen ohne Symbol. 23 Unit-Tests,
isolierte GUI-Prüfung 67/67 bei 1024×768 und 1920×1080 (je zwei Läufe).

Ein erster Stand mit Platzhalter-Fenstersymbol (Commit `7ac279f`) trägt den
Tag `software-ui/0.1.0-vorab`; der Platzhalter wurde bei der Abnahme abgelehnt.
