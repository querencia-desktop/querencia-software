#!/bin/bash
# Rauchlauf (Entwicklung): echte Backends nur lesend, Zahlen sammeln.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
start_programm
warte '^zeit\tkatalog' 0 30
warte '^zeit\tpaketstand' 0 60
sleep 2
shot start
for t in firefox querencia gimp; do suche "$t" 5; sleep 0.5; info "suche $t: $(letzte '^ansicht')"; layout | grep -P '^(zeile|zeilensymbol)' | head -12 >>"$ERG"; shot "suche-$t"; done
grep -P '^(backends|katalog|zeit|status|seitentext)' "$LOG" >>"$ERG"
beenden
abschluss
