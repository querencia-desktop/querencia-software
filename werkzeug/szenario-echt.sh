#!/bin/bash
# SW-6, Modus „echt“: integriertes Programm gegen den System-PackageKit (NUR
# LESEND – QS_NUR_LESEN=1, das Programm verweigert ausfuehren/simulieren) und
# den AppStream-Katalog des Systems; MetaInfo der Querencia-Programme liegen im
# Profil ($XDG_DATA_HOME/metainfo). Fallnummern nach SW-5 (FAELLE.md, F01–F30);
# schreibende Fälle (F09–F12, F25, F26) laufen im Modus „pruefbus“.
# Kein Klick auf Installieren/Entfernen/Aktualisieren in diesem Szenario.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt – Abbruch"; abschluss; exit 0; }
[ -z "${QS_PK_BUS:-}" ] || { nok "Schutz: QS_PK_BUS gesetzt – falscher Modus"; abschluss; exit 0; }

start_programm
running && ok "F01 Start ($R)" || { nok "F01 Start"; abschluss; exit 0; }
[ "$KALIBRIERT" = 1 ] && [ "$OX" -ge 15 ] && [ "$OX" -le 30 ] && ok "F01 App-ID org.querencia.Software: labwc-Fensterregel greift (Inhalt bei $OX,$OY)" || nok "F01 Fensterregel/Kalibrierung ($OX,$OY, kalibriert=$KALIBRIERT)"
fall "Backends: echt, Systembus, nur lesend" hat_letzte '^backends' $'backends\techt\tsystem\tnur-lesen'
read -r FW FH < <(letzte '^fenster' | cut -f2,3)
fall "F01 Fenstergröße 960×640 (ist ${FW:-?}×${FH:-?})" test "${FW:-0}" = 960 -a "${FH:-0}" = 640
fall "Programmsymbol (SW-5) und Ersatzsymbol als Ressource im Symbolthema" \
    hat_letzte '^symbole' $'symbole\torg.querencia.Software-fenster\ttrue\torg.querencia.Software-paket\ttrue'
fall "Während des Ladens: Statuszeile „Katalog wird geladen …“" grep -qP '^status\t.*Katalog wird geladen …' "$LOG"

# F21 Katalog ----------------------------------------------------------------
warte '^zeit\tkatalog' 0 30
read -r _ _ KMS KSTART < <(letzte '^zeit\tkatalog' | tr '\t' ' ')
info "Katalog geladen in ${KMS:-?} ms (Hintergrund-Thread), ${KSTART:-?} ms nach Programmstart"
fall "Katalog lädt in < 2 s (${KMS:-?} ms)" test "${KMS:-99999}" -lt 2000
read -r _ KK KP KS KD _ < <(letzte '^katalog' | tr '\t' ' ' | cut -d' ' -f1-6)
KM=$(letzte '^katalog' | grep -o "$HMP04_PROFILE/data/metainfo/[^ ]*" | wc -l)
info "Katalog: $KK Komponenten, $KP mit Paket, $KS mit Symbol, $KD deutsch, $KM MetaInfo-Dateien aus dem Profil"
fall "F21 Katalog: ≥ 2235 Anwendungen mit Paket, 14 MetaInfo-Dateien aus \$XDG_DATA_HOME/metainfo" test "${KP:-0}" -ge 2235 -a "$KM" = 14
warte '^zeit\tpaketstand' 0 60
read -r _ _ PMS PN PG < <(letzte '^zeit\tpaketstand' | tr '\t' ' ')
info "Paketstand: $PN Paketnamen per Resolve in $PMS ms aufgelöst, $PG Pakete geliefert"
fall "Paketstand: Sammel-Resolve gegen den System-PackageKit liefert Pakete" test "${PG:-0}" -gt 1000
sleep 1.5; warte_layout >/dev/null
passt_letzte '^ansicht' "^ansicht\talle\t$KP\t\$" && ok "Alle Programme: $KP Einträge" || nok "Alle Programme ($(letzte '^ansicht'))"
shot start

# F02/F03 Menü und Seitenleiste ---------------------------------------------
fall "F02 Menüleiste: 4 Köpfe (Datei, Bearbeiten, Ansicht, Hilfe)" test "$(layout | grep -c '^menue')" = 4
fall "F03 Seitenleiste: 14 Einträge (Alle, 10 Kategorien, Installiert, Aktualisierungen, Expertenliste)" test "$(layout | grep -c '^seite')" = 14

