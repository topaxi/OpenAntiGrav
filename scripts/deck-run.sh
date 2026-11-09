#!/usr/bin/env bash
# Run a command on the Steam Deck under the Deck's own lock, and only while nobody
# is playing. Lives ON the Deck (installed as ~/oag-perf/deck-run.sh); agents call
# it over ssh instead of typing `flock` themselves, so the lock is always the Deck's
# file and never a workstation file that a local `~` expanded to.
#
#   ssh deck@steamdeck '~/oag-perf/deck-run.sh [--no-wait | --wait SECONDS] -- COMMAND...'
#
# Refuses (exit 75) while a Steam game is running, and (exit 76) while the file
# ~/.cache/oag/deck.reserved exists: `touch` it to keep agents off the Deck while you
# use it, `rm` it to give it back. Exit 77: the lock stayed held for the whole wait.
# Any other exit code is the command's own.
#
# The default waits up to 600 s for the lock. Run order is first come, first served
# only roughly (flock does not queue); a waiter re-checks the game and the
# reservation after it gets the lock, since either can change while it waited.
set -euo pipefail

lock="$HOME/.cache/oag/deck.lock"
reserved="$HOME/.cache/oag/deck.reserved"
wait_s=600

while [ $# -gt 0 ]; do
  case $1 in
    --no-wait) wait_s=0; shift ;;
    --wait) wait_s=$2; shift 2 ;;
    --) shift; break ;;
    *) echo "deck-run: unknown option $1 (commands go after --)" >&2; exit 2 ;;
  esac
done
[ $# -gt 0 ] || { echo "deck-run: no command after --" >&2; exit 2; }

mkdir -p "$(dirname "$lock")"

# A Steam game is a `reaper SteamLaunch AppId=N` process; gamescope itself runs in
# Game Mode all the time, so its presence says nothing. The [S] keeps pgrep from
# matching this script's own command line.
game_running() { pgrep -f '[S]teamLaunch AppId=' >/dev/null; }

refuse_if_in_use() {
  if [ -e "$reserved" ]; then
    echo "deck-run: reserved by the maintainer ($reserved exists); not starting" >&2
    exit 76
  fi
  if game_running; then
    echo "deck-run: a Steam game is running; not starting" >&2
    exit 75
  fi
}

refuse_if_in_use

# The command runs with descriptor 9 closed, so no child (gamescope, a profiler)
# inherits the lock and outlives the run holding it; this shell keeps the lock
# until the command exits.
exec 9>"$lock"
if [ "$wait_s" -eq 0 ]; then
  flock -n 9 || { echo "deck-run: lock held by another run" >&2; exit 77; }
else
  flock -w "$wait_s" 9 || { echo "deck-run: lock still held after ${wait_s}s" >&2; exit 77; }
fi

refuse_if_in_use

"$@" 9>&-
