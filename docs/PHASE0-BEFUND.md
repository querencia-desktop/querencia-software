# Phase 0 für Querencia Software: aktualisierter Befund (SW-6)

Stand 18.10.2026, 22:06, lesend gemessen auf entwicklungs-vm. Grundlage:
`~/projects/querencia-software-paket/phase0/PHASE0-CHECKLISTE.md` (SW-5, 21:00).
Diese Datei ersetzt die Checkliste nicht, sie hält den Stand vor der Live-Ablage fest.

| Punkt | Annahme im Auftrag | Gemessen | Folge für das Programm |
|---|---|---|---|
| polkit-Agent | „läuft seit 20:26“ | **läuft nicht.** Kein Prozess `querencia-polkit-agent` (ps, /proc). Letzter Eintrag in `~/.local/state/querencia/polkit-agent.log`: 20:29:19 „wieder beendet: keine gültige Freigabe für den Start“. Davor hat er um 20:28 eine echte Legitimierung `packagekit.package-install` bedient (G2-K). Autostart steht seit 20:27 in `querencia-runtime/session/querencia/labwc/autostart` (`QUERENCIA_POLKIT_AGENT`), die laufende Sitzung stammt aber von 16:36, also startet er erst bei der nächsten Anmeldung. | Ohne Agent meldet `qs_paketkit` bei `not-authorized` `Fehler::KeinAgent` (Heuristik `/proc`). Das Programm zeigt dann den Dialog „Berechtigungsdialog fehlt“ mit dem Text von SW-4 („aus einem Terminal der Sitzung starten oder neu anmelden“). Dieser Pfad bleibt im Code und ist im Prüfbus geprüft (Modus `verweigert`, F26). |
| `apt-config-icons` | „installiert“ | **laut dpkg nicht installiert** (`un`, `apt-cache policy`: Installiert (keine)). Der Symbolcache ist trotzdem gefüllt: `/etc/apt/apt.conf.d/50appstream` aus dem Paket `appstream` fordert die DEP-11-Symbolarchive (48 und 64 px) bei `apt update` schon an. `/var/lib/swcatalog/icons/*/64x64/` hat 2292 PNG-Dateien, angelegt 20:51:31. | Katalogsymbole funktionieren ohne das Paket. In der Liste kommen sie aus dem Cache (Fall „Symbole aus dem swcatalog-Cache“, Grafik: 9 von 9 sichtbaren Zeilen). `apt-config-icons` bleibt optional, etwa für `-hidpi`. |
| `language-pack-gnome-de-base` | „installiert“ | **laut dpkg nicht installiert** (`dpkg -l 'language-pack*'` leer). | Kein Einfluss auf den Katalog: die deutschen AppStream-Texte stehen in den DEP-11-Daten (846 von 2648 Komponenten mit deutscher Zusammenfassung). PackageKit-Statustexte übersetzt `qs_paketkit` selbst. |
| PackageKit | 1.3.4 läuft | `packagekit 1.3.4-3ubuntu1.2` installiert, per D-Bus aktivierbar; lesend erreichbar (Resolve von 1998 Namen in 0,5 s, GetUpdates 5, GetDetails). | – |
| pkexec | fehlt | fehlt weiter | wird nicht gebraucht |

## Was das für die Freigabe heißt

- Die **Live-Ablage** (Programm, Menüeintrag, Symbol, MetaInfo) braucht keinen Agenten. Das Programm startet, lädt den Katalog und zeigt Paketstand und Aktualisierungen.
- Eine **echte Installation** braucht den Agenten. Sie bleibt dem Besitzer vorbehalten: nach einer Neuanmeldung oder nach `~/opt/querencia/bin/querencia-polkit-agent &` aus einem Terminal der Sitzung. Danach ein kleines, harmloses Paket wie `sl` oder `cowsay` installieren und wieder entfernen.
- Solange kein Agent läuft, endet jeder Installationsversuch mit dem Dialog „Berechtigungsdialog fehlt“. Am System passiert dabei nichts.

## Nachtrag 22:21

Der Besitzer hat den Agenten um 22:20:24 neu gestartet (`querencia-polkit-agent`, PID 1288664). Damit
erscheint bei einer Installation der Passwortdialog statt „Berechtigungsdialog fehlt“. Die Status-Session
hat bestätigt, dass `apt-config-icons` und `language-pack-gnome-de-base` nicht installiert sind.
