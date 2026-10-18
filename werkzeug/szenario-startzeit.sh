#!/bin/bash
# SW-6, Modus „startzeit“: fünf Starts nacheinander (der erste kalt für den
# Katalog-Dateicache nur, wenn er vorher nicht gelesen wurde). Gemessen: erstes
# Layout (Fenster sichtbar), Katalog geladen, Paketstand aufgelöst – in ms ab
# Prozessstart, aus den `zeit`-Zeilen und der Zeit bis zur ersten Layoutzeile.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt"; abschluss; exit 0; }
for i in 1 2 3 4 5; do
    t0=$(date +%s%N)
    QS_DEBUG=1 "$BIN" >"$LOG" 2>>"$OUT/software.err" &
    PID=$!
    while ! grep -q '^layout	ende' "$LOG" 2>/dev/null; do sleep 0.01; done
    t1=$(date +%s%N)
    warte '^zeit\tkatalog' 0 30 >/dev/null; warte '^zeit\tpaketstand' 0 60 >/dev/null
    t2=$(date +%s%N)
    read -r _ _ KMS KSTART < <(letzte '^zeit\tkatalog' | tr '\t' ' ')
    read -r _ _ PMS _ _ < <(letzte '^zeit\tpaketstand' | tr '\t' ' ')
    F=$(((t1 - t0) / 1000000)); G=$(((t2 - t0) / 1000000))
    info "Start $i: Fenster $F ms, Katalog ${KMS} ms (fertig ${KSTART} ms nach Start), Resolve ${PMS} ms, bedienbar mit Paketstand $G ms"
    echo "$i $F $KMS $KSTART $PMS $G" >>"$OUT/startzeiten.txt"
    q "chord ctrl q" "wait 800"; wait "$PID" 2>/dev/null
    sleep 1
done
MF=$(sort -n -k2 "$OUT/startzeiten.txt" | awk 'NR==3{print $2}')
MK=$(sort -n -k4 "$OUT/startzeiten.txt" | awk 'NR==3{print $4}')
MG=$(sort -n -k6 "$OUT/startzeiten.txt" | awk 'NR==3{print $6}')
info "Median: Fenster $MF ms, Katalog geladen $MK ms, mit Paketstand $MG ms"
fall "Startzeit: Fenster sichtbar in < 1 s (Median $MF ms)" test "$MF" -lt 1000
fall "Startzeit: Katalog geladen in < 2 s nach Start (Median $MK ms)" test "$MK" -lt 2000
abschluss
