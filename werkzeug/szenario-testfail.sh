#!/bin/bash
# SW-6, Modus „testfail“: echter packagekitd mit --backend=test_fail auf SW-1s
# Prüfbus. Suchen enden mit internal-error, Resolve/Details/Updates ohne Daten
# (PRUEFGERUEST.md). Prüft den Fehlerweg der Oberfläche gegen einen echten Daemon.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
case ${QS_PK_BUS:-} in unix:path=/tmp/qs-pk-sw6/*) ;; *) nok "Schutz: kein Prüfbus"; abschluss; exit 0 ;; esac
start_programm
running && ok "Start gegen packagekitd test_fail" || { nok "Start"; abschluss; exit 0; }
warte '^zeit\tkatalog' 0 30; warte '^zeit\tpaketstand' 0 30; warte '^seitentext' 0 10; sleep 1
read -r _ _ PMS PN PG < <(letzte '^zeit\tpaketstand' | tr '\t' ' ')
info "Resolve gegen test_fail: $PN Namen, $PG Pakete, $PMS ms"
fall "Katalog lädt unabhängig vom Paketdienst" passt_letzte '^ansicht' '^ansicht\talle\t[1-9]\d{3}\t$'
fall "Ohne Pakete: Katalogzeilen „Nicht verfügbar“, Knopf gesperrt" bash -c "grep -P '^listenzeile' '$LOG' | tail -5 | cut -f5 | sort -u | grep -qx 'Nicht verfügbar'"
fall "Aktualisierungen (0) bei leerer GetUpdates-Antwort" hat_letzte '^seitentext' $'seitentext\taktualisierungen\t0'
v=$(zahl '^ansicht'); menue 2 3; warte '^ansicht' "$v" 3
suche gtk 3
i=0; while [ $i -lt 100 ] && ! grep -qP '^leer\tSuche fehlgeschlagen' "$LOG"; do sleep 0.1; i=$((i + 1)); done
fall "Fehlerfall test_fail: Expertensuche meldet „Suche fehlgeschlagen: …“" grep -qP '^leer\tSuche fehlgeschlagen: ' "$LOG"
info "$(letzte '^leer')"
shot suche-fehlgeschlagen
fall "Kein Fehlerdialog, Programm läuft weiter" bash -c "[ $(zahl '^dialog\t') = 0 ] && kill -0 $PID"
beenden F19
abschluss
