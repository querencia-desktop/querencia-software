#!/bin/bash
# Rauchtest (Entwicklung): Start, Layout, Menübilder.
set -u
. "$HMP04_PROFILE/env.sh"
E=$OUT/ergebnis.txt; : >"$E"
. "$(dirname "$0")/eingabe.sh"
W=${R%x*}; H=${R#*x}
"$QA" "$R" >"$OUT/ausgabe.log" 2>&1
q "size $W $H" "wait 300"
QS_ATTRAPPE=1 QS_DEBUG=1 "$BIN" >"$OUT/software.log" 2>"$OUT/software.err" &
PID=$!; sleep 4
grim "$OUT/r01-start.png"
q "move 300 300" "wait 300" "move 320 310" "wait 400"
read -r ZX ZY < <(grep '^zeiger' "$OUT/software.log" | tail -1 | cut -f2,3)
OX=$((320 - ZX)); OY=$((310 - ZY)); echo "Ursprung $OX $OY" >>"$E"
for i in 0 2; do
  read -r MX MY < <(awk -F'\t' -v i=$i '$1=="menue" && $2==i {printf "%d %d\n", $3+$5/2, $4+$6/2}' "$OUT/software.log" | tail -1)
  q "move $((OX+MX)) $((OY+MY))" "click" "wait 1000"; grim "$OUT/r02-menue$i.png"; q "key Escape" "wait 400"
done
q "chord ctrl q" "wait 1000"
kill -0 $PID 2>/dev/null && echo "läuft noch" >>"$E"
touch "$HMP04_PROFILE/logs/FERTIG"
