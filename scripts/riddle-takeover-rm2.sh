#!/bin/sh
# Full takeover on the reMarkable 2: stop xochitl, drive the panel through
# the rm2display server (timower/rM2-stuff — vendor libqsgepaper engine,
# standalone), run riddle with per-update waveform control, and ALWAYS hand
# the tablet back to xochitl afterwards — even if riddle dies uncleanly.
#
# Prereqs on the tablet:
#   - rm2fb_server at /opt/bin/rm2fb_server (from rm2display.ipk), on a
#     FIRMWARE VERSION IT SUPPORTS (<= 3.23 as of v0.1.4)
#   - this script + riddle + oracle.env in /home/root/xovi/exthome/appload/riddle
#
# Launch from ssh for now:  sh riddle-takeover-rm2.sh
set -u
HERE=$(cd "$(dirname "$0")" && pwd)

[ -f "$HERE/oracle.env" ] && { set -a; . "$HERE/oracle.env"; set +a; }

SERVER_PID=""
restore() {
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
    systemctl start xochitl 2>/dev/null || true
}
trap restore EXIT INT TERM

systemctl stop xochitl
sleep 1

if systemctl -q is-enabled rm2fb 2>/dev/null; then
    systemctl start rm2fb
else
    /opt/bin/rm2fb_server >/tmp/rm2fb-server.log 2>&1 &
    SERVER_PID=$!
fi
# Give the server a moment to map the panel; bail out (restoring xochitl)
# if it died — usually an unsupported-firmware address lookup failure.
sleep 2
if [ -n "$SERVER_PID" ] && ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "rm2fb_server exited — unsupported firmware? see /tmp/rm2fb-server.log" >&2
    exit 1
fi

cd "$HERE"
HOME=/home/root "$HERE/riddle"
