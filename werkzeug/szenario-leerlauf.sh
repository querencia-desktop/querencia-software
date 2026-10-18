#!/bin/bash
# SW-6, Modus „leerlauf“: Speicher nach dem Start und nach 10 Minuten Leerlauf
# (echte Backends, nur lesend). VmRSS/VmHWM aus /proc, Anzahl Threads.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt"; abschluss; exit 0; }
start_programm
warte '^zeit\tkatalog' 0 30; warte '^zeit\tpaketstand' 0 60; sleep 3
mem() { awk '/^(VmRSS|VmHWM|Threads)/{printf "%s %s ", $1, $2}' "/proc/$PID/status"; }
M0=$(mem); info "Nach dem Laden: $M0"
for m in 2 4 6 8 10; do sleep 120; info "Leerlauf $m min: $(mem)"; done
R0=$(echo "$M0" | grep -oP 'VmRSS: \K\d+'); R1=$(mem | grep -oP 'VmRSS: \K\d+')
info "VmRSS nach dem Laden ${R0} kB, nach 10 min ${R1} kB"
fall "Speicher nach 10 min Leerlauf: kein Wachstum über 10 % (${R0} → ${R1} kB)" test "$R1" -le $((R0 * 110 / 100))
fall "Programm läuft nach 10 min Leerlauf" running
CPU=$(awk '{print $14+$15}' "/proc/$PID/stat"); info "CPU-Zeit gesamt (utime+stime, Ticks): $CPU"
beenden F19
abschluss
