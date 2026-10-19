#!/bin/bash
# Läuft in der isolierten labwc-Sitzung (isoliert.py): glyphenreste.py je
# Renderer starten und fotografieren. Aufruf über werkzeug/glyphenreste.sh.
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"
for r in $RENDERER; do
    rm -f "$OUT/bereit-$r"
    if [ "$r" = standard ]; then unset GSK_RENDERER; else export GSK_RENDERER=$r; fi
    python3 -I "$E/werkzeug/glyphenreste.py" "$OUT/bereit-$r" >"$OUT/glyphenreste-$r.log" 2>&1 &
    pid=$!
    for _ in $(seq 100); do [ -f "$OUT/bereit-$r" ] && break; sleep 0.1; done
    grim "$OUT/glyphenreste-$r.png"
    kill $pid; wait $pid 2>/dev/null
done
touch "$HMP04_PROFILE/logs/FERTIG"
