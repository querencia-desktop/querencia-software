#!/bin/bash
# SW-6: Zeigt das Live-Panel den Menüeintrag „Software“ mit eigenem Symbol?
# Läuft in isoliert.py (eigener labwc, Wegwerf-Profil). Fälle per PANEL_FALL:
#   vorher    Symbol (nur SVG bzw. SVG+PNG) liegt vor dem Panelstart im Profil
#   nachher   Symbol wird erst nach dem Panelstart kopiert
#   beruehrt  wie nachher, danach mtime von $XDG_DATA_HOME/icons/hicolor gesetzt
set -u
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"; ERG=$OUT/ergebnis.txt; : >"$ERG"
. "$P/eingabe.sh"
"$QA" "$R" >/dev/null 2>&1; q "size ${R%x*} ${R#*x}" "wait 300"
ICONS=$XDG_DATA_HOME/icons/hicolor
symbol_ablegen() {
    mkdir -p "$ICONS/scalable/apps"; cp "$SVG" "$ICONS/scalable/apps/org.querencia.Software.svg"
    if [ -d "${PNGS:-}" ]; then cp -r "$PNGS"/. "$ICONS/"; fi
}
for g in 16 22 24 32 64 128 256; do mkdir -p "$ICONS/${g}x${g}/apps"; done; mkdir -p "$ICONS/scalable/apps"   # wie live: alle Größenordner gibt es schon
[ "$PANEL_FALL" = vorher ] && symbol_ablegen
"${QS_PANEL:-$HOME/opt/querencia/bin/querencia-panel}" >"$OUT/panel.log" 2>&1 &
PANEL=$!; sleep 5
if [ "$PANEL_FALL" != vorher ]; then symbol_ablegen; sleep 1; fi
if [ "$PANEL_FALL" = beruehrt ]; then touch "$ICONS"; sleep 7; fi
grim "$OUT/0-panel.png"
q "move $SYS_X 15" "click" "wait 1200"; grim "$OUT/1-system.png"
q "move $((SYS_X + 40)) $((15 + SV_DY))" "wait 400" "click" "wait 1500"; grim "$OUT/2-systemverwaltung.png"
q "key Escape" "wait 300" "key Escape" "wait 300"
kill $PANEL 2>/dev/null
echo "fertig $PANEL_FALL" >>"$ERG"
touch "$HMP04_PROFILE/logs/FERTIG"
