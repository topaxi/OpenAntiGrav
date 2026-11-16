#!/usr/bin/env bash
# Download the WipEout HD Fury (BCES00664) update packages into data/images/hdfury-updates/.
# The packages are served over plain http; each is checked against the SHA-1 from
# Sony's update manifest (hashes copied here, since the manifest host's
# certificate no longer verifies). The manifest hash is the SHA-1 of the file
# without its last 32 bytes (the PKG trailer, which holds the same digest).
# RPCS3 verifies the package signature on install.
set -euo pipefail

pkg_sha1() { head -c "$(( $(stat -c%s "$1") - 32 ))" "$1" | sha1sum | cut -d' ' -f1; }

dest="${1:-$(dirname "$0")/../data/images/hdfury-updates}"
base="http://b0.ww.np.dl.playstation.net/tppkg/np/BCES00664/BCES00664_T7/abe25686c1ba8006"

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
02.10 4da5a7abc0996bec3320cb08df48eeb25fadc5a4 EP9000-BCES00664_00-WHDPATCH21000000-A0210-V0100-PE.pkg
02.30 671e91208221b72ae85720661d21a5aac711709c EP9000-BCES00664_00-WHDPATCH23000000-A0230-V0101-PE.pkg
02.50 ccee085c50a1a5082fef8d49dc81e503d6d69ae5 EP9000-BCES00664_00-WHDPATCH25000000-A0250-V0101-PE.pkg
02.51 a0be72390c1945e445bb87a5ce0fbee08fb7048b EP9000-BCES00664_00-WHDPATCH25100000-A0251-V0100-PE.pkg
EOF
