#!/bin/bash
# Software 0.1.1, Modus „glyphen“: Statuszeile nach Ansichtswechseln (Werkstatt
# QREL-05 W.5, wie Editor E.7). Ablauf wie in der Werkstatt: Start, Suche „gimp“,
# Kategorien Grafik und Internet, Detailseite GIMP, Installiert,
# Aktualisierungen, Expertenliste. Nach jedem Schritt wird die Statuszeile
# ausgeschnitten. Bezug ist die Statuszeile nach Maximieren und
# Wiederherstellen (beides zeichnet das Fenster vollständig neu); verglichen
# wird jeder Schritt mit gleichem Statustext. Nur lesend (QS_NUR_LESEN=1).
# Sinnvoll mit QS_GPU=1 (Renderer wie live): Die Reste entstehen nur bei den
# GPU-Renderern von GTK 4.22 und nicht bei jedem Lauf.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt – Abbruch"; abschluss; exit 0; }
[ -z "${QS_PK_BUS:-}" ] || { nok "Schutz: QS_PK_BUS gesetzt – falscher Modus"; abschluss; exit 0; }
info "Renderer: GSK_RENDERER=${GSK_RENDERER:-Standard}, labwc ${WLR_RENDERER:-?}"

start_programm
running && ok "G0 Start ($R)" || { nok "G0 Start"; abschluss; exit 0; }
warte '^zeit\tkatalog' 0 30
warte '^zeit\tpaketstand' 0 60
sleep 2; warte_layout >/dev/null
read -r FW FH < <(letzte '^fenster' | cut -f2,3)
# Lage der Statuszeile: ab 0.1.1 aus der Debug-Ausgabe, sonst die untersten
# 21 Bildpunkte des Fensterinhalts.
statusrechteck() {
    local z; z=$(layout | awk -F'\t' '$1=="statuszeile"{print $2, $3, $4, $5; exit}')
    if [ -n "$z" ]; then read -r SX SY SB SH <<<"$z"; else SX=0; SY=$((FH - 21)); SB=$FW; SH=21; fi
}
statusrechteck
info "Statuszeile bei $SX,$SY ${SB}×$SH (Fensterinhalt ab $OX,$OY)"
STATUS0=$(letzte '^status\t')
info "Statustext: ${STATUS0#status	}"

SCHRITTE=""
schritt() { # schritt <Name>: Bild und Ausschnitt der Statuszeile
    local name=$1 bild
    sleep 1.2
    shot "$name"
    bild=$(ls "$OUT"/[0-9][0-9]-"$name".png | tail -1)
    python3 -I "$E/werkzeug/ausschnitt.py" "$bild" $((OX + SX)) $((OY + SY)) "$SB" "$SH" "$OUT/status-$name.png" >/dev/null
    [ "$(letzte '^status\t')" = "$STATUS0" ] && SCHRITTE="$SCHRITTE $name" || info "$name: Statustext geändert ($(letzte '^status\t' | cut -f2-)), kein Vergleich"
}

schritt start
TX=$((OX + FW / 2)); TY=$((OY - 14))
# Verschieben und zurück: labwc setzt das Fenster aus dem Puffer des Clients
# neu zusammen, GTK zeichnet dabei nichts neu. Stimmt die Statuszeile danach,
# lag der Rest nur im Bild des Compositors, nicht im Puffer von GTK.
q "move $TX $TY" "press" "glide $((TX + 60)) $((TY + 40)) 400" "release" "wait 600"
q "move $((TX + 60)) $((TY + 40))" "press" "glide $TX $TY 400" "release" "wait 600" "move 400 400" "wait 300"
schritt verschoben
suche gimp 5
schritt suche-gimp
v=$(zahl '^ansicht'); seite "kategorie:Grafik"; warte '^ansicht' "$v" 3; warte_layout >/dev/null
schritt kategorie-grafik
v=$(zahl '^ansicht'); seite "kategorie:Internet"; warte '^ansicht' "$v" 3; warte_layout >/dev/null
schritt kategorie-internet
v=$(zahl '^ansicht'); seite alle; warte '^ansicht' "$v" 3; warte_layout >/dev/null
v=$(zahl '^paketdetails')
zeile "GNU Image Manipulation Program"; q "key Return" "wait 600"
warte '^paketdetails' "$v" 20; warte_layout >/dev/null
schritt details-gimp
q "key Escape" "wait 400" "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"
v=$(zahl '^ansicht'); seite installiert; warte '^ansicht' "$v" 3; warte_layout >/dev/null
schritt installiert
v=$(zahl '^ansicht'); seite aktualisierungen; warte '^ansicht' "$v" 3; warte_layout >/dev/null
schritt aktualisierungen
v=$(zahl '^ansicht'); seite experten; warte '^ansicht' "$v" 3; warte_layout >/dev/null
schritt expertenliste
v=$(zahl '^ansicht'); seite alle; warte '^ansicht' "$v" 3; warte_layout >/dev/null
schritt alle

# Bezug: Doppelklick auf die Titelleiste maximiert, ein zweiter stellt wieder her.
q "move $TX $TY" "dclick" "wait 1500"
q "move $((TX * 2)) 14" "dclick" "wait 1500" "move 400 400" "wait 300"
sleep 1.2
shot bezug
BILD=$(ls "$OUT"/[0-9][0-9]-bezug.png | tail -1)
python3 -I "$E/werkzeug/ausschnitt.py" "$BILD" $((OX + SX)) $((OY + SY)) "$SB" "$SH" "$OUT/status-bezug.png" >/dev/null
if [ "$(letzte '^status\t')" = "$STATUS0" ]; then
    for name in $SCHRITTE; do
        a=$(python3 -I "$E/werkzeug/ausschnitt.py" "$OUT/status-$name.png" 0 0 "$SB" "$SH" "$OUT/vergleich-$name.png" "$OUT/status-bezug.png")
        fall "Statuszeile nach „$name“ gleich der vollständig neu gezeichneten ($a)" test "$a" = "abweichung 0"
    done
else
    blocked "Statustext am Ende geändert – kein Bezug"
fi

# Strg+W: Einfensterprogramm wie Rechner und Systemmonitor, kein Kürzel.
q "chord ctrl w" "wait 1200"
fall "Strg+W lässt das Fenster offen (kein Kürzel, wie Rechner/Systemmonitor)" running
beenden "Strg+Q"
abschluss
