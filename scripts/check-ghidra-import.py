#!/usr/bin/env python3
"""Assert an open PSP Ghidra program actually had its relocations applied.

Three sessions each rediscovered the same bad import the slow way - by
byte-scanning for a `lui`/`addiu` pair, by following a jump table by hand, by
reading the decompiler instead of the xref tool - before anyone measured the
cause. The cause is that `ghidra-emotionengine-reloaded`'s `EE_ElfExtension`
wins ELF adapter selection for PSP files, so `Allegrex_ElfExtension`'s
relocation pass never runs and the relocation table stays empty. See
docs/ghidra/workflow.md and
scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch.

The primary check is one number: a PSP program whose relocation table has zero
entries did not have its relocations applied, full stop. That is unambiguous,
costs one query, and does not depend on the function count - which varies from
6,927 to 10,679 across four near-identical binaries and is inflated by
hand-created functions, so it cannot discriminate.

Three secondary probes run only against `psp-pulse-usa`, because they name
addresses in that binary: a `lui` at 0x0894f6d8, the `jal` calls inside
`Billboard_ConstructResource` (0x08900220), and `g_scream_opcode_table`
(0x08ac326c). Left pinned to USA rather than migrated to EU
(ADR-0048's Ghidra target of record): re-deriving equivalent EU addresses for
these three needs a live Ghidra session, out of scope for a tooling-defaults
pass, and the primary check above (relocation-table population) already runs
against whichever program is asked for - only these three specific spot-checks
are USA-only.

Needs a running Ghidra with the GhidraMCP bridge, started with
GHIDRA_MCP_ALLOW_SCRIPTS=1 in *its own* environment - the script endpoint is
gated on the Ghidra plugin side, not on the bridge process. Without that, this
exits 2 (could not check) rather than 0 or 1.

Usage:
  scripts/check-ghidra-import.py [--url URL] [--program PATH] [--verbose]
  scripts/check-ghidra-import.py --self-test   # no Ghidra needed
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.parse
import urllib.request

DEFAULT_URL = "http://127.0.0.1:8089"

# Offsets from the canonical PSP image base, so the probe works at any base.
PULSE_USA = "/pulse/BOOT-psp-pulse-usa.BIN"

PROBE_SOURCE = r"""
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.reloc.Relocation;
import ghidra.program.model.reloc.RelocationTable;
import java.util.Iterator;

public class OagCheckImport extends GhidraScript {
  private static final long OFF_HI16 = 0x14b6d8L;
  private static final long OFF_BILLBOARD = 0xfc220L;
  private static final long OFF_SCREAM = 0x2bf26cL;
  private static final long TEXT_END = 0x272a3cL;

  @Override
  public void run() throws Exception {
    Address base = currentProgram.getImageBase();
    long b = base.getOffset();
    println("PROBE path=" + currentProgram.getDomainFile().getPathname());
    println("PROBE imageBase=" + base);
    println("PROBE language=" + currentProgram.getLanguageID());
    println("PROBE functionCount=" + currentProgram.getFunctionManager().getFunctionCount());

    RelocationTable rt = currentProgram.getRelocationTable();
    int n = 0;
    Iterator<Relocation> it = rt.getRelocations();
    while (it.hasNext()) {
      it.next();
      n++;
    }
    println("PROBE relocCount=" + n);

    if (!"PULSE_USA".equals(getScriptArgs().length > 0 ? getScriptArgs()[0] : "")) {
      return;
    }

    Memory mem = currentProgram.getMemory();
    println("PROBE hi16=0x" + String.format("%08x", mem.getInt(base.add(OFF_HI16))));

    int jalSeen = 0;
    int jalOk = 0;
    for (long o = OFF_BILLBOARD; o < OFF_BILLBOARD + 0x200; o += 4) {
      Address a = base.add(o);
      int w = mem.getInt(a);
      if ((w >>> 26) != 3) {
        continue;
      }
      long target = (a.getOffset() & 0xf0000000L) | (((long) (w & 0x03ffffff)) << 2);
      jalSeen++;
      if (target >= b && target < b + TEXT_END && getFunctionAt(toAddr(target)) != null) {
        jalOk++;
      }
    }
    println("PROBE jalSeen=" + jalSeen + " jalOk=" + jalOk);

    Address st = base.add(OFF_SCREAM);
    int inText = 0;
    for (int i = 0; i < 40; i++) {
      long v = mem.getInt(st.add(i * 4L)) & 0xffffffffL;
      if (v >= b && v < b + TEXT_END) {
        inText++;
      }
    }
    println("PROBE screamInText=" + inText);
  }
}
"""


class ProgramNotCurrent(Exception):
    """The bridge ran the probe against a program other than the one asked for."""

    def __init__(self, wanted: str, got: str) -> None:
        super().__init__(f"asked for {wanted}, the bridge ran against {got}")
        self.wanted = wanted
        self.got = got


def post(url: str, endpoint: str, **body: object) -> str:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        f"{url.rstrip('/')}/{endpoint}",
        data=data,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read().decode("utf-8", "replace")


def get(url: str, endpoint: str) -> str:
    with urllib.request.urlopen(f"{url.rstrip('/')}/{endpoint}", timeout=30) as r:
        return r.read().decode("utf-8", "replace")


def parse_probes(reply: str) -> dict[str, str]:
    """Pull `PROBE key=value` pairs out of the script's stdout."""
    found: dict[str, str] = {}
    for line in reply.splitlines():
        marker = line.find("PROBE ")
        if marker < 0:
            continue
        for token in line[marker + len("PROBE ") :].split():
            key, sep, value = token.partition("=")
            if sep:
                found[key] = value
    return found


