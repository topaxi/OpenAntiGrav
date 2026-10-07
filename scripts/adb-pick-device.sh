# Sourced by scripts/deploy-to-android.sh and scripts/push-game-data.sh, never
# run: picks which adb device the caller talks to.
#
# `pick_adb_device` leaves `adb_args` alone when the caller already set it
# (`--serial S`) or `$ANDROID_SERIAL` is set, both of which adb honours. With
# exactly one connected device it uses that one. With more than one - a phone
# and Waydroid, say - it asks: an fzf single-select when fzf is installed, a
# numbered prompt otherwise, and an error naming the serials when stdin is not a
# terminal. Expects `die` and an `adb_args` array from the caller.

pick_adb_device() {
    (( ${#adb_args[@]} )) && return 0
    [[ -n ${ANDROID_SERIAL:-} ]] && return 0

    local -a rows=() serials=()
    local line serial state rest model
    while read -r serial state rest; do
        [[ $state == device ]] || continue
        model="$(grep -o 'model:[^ ]*' <<< "$rest" | cut -d: -f2)"
        serials+=("$serial")
        rows+=("$serial  ${model:-?}")
    done < <(command adb devices -l | tail -n +2)

    case ${#serials[@]} in
        0) die "no device. 'adb devices' lists them; USB debugging must be on and the host authorised." ;;
        1) adb_args=(-s "${serials[0]}"); return 0 ;;
    esac

    if [[ ! -t 0 ]]; then
        die "more than one adb device; pass --serial with one of: ${serials[*]}"
    fi

    local choice
    if command -v fzf >/dev/null; then
        choice="$(printf '%s\n' "${rows[@]}" | fzf --header 'More than one adb device: pick one' \
            --layout reverse --height ~10 | awk '{print $1}')"
    else
        echo "More than one adb device:" >&2
        local i
        for i in "${!rows[@]}"; do echo "  $((i + 1))) ${rows[$i]}" >&2; done
        read -r -p "Which one? [1-${#rows[@]}] " i
        [[ $i =~ ^[0-9]+$ ]] && (( i >= 1 && i <= ${#rows[@]} )) && choice="${serials[$((i - 1))]}"
    fi
    [[ -n ${choice:-} ]] || die "no device picked"
    adb_args=(-s "$choice")
}
