#!/bin/sh
# AppLoad entry point for windowed (qtfb) mode — used on the reMarkable 2,
# where the takeover backend does not exist. AppLoad sets QTFB_KEY for us;
# riddle sees it and picks the qtfb display backend.
HERE=$(cd "$(dirname "$0")" && pwd)

# Oracle config: put your API key in oracle.env next to this script.
if [ -f "$HERE/oracle.env" ]; then
    set -a; . "$HERE/oracle.env"; set +a
fi

# Path B: snapshot the screen as it was before our window covers it (the
# stock notes page you were just writing on) and hand it to riddle, which
# asks the oracle about it at startup. Set RIDDLE_ASK=off to skip.
if [ "${RIDDLE_ASK:-on}" != "off" ] && [ -f "$HERE/capture-xochitl.sh" ]; then
    if sh "$HERE/capture-xochitl.sh" /tmp/xochitl-screen.raw; then
        export RIDDLE_ASK_RAW=/tmp/xochitl-screen.raw
    else
        echo "riddle: screen capture failed; opening a blank diary" >&2
    fi
fi

cd "$HERE"
HOME=/home/root exec "$HERE/riddle"