# F04 Kategorien gefüllt ------------------------------------------------------
leer=0; liste=""
for k in Zubehör Büro Grafik Internet Unterhaltungsmedien Spiele Entwicklung Bildung Systemwerkzeuge Sonstiges; do
    v=$(zahl '^ansicht'); seite "kategorie:$k"; warte '^ansicht' "$v" 3
    n=$(letzte '^ansicht' | cut -f3)
    liste="$liste $k=$n"
    [ "$(letzte '^ansicht' | cut -f2)" = "kategorie:$k" ] && [ "${n:-0}" -gt 0 ] || leer=1
done
info "Kategorien:$liste"
fall "F04 Alle 10 Kategorien per Mausklick gefüllt" test $leer = 0
v=$(zahl '^ansicht'); seite "kategorie:Grafik"; warte '^ansicht' "$v" 3; warte_layout >/dev/null; shot kategorie-grafik

# F06 Listenzeile und Symbole --------------------------------------------------
fall "F06 Listenzeile: Name und Knopf gemeldet" bash -c "[ \"\$(tac '$LOG' | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | grep -c -P '^(zeile|knopf)\t')\" -ge 4 ]"
nd=$(layout | grep -c -P '^zeilensymbol\t.*\tdatei:/var/lib/swcatalog/icons/')
ne=$(layout | grep -c -P '^zeilensymbol\t.*\tthema:org.querencia.Software-paket$')
nz=$(layout | grep -c '^zeilensymbol')
info "Grafik, sichtbare Zeilen: $nz, davon $nd Symbole aus dem swcatalog-Cache, $ne Ersatzsymbol"
fall "Symbole aus dem swcatalog-Cache (apt-Symbolarchive) in der Liste" test "$nd" -ge 1

# F05/F22 Suche ----------------------------------------------------------------
v=$(zahl '^ansicht'); seite alle; warte '^ansicht' "$v"; warte_layout >/dev/null
suche querencia 5
NQ=$(letzte '^ansicht' | cut -f3)
fall "F22 Suche „querencia“ findet ≥ 10 eigene Programme aus den MetaInfo-Dateien ($NQ)" test "${NQ:-0}" -ge 10
fall "F22 Erste Treffer heißen „Querencia …“" bash -c "[ \"\$(tac '$LOG' | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | awk -F'\t' '\$1==\"zeile\"{print \$3}' | grep -c '^Querencia ')\" -ge 5 ]"
shot suche-querencia
suche firefox 5
NF=$(letzte '^ansicht' | cut -f3)
fall "F05 Sofortsuche „firefox“ liefert Treffer ($NF)" test "${NF:-0}" -ge 1
FT=$(layout | awk -F'\t' '$1=="zeile"{print $3}' | head -5 | tr '\n' ';')
info "Suche firefox, erste Treffer: $FT (Firefox selbst ist auf der VM ein Snap und steht nicht im deb-Katalog)"
shot suche-firefox

# F07/F23 Details eines echten Pakets --------------------------------------------
suche gimp 5
v=$(zahl '^paketdetails')
zeile "GNU Image Manipulation Program"; q "key Return" "wait 600"
passt_letzte '^details' '^details\tGNU Image Manipulation Program\t(Verfügbar|Installiert)\t(Installieren|Entfernen)$' && ok "F07 Detailseite GIMP ($(letzte '^details' | cut -f3))" || nok "F07 Detailseite GIMP"
warte '^paketdetails' "$v" 20
read -r _ DN DG < <(letzte '^paketdetails' | tr '\t' ' ')
APT=$(apt-cache show --no-all-versions gimp 2>/dev/null | awk '/^Installed-Size:/{print $2*1024; exit}')
DL=$(apt-cache show --no-all-versions gimp 2>/dev/null | awk '/^Size:/{print $2; exit}')
info "GetDetails gimp: ${DG:-?} Byte; apt-cache: Size ${DL:-?}, Installed-Size ${APT:-?} Byte"
fall "F23 Details über PackageKit GetDetails (lesend): Größe von gimp > 0" test "${DN:-}" = gimp -a "${DG:-0}" -gt 0
warte_layout >/dev/null; shot details-gimp
q "key Escape" "wait 500"
fall "F08 Escape: zurück zur Liste" hat_letzte '^details' $'details\tzu'

