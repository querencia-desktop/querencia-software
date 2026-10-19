#!/bin/bash
# Software 0.1.1, Modus „anlauf“: Bildfolge der Statuszeile während des Starts
# (Entstehung der Glyphenreste, Werkstatt QREL-05 W.5). Nur lesend.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt – Abbruch"; abschluss; exit 0; }
env QS_DEBUG=1 "$BIN" >"$LOG" 2>"$OUT/software.err" &
PID=$!
for i in $(seq -w 1 60); do grim -g "0,0 1000x720" "$OUT/anlauf-$i.png"; echo "$i $(date +%s.%N) $(grep -c '^status' "$LOG")" >>"$OUT/anlauf.txt"; sleep 0.05; done
grep -P '^(status|ansicht|zeit|layout\tende)' "$LOG" | head -20 >>"$OUT/anlauf.txt"
beenden "Strg+Q"
abschluss
