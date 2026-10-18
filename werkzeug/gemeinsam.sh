# Gemeinsame Hilfen der SW-6-Szenarien (läuft IN der isolierten labwc-Sitzung).
# Übernommen aus SW-3s werkzeug/szenario.sh (Debug-Protokoll QS_DEBUG=1,
# virtuelle Eingaben über eine dauerhafte qeingabe-Sitzung).
set -u
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"
ERG=$OUT/ergebnis.txt; : >"$ERG"
LOG=$OUT/software.log
ok() { echo "PASS $*" | tee -a "$ERG"; }
nok() { echo "FAIL $*" | tee -a "$ERG"; }
blocked() { echo "BLOCKED $*" | tee -a "$ERG"; }
info() { echo "INFO $*" | tee -a "$ERG"; }
. "$P/eingabe.sh"
N=0
shot() { N=$((N + 1)); grim "$OUT/$(printf '%02d' $N)-$1.png"; }
running() { kill -0 "$PID" 2>/dev/null; }
W=${R%x*}; H=${R#*x}
fall() { # fall "<Name>" <Bedingung …>
    local name=$1; shift
    if "$@"; then ok "$name"; else nok "$name"; fi
}
zahl() { grep -c -P -- "$1" "$LOG" 2>/dev/null || true; }
warte() { # warte "<regex>" <vorher> [Sekunden]
    local muster=$1 vorher=$2 t=${3:-6} i=0
    while [ $i -lt $((t * 10)) ]; do
        [ "$(zahl "$muster")" -gt "$vorher" ] && return 0
        sleep 0.1; i=$((i + 1))
    done
    echo "   Zeitlimit beim Warten auf /$muster/" >>"$ERG"
    return 1
}
letzte() { grep -P -- "$1" "$LOG" | tail -1; }
hat_letzte() {
    local ist; ist=$(letzte "$1")
    [ "$ist" = "$2" ] && return 0
    printf '   erwartet: [%s]\n   ist:      [%s]\n' "$2" "$ist" >>"$ERG"; return 1
}
passt_letzte() { # passt_letzte "<regex Zeilenanfang>" "<regex ganze Zeile>"
    local ist; ist=$(letzte "$1")
    printf '%s' "$ist" | grep -qP -- "$2" && return 0
    printf '   erwartet: /%s/\n   ist:      [%s]\n' "$2" "$ist" >>"$ERG"; return 1
}
layout() { tac "$LOG" | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | tac; }
warte_layout() { local v i=0; v=$(zahl '^layout\tende'); while [ $i -lt 15 ] && [ "$(zahl '^layout\tende')" -le "$v" ]; do sleep 0.1; i=$((i + 1)); done; return 0; }
mitte_von() { layout | awk -F'\t' "$1 {printf \"%d %d\n\", \$(NF-3)+\$(NF-1)/2, \$(NF-2)+\$NF/2; exit}"; }
klick_auf() {
    local x y
    read -r x y < <(mitte_von "$1")
    if [ -z "${x:-}" ]; then echo "   keine Position für $2" >>"$ERG"; return 1; fi
    q "move $((OX + x)) $((OY + y))" "click" "wait 250"
}
seite() { klick_auf "\$1==\"seite\" && \$2==\"$1\"" "Seite $1"; }
zeile() { klick_auf "\$1==\"zeile\" && \$3==\"$1\"" "Zeile $1"; }
knopf_von() {
    local i
    i=$(layout | awk -F'\t' -v t="$1" '$1=="zeile" && $3==t {print $2; exit}')
    [ -n "$i" ] || { echo "   keine Zeile $1" >>"$ERG"; return 1; }
    klick_auf "\$1==\"knopf\" && \$2==\"$i\"" "Knopf von $1"
}
menue() { # menue <Kopf 0..3> <Eintrag 0..> [Trenner davor]
    local x y kx ky
    read -r kx ky < <(mitte_von "\$1==\"menue\" && \$2==\"$1\"")
    [ -n "${kx:-}" ] || { echo "   Menükopf $1 fehlt" >>"$ERG"; return 1; }
    q "move $((OX + kx)) $((OY + ky))" "click" "wait 900"
    x=$((kx + 20)); y=$((ky + 12 + 33 + 36 * $2 + 13 * ${3:-0}))
    q "move $((OX + x)) $((OY + y))" "wait 300" "click" "wait 600"
}
suche() { # suche <Text> [Sekunden]: Suchfeld leeren, tippen, auf Liste warten
    local v
    q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 450"
    v=$(zahl '^ansicht')
    q "type $1"
    warte '^ansicht\t.*\t'"$1"'$' 0 "${2:-3}" >/dev/null 2>&1 || warte '^ansicht' "$v" "${2:-3}"
    warte_layout >/dev/null
}
# Wartet, bis die letzte ansicht-Zeile zum Suchtext gehört und Zeilen hat
# (asynchrone PackageKit-Suche in der Expertenliste).
warte_treffer() { # warte_treffer <Text> <Sekunden>
    local i=0
    while [ $i -lt $(($2 * 10)) ]; do
        local l; l=$(letzte '^ansicht')
        [ "$(printf '%s' "$l" | cut -f4)" = "$1" ] && [ "$(printf '%s' "$l" | cut -f3)" -gt 0 ] && { warte_layout; return 0; }
        sleep 0.1; i=$((i + 1))
    done
    echo "   Zeitlimit: keine Treffer für $1" >>"$ERG"; return 1
}
# start_programm [Umgebung…]: Programm mit QS_DEBUG=1 starten, auf Layout warten,
# Fensterlage kalibrieren (OX/OY).
start_programm() {
    env QS_DEBUG=1 "$@" "$BIN" >"$LOG" 2>"$OUT/software.err" &
    PID=$!
    warte '^layout\tende' 0 15
    q "move 400 400" "wait 300" "move 420 410" "wait 400"
    read -r ZX ZY < <(grep '^zeiger' "$LOG" | tail -1 | cut -f2,3)
    if [ -n "${ZX:-}" ]; then OX=$((420 - ZX)); OY=$((410 - ZY)); KALIBRIERT=1
    else OX=21; OY=47; KALIBRIERT=0; fi
}
aufloesung() {
    "$QA" "$R" >"$OUT/ausgabe.log" 2>&1 && ok "U1 Auflösung $R gesetzt" || blocked "U1 Auflösung $R nicht setzbar"
    q "size $W $H" "wait 500"
}
beenden() {
    q "chord ctrl q" "wait 1500"
    if running; then nok "${1:-F19} Strg+Q beendet nicht"; kill "$PID"; else
        wait "$PID"; local ec=$?
        [ "$ec" = 0 ] && ok "${1:-F19} Strg+Q beendet das Programm (Exit 0)" || nok "${1:-F19} Exit $ec beim Beenden"
    fi
}
abschluss() {
    grep -i -E 'critical|warning|error' "$OUT/software.err" | grep -v -E 'VK_ERROR_SURFACE_LOST|libEGL' | head -20 >"$OUT/gtk-meldungen.txt" || true
    [ -s "$OUT/gtk-meldungen.txt" ] && echo "HINW GTK-/stderr-Meldungen (ohne Vulkan/EGL der headless-Umgebung): $(wc -l <"$OUT/gtk-meldungen.txt")" | tee -a "$ERG"
    echo "Ergebnis $MODUS $R: $(grep -c '^PASS' "$ERG") PASS, $(grep -c '^FAIL' "$ERG") FAIL, $(grep -c '^BLOCKED' "$ERG") BLOCKED" | tee -a "$ERG"
    touch "$HMP04_PROFILE/logs/FERTIG"
}
