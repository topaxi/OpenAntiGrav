#!/usr/bin/env bash
# Download the PSN WipEout HD (NPEA00057) update packages into data/images/hd-psn-updates/.
# The packages are served over plain http; each is checked against the SHA-1 from
# Sony's update manifest (hashes copied here, since the manifest host's
# certificate no longer verifies). The manifest hash is the SHA-1 of the file
# without its last 32 bytes (the PKG trailer, which holds the same digest).
# RPCS3 verifies the package signature on install.
set -euo pipefail

pkg_sha1() { head -c "$(( $(stat -c%s "$1") - 32 ))" "$1" | sha1sum | cut -d' ' -f1; }

dest="${1:-$(dirname "$0")/../data/images/hd-psn-updates}"
base="http://b0.ww.np.dl.playstation.net/tppkg/np/NPEA00057/NPEA00057_T19/48533f22292cfc63"

mkdir -p "$dest"
cd "$dest"

# version  sha1  file
while read -r ver sha file; do
  out="update-$ver.pkg"
  if [ -f "$out" ] && [ "$(pkg_sha1 "$out")" = "$sha" ]; then
    echo "$ver: already present, hash ok"
    continue
  fi
  curl -fL --retry 3 -o "$out.part" "$base/$file"
  got=$(pkg_sha1 "$out.part")
  if [ "$got" != "$sha" ]; then
    echo "$ver: HASH MISMATCH (got $got, want $sha); left as $out.part" >&2
    exit 1
  fi
  mv "$out.part" "$out"
  echo "$ver: ok"
done <<'EOF'
02.01 4cd73970f460b0a73df63fd23c9c48124e48a6e0 EP9000-NPEA00057_00-WHDPATCH20100000-A0201-V0100-PE.pkg
02.10 e5e2d13b08d6d2a363d77cbed40069c684b56d87 EP9000-NPEA00057_00-WHDPATCH21000000-A0210-V0100-PE.pkg
02.20 cec96a9e1ad0763938b70509a2ffa654c2c361ce EP9000-NPEA00057_00-WHDPATCH22000000-A0220-V0100-PE.pkg
02.30 9c5fb42eb03afb40e647d9124d497f7961492392 EP9000-NPEA00057_00-WHDPATCH23000000-A0230-V0101-PE.pkg
02.50 198af7c876797ddee97eb51ea05e3318f0dce89d EP9000-NPEA00057_00-WHDPATCH25000000-A0250-V0101-PE.pkg
02.51 737ab4c212b9deecd97988b0bd599e05eff4ab7c EP9000-NPEA00057_00-WHDPATCH25100000-A0251-V0100-PE.pkg
EOF
