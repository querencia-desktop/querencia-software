#!/bin/bash
# Läuft IN der isolierten labwc-Sitzung (isoliert.py). Prüft Querencia
# Software gegen die Attrappen (QS_ATTRAPPE=1) mit virtuellen Eingaben (Maus
# und Tastatur) und Bildschirmfotos. Zustände werden aus der Debug-Ausgabe
# des Programms (QS_DEBUG=1) gelesen, nicht aus den Bildern.
set -u
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"
E=$OUT/ergebnis.txt; : >"$E"
LOG=$OUT/software.log
ok() { echo "PASS $*" | tee -a "$E"; }
nok() { echo "FAIL $*" | tee -a "$E"; }
blocked() { echo "BLOCKED $*" | tee -a "$E"; }
. "$(dirname "$0")/eingabe.sh"
N=0
shot() { N=$((N + 1)); grim "$OUT/$(printf '%02d' $N)-$1.png"; }
running() { kill -0 "$PID" 2>/dev/null; }
W=${R%x*}; H=${R#*x}
fall() { # fall "<Name>" <Bedingung …>  → PASS/FAIL
    local name=$1; shift
    if "$@"; then ok "$name"; else nok "$name"; fi
}

# Zeilen zählen / warten -------------------------------------------------
zahl() { grep -c -P -- "$1" "$LOG" 2>/dev/null || true; }
# warte "<regex>" <vorher> [Sekunden]: bis mehr Treffer als <vorher> da sind
warte() {
    local muster=$1 vorher=$2 t=${3:-6} i=0
    while [ $i -lt $((t * 10)) ]; do
        [ "$(zahl "$muster")" -gt "$vorher" ] && return 0
        sleep 0.1; i=$((i + 1))
    done
    echo "   Zeitlimit beim Warten auf /$muster/" >>"$E"
    return 1
}
letzte() { grep -P -- "$1" "$LOG" | tail -1; }
hat_letzte() { # hat_letzte "<regex für Zeilenanfang>" "<erwartet>"
    local ist; ist=$(letzte "$1")
    [ "$ist" = "$2" ] && return 0
    printf '   erwartet: [%s]\n   ist:      [%s]\n' "$2" "$ist" >>"$E"; return 1
}

# Layout: letzter vollständiger Block "layout anfang" … "layout ende"
layout() { tac "$LOG" | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | tac; }
# Wartet auf einen neuen Layoutblock (höchstens 1,5 s; nicht jede Aktion
# erzeugt einen, dann gilt der letzte).
warte_layout() { local v i=0; v=$(zahl '^layout\tende'); while [ $i -lt 15 ] && [ "$(zahl '^layout\tende')" -le "$v" ]; do sleep 0.1; i=$((i + 1)); done; return 0; }
mitte_von() { # mitte_von <awk-Bedingung>  → "x y" (Fensterkoordinaten)
    layout | awk -F'\t' "$1 {printf \"%d %d\n\", \$(NF-3)+\$(NF-1)/2, \$(NF-2)+\$NF/2; exit}"
}
klick_auf() { # klick_auf "<awk-Bedingung>" "<Beschreibung>"
    local x y
    read -r x y < <(mitte_von "$1")
    if [ -z "${x:-}" ]; then echo "   keine Position für $2" >>"$E"; return 1; fi
    q "move $((OX + x)) $((OY + y))" "click" "wait 250"
}
seite() { klick_auf "\$1==\"seite\" && \$2==\"$1\"" "Seite $1"; }
zeile() { klick_auf "\$1==\"zeile\" && \$3==\"$1\"" "Zeile $1"; }
knopf_von() { # Knopf in der Zeile mit Titel $1
    local i
    i=$(layout | awk -F'\t' -v t="$1" '$1=="zeile" && $3==t {print $2; exit}')
    [ -n "$i" ] || { echo "   keine Zeile $1" >>"$E"; return 1; }
    klick_auf "\$1==\"knopf\" && \$2==\"$i\"" "Knopf von $1"
}
menue() { # menue <Kopf 0..3> <Eintrag 0..> [Trenner davor]
    local x y kx ky
    read -r kx ky < <(mitte_von "\$1==\"menue\" && \$2==\"$1\"")
    [ -n "${kx:-}" ] || { echo "   Menükopf $1 fehlt" >>"$E"; return 1; }
    q "move $((OX + kx)) $((OY + ky))" "click" "wait 900"
    # Einträge: erster bei Kopfunterkante + 33 px, Abstand 36 px; ein
    # Abschnittstrenner kostet 13 px (gemessen im Rauchtest, Bild r02-*).
    x=$((kx + 20)); y=$((ky + 12 + 33 + 36 * $2 + 13 * ${3:-0}))
    q "move $((OX + x)) $((OY + y))" "wait 300" "click" "wait 600"
}
suche() { # Suchfeld leeren und Text tippen, auf Liste warten
    local v; v=$(zahl '^ansicht')
    q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 450"
    v=$(zahl '^ansicht')
    q "type $1"
    warte '^ansicht\t.*\t'"$1"'$' 0 3 >/dev/null 2>&1 || warte '^ansicht' "$v" 3
    warte_layout >/dev/null
}

# Auflösung ---------------------------------------------------------------
"$QA" "$R" >"$OUT/ausgabe.log" 2>&1 && ok "Auflösung $R gesetzt" || blocked "Auflösung $R nicht setzbar"
q "size $W $H" "wait 500"

# 1 Start -----------------------------------------------------------------
QS_ATTRAPPE=1 QS_DEBUG=1 "$BIN" >"$LOG" 2>"$OUT/software.err" &
PID=$!
warte '^layout\tende' 0 10
running && ok "Start ($R)" || { nok "Start"; touch "$HMP04_PROFILE/logs/FERTIG"; exit 0; }
sleep 0.5
shot start

# Kalibrierung: Fensterlage aus einer Zeigermeldung
q "move 400 400" "wait 300" "move 420 410" "wait 400"
read -r ZX ZY < <(grep '^zeiger' "$LOG" | tail -1 | cut -f2,3)
if [ -n "${ZX:-}" ]; then OX=$((420 - ZX)); OY=$((410 - ZY)); ok "Kalibrierung: Fensterinhalt beginnt bei ($OX, $OY)"
else OX=21; OY=47; blocked "Kalibrierung: keine Zeigermeldung, Annahme ($OX, $OY)"; fi

read -r FW FH < <(letzte '^fenster' | cut -f2,3)
fall "Fenstergröße 960×640 (ist ${FW:-?}×${FH:-?})" test "${FW:-0}" = 960 -a "${FH:-0}" = 640
fall "Katalog geladen: Alle Programme mit 30 Einträgen" hat_letzte '^ansicht' $'ansicht\talle\t30\t'
fall "Statuszeile: 30 Programme, 8 installiert, Prüfzeit" \
    grep -qP '^status\tKatalog: 30 Programme, 8 installiert · letzte Aktualisierungsprüfung \d\d:\d\d' "$LOG"
fall "Seitenleiste: 14 Einträge (Alle, 10 Kategorien, Installiert, Aktualisierungen, Expertenliste)" \
    test "$(layout | grep -c '^seite')" = 14
fall "Seitenleiste: „Aktualisierungen (3)“" hat_letzte '^seitentext' $'seitentext\taktualisierungen\t3'

# 2 Kategorie wechseln ----------------------------------------------------
v=$(zahl '^ansicht'); seite "kategorie:Grafik"; warte '^ansicht' "$v"; warte_layout
fall "Kategorie Grafik: 3 Programme" hat_letzte '^ansicht' $'ansicht\tkategorie:Grafik\t3\t'
fall "Kategorie Grafik zeigt GIMP, Inkscape, Bildbetrachter" \
    test "$(layout | awk -F'\t' '$1=="zeile"{print $3}' | sort | tr '\n' ,)" = "Bildbetrachter,GIMP,Inkscape,"
shot kategorie-grafik
leer=0
for k in Zubehör Büro Grafik Internet Unterhaltungsmedien Spiele Entwicklung Bildung Systemwerkzeuge Sonstiges; do
    v=$(zahl '^ansicht'); seite "kategorie:$k"; warte '^ansicht' "$v" 3
    n=$(letzte '^ansicht' | cut -f3)
    [ "$(letzte '^ansicht' | cut -f2)" = "kategorie:$k" ] && [ "${n:-0}" -gt 0 ] || { leer=1; echo "   $k: [$(letzte '^ansicht')]" >>"$E"; }
done
fall "Alle 10 Kategorien per Mausklick, keine leer" test $leer = 0
warte_layout >/dev/null

# 3 Suche ------------------------------------------------------------------
v=$(zahl '^ansicht'); seite alle; warte '^ansicht' "$v"; warte_layout >/dev/null
v=$(zahl '^fokus\tsuche'); q "chord ctrl f" "wait 300"
fall "Strg+F setzt den Fokus ins Suchfeld" test "$(zahl '^fokus\tsuche')" -gt "$v"
v=$(zahl '^ansicht')
q "type gimp" "wait 120"
fall "Sofortsuche wartet (kein Neuaufbau 120 ms nach dem Tippen)" test "$(zahl '^ansicht')" = "$v"
warte '^ansicht' "$v" 3; sleep 0.3
fall "Sofortsuche „gimp“: 1 Treffer" hat_letzte '^ansicht' $'ansicht\talle\t1\tgimp'
warte_layout >/dev/null; shot suche-gimp
v=$(zahl '^paketdetails')
q "key Return" "wait 500"
fall "Eingabetaste im Suchfeld öffnet die Detailseite" hat_letzte '^details' $'details\tGIMP\tInstalliert\tEntfernen'
warte '^paketdetails' "$v" 3
fall "Detailseite lädt Paketdetails (Größe)" hat_letzte '^paketdetails' $'paketdetails\tgimp\t122880000'
warte_layout >/dev/null; shot details-gimp
fall "Detailseite: Homepage als Link vorhanden" bash -c "grep -q '^homepage' <(tac '$LOG' | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f')"
q "key Escape" "wait 400"
fall "Escape führt von der Detailseite zurück zur Liste" hat_letzte '^details' $'details\tzu'
v=$(zahl '^ansicht'); q "key Escape" "wait 500"
fall "Escape in der Liste leert die Suche" hat_letzte '^ansicht' $'ansicht\talle\t30\t'
warte_layout >/dev/null

# Zeile anklicken + Eingabetaste
suche minen
zeile Minen; q "key Return" "wait 500"
fall "Zeile anklicken, Eingabetaste öffnet Details" hat_letzte '^details' $'details\tMinen\tVerfügbar\tInstallieren'
warte_layout >/dev/null

# 4 Installieren auf der Detailseite --------------------------------------
v=$(zahl '^transaktion\tfertig')
klick_auf '$1=="aktion"' "Knopf Installieren"
sleep 1.0; shot installieren-fortschritt
fall "Fortschritt läuft (Zwischenstand gemeldet)" grep -qP '^transaktion\tlaeuft\t(4|5|6)\d\t' "$LOG"
warte '^transaktion\tfertig' "$v" 5
fall "Installieren: Fortschritt bis 100 % und Fertig" bash -c "grep -qP '^transaktion\tlaeuft\t100\t' '$LOG' && grep -qP '^transaktion\tfertig\tgnome-mines' '$LOG'"
sleep 0.6
fall "Zustand wechselt: Minen ist installiert, Knopf „Entfernen“" hat_letzte '^details' $'details\tMinen\tInstalliert\tEntfernen'
fall "Statuszeile meldet „Minen wurde installiert.“" grep -qP '^status\t.*9 installiert.*Minen wurde installiert\.' "$LOG"
shot installiert
q "key Escape" "wait 300"

# 5 Fenster bleibt bedienbar während des Vorgangs --------------------------
suche audacity
v=$(zahl '^transaktion\tfertig')
knopf_von Audacity; sleep 0.4
vs=$(zahl '^ansicht'); seite "kategorie:Spiele"; warte '^ansicht' "$vs" 2
f_vor=$(zahl '^transaktion\tfertig')
fall "Fortschritt nicht modal: Kategorie wechselt während der Installation" \
    test "$(letzte '^ansicht' | cut -f2)" = "kategorie:Spiele" -a "$f_vor" = "$v"
shot bedienbar-waehrend-vorgang
warte '^transaktion\tfertig' "$v" 5
fall "Audacity danach installiert" grep -qP '^transaktion\tfertig\taudacity' "$LOG"

# 6 Abbrechen ----------------------------------------------------------------
v=$(zahl '^ansicht'); seite alle; warte '^ansicht' "$v"; warte_layout >/dev/null
suche thunderbird
knopf_von Thunderbird; sleep 0.7
klick_auf '$1=="abbrechen"' "Abbrechen"
sleep 0.6
fall "Abbrechen in der Fortschrittsleiste bricht ab" hat_letzte '^transaktion' $'transaktion\tabgebrochen'
sleep 0.8; warte_layout >/dev/null
fall "Thunderbird bleibt verfügbar" bash -c "! grep -qP '^transaktion\tfertig\tthunderbird' '$LOG'"

# 7 Entfernen mit Bestätigung: erst Abbruch, dann bestätigen ----------------
suche zeichentabelle
vd=$(zahl '^dialog\tentfernen')
knopf_von Zeichentabelle
warte '^dialog\tentfernen' "$vd" 3; sleep 0.6
fall "Bestätigungsdialog vor dem Entfernen erscheint" test "$(zahl '^dialog\tentfernen')" -gt "$vd"
fall "Simulation: gucharmap-data und gucharmap-plugins gehen mit" \
    grep -qP '^transaktion\tsimuliert\tgucharmap-data,gucharmap-plugins,gucharmap$' "$LOG"
shot entfernen-bestaetigen
q "key Escape" "wait 700"
fall "Entfernen-Dialog: Escape bricht ab" bash -c "grep -qP '^dialog\tzu\tentfernen' '$LOG' && [ \"\$(grep -P '^transaktion' '$LOG' | tail -1)\" = \$'transaktion\tabgebrochen' ]"
fall "Zeichentabelle bleibt installiert" bash -c "! grep -qP '^transaktion\tfertig\tgucharmap' '$LOG'"
warte_layout >/dev/null
v=$(zahl '^transaktion\tfertig'); vd=$(zahl '^dialog\tentfernen')
knopf_von Zeichentabelle
warte '^dialog\tentfernen' "$vd" 3; sleep 0.6
q "key Return"
warte '^transaktion\tfertig' "$v" 5
fall "Entfernen bestätigt (Eingabetaste): Fertig" grep -qP '^transaktion\tfertig\tgucharmap' "$LOG"
sleep 0.6
fall "Statuszeile: 9 installiert nach dem Entfernen" grep -qP '^status\t.*9 installiert.*Zeichentabelle wurde entfernt\.' "$LOG"

# 8 SperreBelegt -------------------------------------------------------------
suche schach
vd=$(zahl '^dialog\tsperre'); v=$(zahl '^transaktion\tfertig')
knopf_von Schach
warte '^dialog\tsperre' "$vd" 4; sleep 0.8
fall "SperreBelegt: Dialog „Paketverwaltung belegt“" test "$(zahl '^dialog\tsperre')" -gt "$vd"
shot sperre-belegt
running && ok "Programm läuft mit offenem Sperre-Dialog" || nok "Absturz beim Sperre-Dialog"
q "key Return"
warte '^transaktion\tfertig' "$v" 5
fall "Sperre-Dialog „Warten“: zweiter Versuch installiert Schach" grep -qP '^transaktion\tfertig\tgnome-chess' "$LOG"
fall "Belegte Sperre zeigt keinen Fehlerdialog" test "$(zahl '^dialog\tfehler')" = 0

# 9 KeinAgent ------------------------------------------------------------------
suche virtuelle
vd=$(zahl '^dialog\tkein-agent')
knopf_von "Virtuelle Maschinen"
warte '^dialog\tkein-agent' "$vd" 3; sleep 0.6
fall "KeinAgent: Erklärdialog „Berechtigungsdialog fehlt“" test "$(zahl '^dialog\tkein-agent')" -gt "$vd"
fall "KeinAgent: Statuszeile nennt den polkit-Agenten" grep -qP '^status\t.*Virtuelle Maschinen: Es fehlt der Berechtigungsdialog der Sitzung \(polkit-Agent\)' "$LOG"
shot kein-agent
q "key Return" "wait 600"
fall "KeinAgent-Dialog per Eingabetaste (Fokus) geschlossen" grep -qP '^dialog\tzu\tkein-agent' "$LOG"

# 10 NichtBerechtigt -------------------------------------------------------------
suche wireshark
vd=$(zahl '^dialog\tnicht-berechtigt')
knopf_von Wireshark
warte '^dialog\tnicht-berechtigt' "$vd" 3; sleep 0.6
fall "NichtBerechtigt: Meldung „Keine Berechtigung“" test "$(zahl '^dialog\tnicht-berechtigt')" -gt "$vd"
shot nicht-berechtigt
q "key Escape" "wait 600"
fall "NichtBerechtigt-Meldung per Escape geschlossen" grep -qP '^dialog\tzu\tnicht-berechtigt' "$LOG"
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"

# 11 Aktualisierungen -------------------------------------------------------------
v=$(zahl '^ansicht'); seite aktualisierungen; warte '^ansicht' "$v"; warte_layout >/dev/null
fall "Aktualisierungen-Ansicht: 3 Einträge" hat_letzte '^ansicht' $'ansicht\taktualisierungen\t3\t'
shot aktualisierungen
v=$(zahl '^transaktion\tfertig')
knopf_von Firefox
warte '^transaktion\tfertig' "$v" 5; sleep 0.8
fall "Firefox aktualisiert" grep -qP '^transaktion\tfertig\tfirefox' "$LOG"
fall "Aktualisierungen-Ansicht danach: 2 Einträge" hat_letzte '^ansicht' $'ansicht\taktualisierungen\t2\t'
fall "Seitenleiste: „Aktualisierungen (2)“" hat_letzte '^seitentext' $'seitentext\taktualisierungen\t2'

# 12 Menü Ansicht → Installiert / Expertenliste (Maus) ----------------------------
v=$(zahl '^ansicht'); menue 2 1; warte '^ansicht' "$v" 3
fall "Menü Ansicht → Installiert: 10 Programme" hat_letzte '^ansicht' $'ansicht\tinstalliert\t10\t'
warte_layout >/dev/null
v=$(zahl '^ansicht'); menue 2 3; warte '^ansicht' "$v" 3
fall "Menü Ansicht → Expertenliste: 34 Pakete" hat_letzte '^ansicht' $'ansicht\texperten\t34\t'
warte_layout >/dev/null; shot expertenliste
suche lib
fall "Expertenliste: Suche „lib“ findet Pakete" bash -c "[ \"\$(grep -P '^ansicht' '$LOG' | tail -1 | cut -f2,4)\" = \$'experten\tlib' ] && [ \"\$(grep -P '^ansicht' '$LOG' | tail -1 | cut -f3)\" -gt 0 ]"
v=$(zahl '^details')
zeile libqs-demo1; q "key Return" "wait 500"
fall "Expertenliste: Details eines reinen Pakets" hat_letzte '^details' $'details\tlibqs-demo1\tVerfügbar\tInstallieren'
q "key Escape" "wait 300" "key Escape" "wait 400"
v=$(zahl '^ansicht'); menue 2 0; warte '^ansicht' "$v" 3
fall "Menü Ansicht → Alle Programme" hat_letzte '^ansicht' $'ansicht\talle\t30\t'
warte_layout >/dev/null

# 13 Datei → Aktualisierungen prüfen, Bearbeiten → Suchen ---------------------------
menue 0 0; sleep 0.4
fall "Menü Datei → Aktualisierungen prüfen" grep -qP '^status\t.*Aktualisierungen geprüft: 2 verfügbar' "$LOG"
v=$(zahl '^fokus\tsuche'); q "move $((OX + 500)) $((OY + 300))" "click" "wait 300"
menue 1 0
fall "Menü Bearbeiten → Suchen fokussiert das Suchfeld" test "$(zahl '^fokus\tsuche')" -gt "$v"

# 14 Info-Dialog, mit Tastaturfokus schließen (Symbolabsturz-Muster) -----------------
for weg in Eingabetaste Escape Alt+S; do
    vd=$(zahl '^dialog\tinfo'); vz=$(zahl '^dialog\tzu\tinfo')
    menue 3 0
    warte '^dialog\tinfo' "$vd" 3; sleep 0.5
    [ "$weg" = Eingabetaste ] && shot info-dialog
    case $weg in
        Eingabetaste) q "key Return" ;;
        Escape) q "key Escape" ;;
        Alt+S) q "chord alt s" ;;
    esac
    q "wait 800"
    warte '^dialog\tzu\tinfo' "$vz" 2
    v=$(zahl '^ansicht')
    q "chord ctrl f" "type mine" "wait 700"
    sleep 0.3
    if running && [ "$(letzte '^ansicht' | cut -f4)" = mine ]; then
        ok "Info-Dialog per $weg geschlossen, danach Tastatur ohne Absturz"
    else nok "Info-Dialog per $weg schließen / Folgetasten ($(letzte '^ansicht'))"; fi
    q "chord ctrl a" "key BackSpace" "wait 500"