# F13 Installiert ------------------------------------------------------------------
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"
v=$(zahl '^ansicht'); menue 2 1; warte '^ansicht' "$v" 3; warte_layout >/dev/null
NI=$(letzte '^ansicht' | cut -f3)
fall "F13 Menü Ansicht → Installiert: $NI Programme" passt_letzte '^ansicht' '^ansicht\tinstalliert\t[1-9]\d*\t$'
shot installiert
T0=$(layout | awk -F'\t' '$1=="zeile" && $2==0 {print $3}')
v=$(zahl '^paketdetails')
zeile "$T0"; q "key Return" "wait 600"
warte '^paketdetails' "$v" 20
PN0=$(letzte '^paketdetails' | cut -f2)
st=$(dpkg-query -W -f='${Status}' "$PN0" 2>/dev/null || true)
info "Installiert, erste Zeile: $T0 → Paket $PN0, dpkg: $st"
fall "F13 Erste Zeile der Ansicht Installiert ist laut dpkg installiert ($PN0)" test "$st" = "install ok installed"
q "key Escape" "wait 400"

# F14/F24 Aktualisierungen -------------------------------------------------------------
NU=$(letzte '^seitentext\taktualisierungen' | cut -f3)
AU=$(LC_ALL=C apt list --upgradable 2>/dev/null | grep -c upgradable || true)
info "Aktualisierungen: PackageKit GetUpdates $NU, apt list --upgradable $AU"
fall "F24 Updates-Liste über GetUpdates (lesend): Zähler = apt ($NU/$AU)" test "${NU:-x}" = "$AU"
v=$(zahl '^ansicht'); seite aktualisierungen; warte '^ansicht' "$v"; warte_layout >/dev/null
fall "F14 Ansicht Aktualisierungen: $NU Einträge" hat_letzte '^ansicht' $'ansicht\taktualisierungen\t'"$NU"$'\t'
fall "F14 Seitenleiste: „Aktualisierungen ($NU)“" hat_letzte '^seitentext' $'seitentext\taktualisierungen\t'"$NU"
shot aktualisierungen

# F15 Datei → Aktualisierungen prüfen (nur lesen: GetUpdates, kein RefreshCache) --------
v=$(zahl '^status\t.*Aktualisierungen geprüft'); menue 0 0; warte '^status\t.*Aktualisierungen geprüft' "$v" 30
fall "F15 Datei → Aktualisierungen prüfen: Statuszeile meldet das Ergebnis" grep -qP "^status\t.*Aktualisierungen geprüft: $NU verfügbar" "$LOG"

# F16 Expertenliste -------------------------------------------------------------------
v=$(zahl '^ansicht'); menue 2 3; warte '^ansicht' "$v" 3; warte_layout >/dev/null
NE=$(letzte '^ansicht' | cut -f3)
fall "F16 Menü Ansicht → Expertenliste: $NE Pakete (aufgelöste Katalogpakete)" passt_letzte '^ansicht' '^ansicht\texperten\t[1-9]\d*\t$'
suche l 3
fall "F16 Expertenliste: ein Zeichen sucht nicht (Mindestlänge 2)" hat_letzte '^ansicht' $'ansicht\texperten\t0\tl'
suche libgtk-4 3
warte_treffer libgtk-4 30
NL=$(letzte '^ansicht' | cut -f3)
fall "F16 Expertenliste: PackageKit-Suche „libgtk-4“ über alle Pakete ($NL Treffer)" test "${NL:-0}" -ge 1
shot expertenliste
v=$(zahl '^paketdetails')
T1=$(layout | awk -F'\t' '$1=="zeile" && $2==0 {print $3}')
zeile "$T1"; q "key Return" "wait 500"
warte '^paketdetails' "$v" 20
passt_letzte '^details' "^details\t$T1\t" && ok "F16 Expertenliste: Details des reinen Pakets $T1" || nok "F16 Details reines Paket"
q "key Escape" "wait 300" "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"
v=$(zahl '^ansicht'); menue 2 0; warte '^ansicht' "$v" 3; warte_layout >/dev/null

