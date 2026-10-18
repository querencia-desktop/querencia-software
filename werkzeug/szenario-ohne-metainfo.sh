#!/bin/bash
# SW-6, Modus „ohne-metainfo“: Gegenprobe zu F22 – ohne MetaInfo-Dateien im
# Profil findet die Suche „querencia“ die eigenen Programme nicht. Nur lesend.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt"; abschluss; exit 0; }
start_programm
warte '^zeit\tkatalog' 0 30; sleep 1
KM=$(letzte '^katalog' | grep -o "$HMP04_PROFILE/data/metainfo/[^ ]*" | wc -l)
fall "Profil ohne MetaInfo: keine Datei aus \$XDG_DATA_HOME/metainfo gelesen" test "$KM" = 0
suche querencia 5
NQ=$(letzte '^ansicht' | cut -f3)
NQT=$(grep -P '^listenzeile' "$LOG" | tail -40 | awk -F'\t' '$4 ~ /^querencia-/' | wc -l)
info "Suche querencia ohne MetaInfo: $NQ Treffer, davon $NQT mit Paket querencia-*"
fall "Gegenprobe F22: ohne MetaInfo keine eigenen Programme (querencia-*) in der Suche" test "$NQT" = 0
shot suche-querencia-ohne-metainfo
beenden F19
abschluss
