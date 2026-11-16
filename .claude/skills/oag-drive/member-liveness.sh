#!/usr/bin/env bash
# Usage: member-liveness.sh <member-name> [<worktree-path>] ...
#   e.g. member-liveness.sh alloc-items-2 scenery-anim
# For each member: its newest transcript, minutes since the transcript last
# grew, and how many processes run inside its worktree. A member whose
# transcript is idle past STALE_MIN with no process in its worktree is DEAD:
# its session ended without a report (it happened on 2026-11-10 and sat dead
# for 2.5 h because the lead read the worktree's uncommitted edits as progress).
#
# Transcripts live under ~/.claude/projects/<cwd-slug>/, and the slug is the
# lead's cwd at spawn time, so every project directory is searched.
set -u
STALE_MIN=${STALE_MIN:-30}
WT_ROOT=${WT_ROOT:-/home/topaxi/projects/OpenAntiGrav-worktrees}
now=$(date +%s)
for arg in "$@"; do
	name=${arg%%=*}
	wtdir=${arg#*=}
	f=$(rg -l -F "\"agentName\":\"$name\"" --glob '*.jsonl' ~/.claude/projects/ 2>/dev/null \
		| xargs -r /bin/ls -t 2>/dev/null | head -1)
	if [ -z "$f" ]; then
		echo "$name: NO-TRANSCRIPT (just spawned, or wrong name)"
		continue
	fi
	idle=$(( (now - $(stat -c %Y "$f")) / 60 ))
	# A long gate is one blocking tool call that writes nothing to the
	# transcript; a process in the worktree is what tells it from a death.
	wt="$WT_ROOT/$wtdir"
	procs=0
	for p in /proc/[0-9]*; do
		c=$(readlink "$p/cwd" 2>/dev/null) || continue
		case "$c" in "$wt"|"$wt"/*) procs=$((procs + 1)) ;; esac
	done
	state=alive
	if [ "$idle" -ge "$STALE_MIN" ] && [ "$procs" -eq 0 ]; then
		state=DEAD
	elif [ "$idle" -ge $((STALE_MIN * 3)) ]; then
		# An orphaned child of a dead session (a hung emulator, heaptrack)
		# also counts as a process; past three windows, ask the member.
		state="SUSPECT (busy ${idle} min: SendMessage it; no reply by next tick = dead)"
	elif [ "$idle" -ge "$STALE_MIN" ]; then
		state="busy (long tool call)"
	fi
	echo "$name: $state, transcript idle ${idle} min, ${procs} procs in $wt ($f)"
done