done
shot nach-info

# 15 Zweite Instanz ------------------------------------------------------------
QS_ATTRAPPE=1 "$BIN" >>"$OUT/software2.log" 2>&1 &
PID2=$!
sleep 2
if running && ! kill -0 "$PID2" 2>/dev/null; then ok "Zweiter Aufruf holt das vorhandene Fenster nach vorn (eine Instanz)"
else nok "Zweite Instanz"; kill "$PID2" 2>/dev/null; fi

# 16 Beenden -----------------------------------------------------------------------
q "chord ctrl q" "wait 1500"
if running; then nok "Strg+Q beendet nicht"; kill "$PID"; else
    wait "$PID"; EC=$?
    [ "$EC" = 0 ] && ok "Strg+Q beendet das Programm (Exit 0)" || nok "Exit $EC beim Beenden"
fi
grep -i -E 'critical|warning|error' "$OUT/software.err" | grep -v -E 'VK_ERROR_SURFACE_LOST|libEGL' | head -20 >"$OUT/gtk-meldungen.txt"
[ -s "$OUT/gtk-meldungen.txt" ] && echo "HINW GTK-Meldungen (ohne Vulkan/EGL der headless-Umgebung): $(wc -l <"$OUT/gtk-meldungen.txt")" | tee -a "$E"
echo "Ergebnis $R: $(grep -c '^PASS' "$E") PASS, $(grep -c '^FAIL' "$E") FAIL, $(grep -c '^BLOCKED' "$E") BLOCKED" | tee -a "$E"
touch "$HMP04_PROFILE/logs/FERTIG"