def allegrex_programs(url: str) -> tuple[list[str], str]:
    """Every open Allegrex program, and whichever one is current.

    Only the current one is checkable: the bridge's script endpoint has no way
    to target a program that is not active, so the rest have to be made current
    in the Ghidra GUI one at a time.
    """
    reply = get(url, "list_open_programs")
    programs = json.loads(reply).get("programs", [])
    allegrex = [p for p in programs if str(p.get("language", "")).startswith("Allegrex")]
    current = next((p["path"] for p in programs if p.get("is_current")), "")
    return [p["path"] for p in allegrex], current


def verdict(probes: dict[str, str], program: str) -> tuple[bool, list[str]]:
    """Turn a parsed probe result into a pass/fail and the lines to print.

    Split out from `check` so both branches are exercisable without a running
    Ghidra - see `--self-test`. The passing branch has never been reachable on
    this machine, because no PSP database here has a non-empty relocation table
    yet, and it is the first branch the maintainer will hit after a reimport.
    """
    lines: list[str] = []

    if "relocCount" not in probes:
        lines.append(f"{program}: FAIL - the probe returned no result at all.")
        lines.append("  Is Ghidra running with GHIDRA_MCP_ALLOW_SCRIPTS=1 in its own environment?")
        lines.append("  Re-run with --verbose to see what the bridge said.")
        return False, lines

    reloc = int(probes["relocCount"])
    functions = probes.get("functionCount", "?")

    if reloc == 0:
        lines.append(f"{program}: FAIL - relocation table is empty ({functions} functions).")
        lines.append("  No PSP relocation was applied. Every `jal` target and every")
        lines.append("  `lui`/`addiu` constant still holds its pre-relocation value, so")
        lines.append("  xref queries answer 'no references' for callers that plainly exist.")
        lines.append("  Cause and fix: docs/ghidra/workflow.md, 'Why a PSP import")
        lines.append("  silently loses every relocation'.")
        # The secondary probes are the same bug seen from three more angles, so
        # printing them here would be noise, not information.
        return False, lines

    lines.append(f"{program}: relocations={reloc} functions={functions}")
    if program != PULSE_USA:
        return True, lines

    ok = True
    if probes.get("hi16", "") == "0x3c110028":
        ok = False
        lines.append("  FAIL - 0x0894f6d8 still holds the on-disk `lui` immediate (0x3c110028).")
    jal_seen = int(probes.get("jalSeen", "0"))
    jal_ok = int(probes.get("jalOk", "0"))
    if jal_seen == 0 or jal_ok != jal_seen:
        ok = False
        lines.append(f"  FAIL - {jal_seen - jal_ok}/{jal_seen} `jal` targets in")
        lines.append("    Billboard_ConstructResource do not resolve to a function.")
    scream = int(probes.get("screamInText", "0"))
    if scream != 40:
        ok = False
        lines.append(f"  FAIL - {40 - scream}/40 g_scream_opcode_table entries fall outside .text.")
    return ok, lines


def check(url: str, program: str, verbose: bool) -> bool:
    args = "PULSE_USA" if program == PULSE_USA else ""
    reply = post(url, "run_script_inline", code=PROBE_SOURCE, program=program, args=args)
    if verbose:
        print(reply)
    probes = parse_probes(reply)

    # `run_script_inline` ignores the `program` parameter and runs against
    # whatever program is current, so without this every program in the list
    # reports the current one's numbers under a different name. Checked rather
    # than assumed: the first version of this script cheerfully reported four
    # identical function counts for four different databases.
    ran_on = probes.get("path", "")
    if ran_on and ran_on != program:
        raise ProgramNotCurrent(program, ran_on)

    ok, lines = verdict(probes, program)
    for line in lines:
        print(line)
    return ok