# F18 Statuszeile --------------------------------------------------------------------
fall "F18 Statuszeile: Programme, installiert, Prüfzeit" \
    grep -qP "^status\tKatalog: $KP Programme, \d+ installiert · letzte Aktualisierungsprüfung \d\d:\d\d" "$LOG"

# F17/F28 Info-Dialog mit Tastaturfokus schließen (3×) ----------------------------------
for weg in Eingabetaste Escape Alt+S; do
    vd=$(zahl '^dialog\tinfo'); vz=$(zahl '^dialog\tzu\tinfo')
    menue 3 0
    warte '^dialog\tinfo' "$vd" 3; sleep 0.5
    [ "$weg" = Eingabetaste ] && shot info-dialog
    case $weg in Eingabetaste) q "key Return" ;; Escape) q "key Escape" ;; Alt+S) q "chord alt s" ;; esac
    q "wait 800"; warte '^dialog\tzu\tinfo' "$vz" 2
    q "chord ctrl f" "type gimp" "wait 700"; sleep 0.3
    if running && [ "$(letzte '^ansicht' | cut -f4)" = gimp ]; then ok "F17/F28 Info-Dialog per $weg geschlossen, danach Tastatur ohne Absturz"
    else nok "F17/F28 Info-Dialog per $weg ($(letzte '^ansicht'))"; fi
    q "chord ctrl a" "key BackSpace" "wait 500"
done
fall "Info-Dialog zeigt Programmsymbol (SW-5) neben dem Querencia-Q" \
    hat_letzte '^info-symbole' $'info-symbole\torg.querencia.Software-fenster\tquerencia-q'

# F29 Starter, F30 --version ---------------------------------------------------------
gio_ergebnis=$(python3 -I -W ignore - <<'PY' 2>/dev/null
import gi
gi.require_version("Gio", "2.0")
from gi.repository import Gio
a = Gio.DesktopAppInfo.new("org.querencia.Software.desktop")
if a is None:
    print("fehlt")
    raise SystemExit
c = (a.get_categories() or "").split(";")
# Panel-Regel (apps.rs, system_section): Settings mit System = Systemverwaltung
sek = "Systemverwaltung" if "Settings" in c and "System" in c else ("Einstellungen" if "Settings" in c else "kein System-Menü")
print(f"{a.get_name()}|{a.get_display_name()}|{sek}|{a.get_string('Icon')}|{a.get_executable()}")
PY
)
case $gio_ergebnis in
"Software|Software|Systemverwaltung|org.querencia.Software|$BIN") ok "F29 Starter: GIO findet org.querencia.Software.desktop, Name „Software“, System → Systemverwaltung, Exec = Binary" ;;
*) nok "F29 Starter: $gio_ergebnis" ;;
esac
if [ -f "$XDG_DATA_HOME/icons/hicolor/scalable/apps/org.querencia.Software.svg" ] && [ ! -e "$XDG_DATA_HOME/icons/hicolor/48x48" ]; then
    ok "U2 Symbolthema im Profil: org.querencia.Software.svg, keine 48x48-Stufe (Symbolregel)"
else nok "U2 Symbolthema im Profil"; fi
vtext=$("$BIN" --version 2>&1); rc=$?
[ $rc = 0 ] && case $vtext in *"Querencia Software 0.1.0"*) ok "F30 --version: „$vtext“" ;; *) nok "F30 --version: $vtext" ;; esac || nok "F30 --version Exit $rc"

# Keine schreibenden Aufrufe ------------------------------------------------------------
fall "Nur lesend: keine Transaktion gestartet, kein Simulieren/Ausführen versucht" \
    test "$(zahl '^(transaktion\tgestartet|nur-lesen\t)')" = 0

# F20 Zweite Instanz, F19 Beenden ----------------------------------------------------------
QS_NUR_LESEN=1 "$BIN" >>"$OUT/software2.log" 2>&1 &
PID2=$!
sleep 2
if running && ! kill -0 "$PID2" 2>/dev/null; then ok "F20 Zweiter Aufruf holt das vorhandene Fenster nach vorn (eine Instanz)"
else nok "F20 Zweite Instanz"; kill "$PID2" 2>/dev/null; fi
shot ende
beenden F19
abschluss
