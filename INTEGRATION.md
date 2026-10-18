# Integration: wo die Attrappen ausgetauscht werden

Die Oberfläche spricht nur über `Rc<dyn Paketquelle>`, `Rc<dyn Katalog>` und
`Rc<dyn Transaktionsmonitor>` (alle aus `qs_typen`, SW-1). Stand 0.1.0 laufen
immer die Attrappen. Diese Stellen ändert die Integration:

| Nr. | Datei:Zeile (Tag software-ui/0.1.0) | Heute | Nach der Integration |
|---|---|---|---|
| 1 | `Cargo.toml:16` | `qs_typen` per `path` aus SW-1 | bleibt; dazu `qs_paketkit`, `qs_katalog`, `qs_transaktion` per `path` bzw. als Workspace-Mitglieder |
| 2 | `src/main.rs:69` `backends()` | baut immer `AttrappeQuelle`, `AttrappeKatalog`, `LokalerMonitor`; ohne `QS_ATTRAPPE` nur eine Warnung | `QS_ATTRAPPE=1` → Attrappen, sonst `qs_paketkit`-Quelle, `qs_katalog`-Katalog, `qs_transaktion`-Monitor (Vorschlag im Doc-Kommentar über der Funktion) |
| 3 | `src/main.rs:75` `katalog.laden()` | synchron im Hauptthread (Attrappe: sofort) | laut Schnittstelle im Hintergrund-Thread (1–2 s); Fenster vorher mit „Katalog wird geladen …“ zeigen, danach Liste aufbauen |
| 4 | `src/main.rs:91` und `src/monitor.rs` | `LokalerMonitor` (Übergang: Simulation vor Entfernen, Bestätigung, `SperreBelegt` als Wartezustand, Abbruch) | Modul `monitor` löschen, `mod monitor;` in `main.rs:21` entfernen, Monitor aus `qs_transaktion` einsetzen; Logging, Zeitgrenzen und Wiederanbindung kommen dann von SW-4 |
| 5 | `src/attrappen.rs` und `mod attrappen;` (`main.rs:17`) | Attrappen | bleiben für `QS_ATTRAPPE=1` und die Unit-Tests erhalten |
| 6 | `src/fenster.rs:772` `paketstand_laden` | `quelle.suchen("", false)` liefert alle Pakete; daraus wird der Installiert-Zustand jeder Anwendung abgelesen | **offen:** gegen den echten PackageKit ist eine Leersuche über alle Pakete zu groß. Vorschlag: `suchen("", true)` für die installierten Pakete; nicht installierte Anwendungen erst bei Details/Knopf über `paket_nach_name` auflösen; für die Expertenliste eine echte Suche verlangen (Mindestlänge). Besser wäre ein Sammelaufruf `aufloesen(&[name])` (PackageKit `Resolve`) im Trait `Paketquelle` – Änderungswunsch an SW-1 über die Status-Session |
| 7 | `src/fenster.rs:1149` `wieder_anbinden` | wird beim Start gerufen; lokaler Monitor liefert immer 0 Griffe | mit SW-4 echte Griffe; v1 zeigt den ersten davon in der Fortschrittsleiste |
| 8 | `src/fenster.rs:611` `setze_symbol` | `symbol_pfad` der Attrappe liefert nur bei `Symbol::Datei` einen Pfad, sonst Themenname | mit SW-2 Cache-Pfade (48/64); Reihenfolge Datei → Thema → `package-x-generic` bleibt |
| 9 | `src/fenster.rs` Fenstersymbol, `data/resource/` | Platzhalter-SVG `org.querencia.Software-fenster` als Ressource | Symbol von SW-5 einsetzen (nur SVG, nie 48x48 neben Scalable), `werkzeug/ressourcen.sh` neu laufen lassen |
| 10 | `werkzeug/szenario.sh` | prüft gegen `QS_ATTRAPPE=1` | bleibt so; für echte Backends ein eigenes Szenario gegen das PackageKit-Prüfgerüst von SW-1 (`test_succeed`/`test_fail`) |

Fehlertexte für die Dialoge stehen in `src/logik.rs` (`fehlertext`) und
bleiben, wenn `monitor.rs` gelöscht wird. Der KeinAgent-Text verweist auf G2-K.