# Real probe output, copied from a headless run against psp-pulse-usa, and the same shape as it would read
# once the relocation fix is installed. `path=` is updated from the original
# capture to the program's current name after the switch_program basename
# rename (see apply-ghidra-names.py's BINARY_PROGRAMS) - everything else here
# is the unmodified historical capture.
BAD_REPLY = """INFO  OagCheckImport.java> PROBE path=/pulse/BOOT-psp-pulse-usa.BIN (GhidraScript)
INFO  OagCheckImport.java> PROBE imageBase=08804000 (GhidraScript)
INFO  OagCheckImport.java> PROBE functionCount=10679 (GhidraScript)
INFO  OagCheckImport.java> PROBE relocCount=0 (GhidraScript)
INFO  OagCheckImport.java> PROBE hi16=0x3c110028 (GhidraScript)
INFO  OagCheckImport.java> PROBE jalSeen=4 jalOk=0 (GhidraScript)
INFO  OagCheckImport.java> PROBE screamInText=0 (GhidraScript)
"""

GOOD_REPLY = BAD_REPLY.replace("relocCount=0", "relocCount=93443") \
    .replace("hi16=0x3c110028", "hi16=0x3c1108a9") \
    .replace("jalSeen=4 jalOk=0", "jalSeen=4 jalOk=4") \
    .replace("screamInText=0", "screamInText=40")


def self_test() -> int:
    """Exercise both verdict branches on recorded probe output."""
    failures = 0

    bad = parse_probes(BAD_REPLY)
    assert bad["path"] == PULSE_USA, bad
    assert bad["relocCount"] == "0", bad
    ok, lines = verdict(bad, PULSE_USA)
    if ok or "relocation table is empty" not in "\n".join(lines):
        print("FAIL: an empty relocation table did not fail the check")
        failures += 1

    good = parse_probes(GOOD_REPLY)
    assert good["hi16"] == "0x3c1108a9", good
    assert good["jalOk"] == "4", good
    ok, lines = verdict(good, PULSE_USA)
    if not ok:
        print("FAIL: a relocated database did not pass:\n  " + "\n  ".join(lines))
        failures += 1

    # Each secondary probe must be able to fail on its own, or it is decoration.
    for key, broken in (
        ("hi16=0x3c1108a9", "hi16=0x3c110028"),
        ("jalSeen=4 jalOk=4", "jalSeen=4 jalOk=3"),
        ("screamInText=40", "screamInText=39"),
    ):
        ok, _ = verdict(parse_probes(GOOD_REPLY.replace(key, broken)), PULSE_USA)
        if ok:
            print(f"FAIL: {broken} did not fail the check")
            failures += 1

    # A non-pulse program must not be judged on pulse-only addresses.
    other = "/pure/BOOT-psp-pure-eu.BIN"
    ok, _ = verdict(parse_probes(BAD_REPLY.replace("relocCount=0", "relocCount=1")), other)
    if not ok:
        print("FAIL: a non-pulse program was judged on psp-pulse-usa's addresses")
        failures += 1

    if failures:
        print(f"{failures} self-test failure(s)")
        return 1
    print("self-test: both verdict branches and all three secondary probes behave")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--url", default=DEFAULT_URL, help="GhidraMCP bridge URL")
    ap.add_argument("--program", help="program path, e.g. /pulse/BOOT-psp-pulse-usa.BIN")
    ap.add_argument("--verbose", action="store_true", help="print the raw bridge reply")
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="check the pass/fail logic against recorded probe output, no Ghidra needed",
    )
    opts = ap.parse_args()

    if opts.self_test:
        return self_test()

    try:
        allegrex, current = allegrex_programs(opts.url)
    except (urllib.error.URLError, OSError) as e:
        print(f"could not reach the Ghidra bridge at {opts.url}: {e}", file=sys.stderr)
        print("Open Ghidra with the GhidraMCP plugin, then re-run.", file=sys.stderr)
        return 2

    if not allegrex:
        print("no Allegrex (PSP) program is open in Ghidra - nothing to check.", file=sys.stderr)
        return 2

    if opts.program:
        targets = [opts.program]
    elif current in allegrex:
        targets = [current]
    else:
        print(f"the current program ({current or 'none'}) is not a PSP one.", file=sys.stderr)
        print("Make one of these current in Ghidra and re-run:", file=sys.stderr)
        for path in allegrex:
            print(f"  {path}", file=sys.stderr)
        return 2

    failures = 0
    for program in targets:
        try:
            if not check(opts.url, program, opts.verbose):
                failures += 1
        except ProgramNotCurrent as e:
            print(f"{program}: could not check - {e}.", file=sys.stderr)
            print("  Make it the current program in Ghidra and re-run.", file=sys.stderr)
            return 2
        except (urllib.error.URLError, OSError) as e:
            print(f"{program}: could not check: {e}", file=sys.stderr)
            return 2

    unchecked = [p for p in allegrex if p not in targets]
    if failures:
        print(f"\n{failures} of {len(targets)} PSP program(s) have a bad import.")
    else:
        print(f"\n{len(targets)} PSP program(s) checked, relocations applied.")
    if unchecked:
        print("Not checked (only the current program is reachable over the bridge):")
        for path in unchecked:
            print(f"  {path}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
