#!/bin/bash
# Launch the diary in full-takeover mode: stop xochitl, run riddle against the
# vendor e-ink engine (instant ink), ALWAYS restore xochitl on exit.
#
# Exit the diary: power button, 5-finger tap, or SIGTERM. Escape hatch if
# anything wedges: ssh rm 'systemctl start xochitl'.

# Resolve our own install directory so the bundle works wherever it lives
# (e.g. /home/root/xovi/exthome/appload/riddle/ when installed via AppLoad).
HERE=$(cd "$(dirname "$0")" && pwd)

restore() {
    "$HERE/riddle-restore.sh"
}
# Under the Remagic Home session host (REMAGIC_SESSION=1), xochitl is already
# stopped and the session owns its restore — skip our own stop/restart.
if [ -z "${REMAGIC_SESSION:-}" ]; then
    trap restore EXIT INT TERM
fi

# Oracle config: put your API key in oracle.env next to this script, e.g.
#   RIDDLE_OPENAI_KEY=sk-...
#   RIDDLE_OPENAI_BASE=https://api.openai.com/v1     # optional
#   RIDDLE_OPENAI_MODEL=gpt-4o-mini                  # optional
# Without it, riddle falls back to the pi backend (if pi is installed).
if [ -f "$HERE/oracle.env" ]; then
    set -a; . "$HERE/oracle.env"; set +a
fi

if [ -z "${REMAGIC_SESSION:-}" ]; then
    # Kernel wake_lock is what blocks kernel autosleep. xochitl normally holds
    # one; without a replacement the kernel can suspend into a second display
    # engine while we still own the panel.
    if ! echo riddle-takeover > /sys/power/wake_lock 2>/dev/null; then
        echo "riddle: warning: kernel wake_lock unavailable; kernel autosleep is unguarded" >&2
    fi
    systemctl stop xochitl
fi
rm -f /tmp/epframebuffer.lock      # stale EPD lock blocks the engine
[ -z "${REMAGIC_SESSION:-}" ] && sleep 1

cd "$HERE"
# libquill.so ships in this bundle; libqsgepaper.so (reMarkable's proprietary
# engine) comes from the device's own scenegraph plugin dir.
export LD_LIBRARY_PATH="$HERE:/home/root/quill:/usr/lib/plugins/scenegraph"
export PAPERTERM_SHELL=
export HOME=/home/root

# systemd-inhibit --what=sleep blocks logind-initiated sleep only. It does not
# replace the kernel wake_lock above. One launch site either way.
wrap=()
if command -v systemd-inhibit >/dev/null 2>&1; then
    wrap=(systemd-inhibit --what=sleep --who=Riddle \
        --why="Riddle owns the e-paper panel during takeover" --mode=block)
fi
"${wrap[@]}" "$HERE/riddle"
echo "riddle-takeover: diary closed ($?), restoring xochitl"
