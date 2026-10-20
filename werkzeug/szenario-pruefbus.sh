#!/bin/bash
# SW-6, Modus „pruefbus“: integriertes Programm gegen SW-1s Prüfgerüst
# (qs-pk-attrappe als org.freedesktop.PackageKit auf einem privaten Bus unter
# /tmp/qs-pk-sw6, polkit-Attrappe „erlauben“) – schreibende Vorgänge ohne ein
# echtes Paket. Katalog: AppStream des Systems + MetaInfo im Profil.
# Benachrichtigungsdienst (Live-Binary) auf dem isolierten Sitzungsbus.
#
# Die Attrappe hat einen festen Bestand (PRUEFGERUEST.md): nach einer
# „Installation“ meldet sie das Paket weiter als verfügbar.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
case ${QS_PK_BUS:-} in unix:path=/tmp/qs-pk-sw6/*) ;; *) nok "Schutz: QS_PK_BUS zeigt nicht auf den Prüfbus ($QS_PK_BUS) – Abbruch"; abschluss; exit 0 ;; esac

# Benachrichtigungsdienst auf dem isolierten Bus, Nicht stören an ---------------
BN=${QS_BENACHRICHTIGUNGEN:-$HOME/opt/querencia/bin/querencia-benachrichtigungen}
bn_zustand() { timeout 10 gdbus call --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications --method org.querencia.Benachrichtigungen.Zustand 2>&1; }
if [ -x "$BN" ]; then
    "$BN" >"$OUT/benachrichtigungen.log" 2>&1 &
    BNPID=$!
    for _ in $(seq 1 50); do gdbus introspect --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications >/dev/null 2>&1 && break; sleep 0.1; done
    z=$(timeout 10 gdbus call --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications --method org.querencia.Benachrichtigungen.SetzeNichtStoeren true 2>&1; bn_zustand)
    case $z in *"(true,"*) ok "F27a Benachrichtigungsdienst auf dem isolierten Bus: Nicht stören gesetzt, Zustand() = $(echo "$z" | tail -1)" ;; *) blocked "F27a Benachrichtigungsdienst: $z" ;; esac
else BNPID=; blocked "F27 Benachrichtigungsdienst nicht unter $BN"; fi

start_programm
running && ok "Start gegen den Prüfbus ($R)" || { nok "Start"; abschluss; exit 0; }
passt_letzte '^backends' '^backends\techt\tunix:path=/tmp/qs-pk-sw6/sock\tschreibend$' && ok "Backends: echt, Prüfbus, schreibend" || nok "Backends ($(letzte '^backends'))"
warte '^zeit\tkatalog' 0 30
warte '^zeit\tpaketstand' 0 30
read -r _ _ PMS PN PG < <(letzte '^zeit\tpaketstand' | tr '\t' ' ')
info "Resolve gegen die Attrappe: $PN Namen, $PG Pakete, $PMS ms"
fall "Paketstand: Resolve liefert die Katalogpakete aus dem Attrappen-Bestand" test "${PG:-0}" -ge 3
fall "Wiederanbindung beim Start: keine laufende Transaktion, keine Leiste" bash -c "grep -qP '^transaktion\twieder-angebunden\t0$' '$LOG'"
warte '^seitentext' 0 10
fall "F24 Updates-Liste (GetUpdates gegen die Attrappe): 3" hat_letzte '^seitentext' $'seitentext\taktualisierungen\t3'
sleep 1; warte_layout >/dev/null

# Hilfen: Zeile über den Paketnamen finden -------------------------------------------
titel_von() { # titel_von <paketname>: Titel aus der letzten Liste
    tac "$LOG" | awk -F'\t' -v p="$1" '$1=="listenzeile" && $4==p {print $3; exit}'
}
experten_suche() { # experten_suche <text>: Expertenliste, PackageKit-Suche
    local v; v=$(zahl '^ansicht')
    if [ "$(letzte '^ansicht' | cut -f2)" != experten ]; then menue 2 3; warte '^ansicht' "$v" 3; fi
    suche "$1" 3; warte_treffer "$1" 15
}
details_von() { # details_von <paketname>: Zeile wählen, Detailseite öffnen
    local t; t=$(titel_von "$1")
    [ -n "$t" ] || { echo "   kein Listeneintrag für $1" >>"$ERG"; return 1; }
    zeile "$t" && q "key Return" "wait 600"
}
TF() { zahl '^transaktion\tfertig'; }

# P1–P4 Suche → Details → Installieren → Fortschritt → Fertig (gedit) ----------------
suche gedit 5
T=$(titel_von gedit)
fall "P1 Suche „gedit“: Katalogeintrag mit Paket gedit, verfügbar ($T)" bash -c "tac '$LOG' | awk -F'\t' '\$1==\"listenzeile\" && \$4==\"gedit\"{print \$5; exit}' | grep -qx Verfügbar"
v=$(zahl '^paketdetails')
details_von gedit
warte '^paketdetails' "$v" 10
fall "P2 Details: GetDetails gegen die Attrappe (gedit, Größe > 0)" passt_letzte '^paketdetails' '^paketdetails\tgedit\t[1-9]\d*$'
fall "P2 Detailseite: Knopf „Installieren“" passt_letzte '^details' "^details\t.*\tVerfügbar\tInstallieren$"
warte_layout >/dev/null
v=$(TF); vp=$(zahl '^zeit\tpaketstand')
klick_auf '$1=="aktion"' "Knopf Installieren"
warte '^transaktion\tgestartet' 0 3
sleep 0.15; shot installieren-fortschritt
warte '^transaktion\tfertig' "$v" 15
fall "P3 Installieren gestartet, Simulation vor der Ausführung" bash -c "grep -qP '^transaktion\tgestartet\tInstallieren\tgedit$' '$LOG' && grep -qP '^transaktion\tsimuliert' '$LOG'"
NP=$(grep -P '^transaktion\tlaeuft\t\d+\t' "$LOG" | cut -f3 | sort -un | wc -l)
fall "P3 Fortschritt: $NP verschiedene Prozentwerte bis 100 %" bash -c "[ $NP -ge 5 ] && grep -qP '^transaktion\tlaeuft\t100\t' '$LOG'"
fall "P3 Fortschritt: deutscher PackageKit-Status („Lade herunter“/„Installiere“)" grep -qP '^transaktion\tlaeuft\t\d+\t(Lade|Installiere)' "$LOG"
fall "P4 Fertig: gedit installiert, Statuszeile meldet es" bash -c "grep -qP '^transaktion\tfertig\tgedit' '$LOG' && grep -qP '^status\t.* wurde installiert\.' '$LOG'"
warte '^zeit\tpaketstand' "$vp" 10
fall "P4 Nach Fertig: Paketstand neu geladen (Attrappe meldet festen Bestand)" test "$(zahl '^zeit\tpaketstand')" -gt "$vp"
q "key Escape" "wait 400"

# P5 Installieren mit Zusatzpaket: Bestätigung (SW-4) --------------------------------
suche "querencia editor" 5
vd=$(zahl '^dialog\tinstallieren'); v=$(TF)
T=$(titel_von querencia-editor)
knopf_von "$T"
warte '^dialog\tinstallieren' "$vd" 6; sleep 0.5
fall "P5 Installieren mit Zusatzpaket: Bestätigungsdialog ($T)" test "$(zahl '^dialog\tinstallieren')" -gt "$vd"
fall "P5 Simulation nennt libquerencia-gemein" grep -qP '^simulation\tlibquerencia-gemein\t' "$LOG"
shot installieren-bestaetigen
q "key Return"
warte '^transaktion\tfertig' "$v" 15
fall "P5 Eingabetaste bestätigt: querencia-editor installiert" grep -qP '^transaktion\tfertig\tquerencia-editor' "$LOG"
sleep 0.5

# P6 Entfernen mit Simulation und Bestätigung (powertop) ---------------------------------
experten_suche powertop
vd=$(zahl '^dialog\tentfernen')
knopf_von powertop
warte '^dialog\tentfernen' "$vd" 6; sleep 0.5
fall "P6 Entfernen powertop: Bestätigungsdialog nach der Simulation" test "$(zahl '^dialog\tentfernen')" -gt "$vd"
fall "P6 Simulation (allow_deps=false) vor dem Dialog, keine mitgehenden Pakete" bash -c "grep -qP '^transaktion\tsimuliert\t$' '$LOG' && grep -qP '^transaktion\tgestartet\tEntfernen\tpowertop$' '$LOG'"
shot entfernen-bestaetigen
q "key Escape" "wait 800"
fall "P6 Escape bricht das Entfernen ab" hat_letzte '^transaktion\t(abgebrochen|fertig)' $'transaktion\tabgebrochen'
vd=$(zahl '^dialog\tentfernen'); v=$(TF)
knopf_von powertop
warte '^dialog\tentfernen' "$vd" 6; sleep 0.5
q "key Return"
warte '^transaktion\tfertig' "$v" 15
fall "P6 Entfernen bestätigt: powertop entfernt" bash -c "grep -qP '^transaktion\tfertig\tpowertop' '$LOG' && grep -qP '^status\t.*powertop wurde entfernt\.' '$LOG'"

# P7 Fehlerfall: Abhängigkeitskonflikt (kaputt) -------------------------------------------
experten_suche kaputt
vd=$(zahl '^dialog\tfehler')
knopf_von kaputt
warte '^dialog\tfehler' "$vd" 8; sleep 0.5
fall "P7 Fehlerfall: Dialog mit SW-4-Text (AbhaengigkeitenKonflikt)" bash -c "grep -qP '^transaktion\tfehler\tAbhaengigkeitenKonflikt' '$LOG' && [ $(zahl '^dialog\tfehler') -gt $vd ]"
shot fehler-abhaengigkeit
q "key Return" "wait 600"
fall "P7 Fehlerdialog per Eingabetaste (Fokus) geschlossen" grep -qP '^dialog\tzu\tfehler' "$LOG"

# P8 Abbruch während des Vorgangs (langsam, 5 s) --------------------------------------------
experten_suche langsam
knopf_von langsam
warte '^transaktion\tlaeuft\t[1-9]' "$(zahl '^transaktion\tlaeuft\t[1-9]')" 5
# Nicht modal: während des Vorgangs die Ansicht wechseln
vs=$(zahl '^ansicht'); seite "kategorie:Spiele"; warte '^ansicht' "$vs" 2
fall "F09 Fortschritt nicht modal: Kategorie wechselt während des Vorgangs" bash -c "[ \"\$(grep -P '^ansicht' '$LOG' | tail -1 | cut -f2)\" = kategorie:Spiele ] && ! grep -qP '^transaktion\tfertig\tlangsam' '$LOG'"
warte_layout >/dev/null
shot bedienbar-waehrend-vorgang
va=$(zahl '^transaktion\tabgebrochen')
klick_auf '$1=="abbrechen"' "Abbrechen"
warte '^transaktion\tabgebrochen' "$va" 6
fall "F12/P8 Abbrechen in der Fortschrittsleiste: Vorgang abgebrochen (Cancel an die Attrappe)" bash -c "[ \"\$(grep -P '^transaktion\t(fertig|abgebrochen|fehler)' '$LOG' | tail -1)\" = \$'transaktion\tabgebrochen' ]"
v=$(zahl '^ansicht'); seite experten; warte '^ansicht' "$v" 3

# P9 Sperre belegt: Dialog, „Warten“ versucht erneut, Escape bricht ab -----------------------
experten_suche gesperrt
vd=$(zahl '^dialog\tsperre')
knopf_von gesperrt
warte '^dialog\tsperre' "$vd" 8; sleep 0.5
fall "P9 Sperre belegt: Dialog „Paketverwaltung belegt“" test "$(zahl '^dialog\tsperre')" -gt "$vd"
shot sperre
vd=$(zahl '^dialog\tsperre')
q "key Return"
warte '^dialog\tsperre' "$vd" 8; sleep 0.5
fall "P9 „Warten“: erneuter Versuch, Sperre wieder belegt, Dialog erneut" test "$(zahl '^dialog\tsperre')" -gt "$vd"
q "key Escape" "wait 800"
fall "P9 Escape: Vorgang abgebrochen, kein Fehlerdialog" bash -c "[ \"\$(grep -P '^transaktion\t(fertig|abgebrochen|fehler)' '$LOG' | tail -1)\" = \$'transaktion\tabgebrochen' ]"

# P10 Neustart nötig ---------------------------------------------------------------------------
experten_suche neustart
v=$(TF)
knopf_von neustart
warte '^transaktion\tfertig' "$v" 15; sleep 0.4
fall "P10 Neustart: Fertig mit Hinweis „Ein Neustart ist nötig.“" bash -c "grep -qP '^transaktion\tneustart$' '$LOG' && grep -qP '^status\t.*neustart wurde installiert\. Ein Neustart ist nötig\.' '$LOG'"

# P11 Aktualisierungen-Ansicht und Aktualisieren ---------------------------------------------------
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 400"
v=$(zahl '^ansicht'); seite aktualisierungen; warte '^ansicht' "$v" 3; warte_layout >/dev/null
fall "P11 Ansicht Aktualisierungen: 3 Einträge" hat_letzte '^ansicht' $'ansicht\taktualisierungen\t3\t'
shot aktualisierungen
T=$(titel_von querencia-bilder)
v=$(TF)
knopf_von "$T"
warte '^transaktion\tfertig' "$v" 15
fall "P11 Aktualisieren querencia-bilder ($T): Fertig" bash -c "grep -qP '^transaktion\tgestartet\tAktualisieren\tquerencia-bilder$' '$LOG' && grep -qP '^transaktion\tfertig\tquerencia-bilder' '$LOG'"

# P12 Nicht stören: Vorgang endet im Hintergrund (Fenster inaktiv) → Meldung, Popup unterdrückt --
z0=$(bn_zustand)
experten_suche langsam
v=$(TF); vb=$(zahl '^benachrichtigung')
knopf_von langsam; sleep 0.5
# Fokus an ein fremdes Fenster (foot im isolierten labwc); ein eigener Dialog
# des Programms zählt dagegen als aktiv und unterdrückt die Meldung (P12b).
foot -T fremd sleep 30 >/dev/null 2>&1 &
FREMD=$!; sleep 1.5
warte '^transaktion\tfertig' "$v" 15; sleep 1
fall "F27b Benachrichtigung nach Fertig bei inaktivem Fenster gesendet" test "$(zahl '^benachrichtigung')" -gt "$vb"
z1=$(bn_zustand)
info "Benachrichtigungsdienst vorher $z0, nachher $z1"
fall "F27c Nicht stören: Dienst legt die Meldung von „Querencia Software“ nur in den Verlauf, kein Popup" \
    bash -c "grep -q 'Nicht stören: Meldung [0-9]* „Querencia Software“ nur in den Verlauf' '$OUT/benachrichtigungen.log' && [ \"\$(printf '%s' '$z1' | grep -oP '^\(true, uint32 \K\d+')\" = 0 ]"
kill $FREMD 2>/dev/null; sleep 0.8

# P12b Eigener Dialog offen (Info) → Programm gilt als aktiv, keine Meldung -------------
experten_suche langsam
v=$(TF); vb=$(zahl '^benachrichtigung')
knopf_von langsam; sleep 0.5
menue 3 0
warte '^transaktion\tfertig' "$v" 15; sleep 0.8
fall "P12b Eigener Dialog aktiv: keine Benachrichtigung" test "$(zahl '^benachrichtigung')" = "$vb"
q "key Escape" "wait 600"

# Protokoll von qs_transaktion -------------------------------------------------------------------
fall "Protokoll (QS_LOG) mit Einträgen der Vorgänge" bash -c "[ -s '$QS_LOG' ] && grep -q gedit '$QS_LOG'"
fall "Kein Absturz, Programm läuft" running
beenden F19
[ -n "$BNPID" ] && kill "$BNPID" 2>/dev/null
abschluss
