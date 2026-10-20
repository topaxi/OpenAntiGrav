#!/usr/bin/env python3
"""Flatten a `psp-autopilot-contacts.py` side file for the contact-by-contact sweep.

One line per tick: tick, the box collider's centre (3), its basis rows (9), its ten
sample points (30), a contact count, and per contact its point (3), normal (3) and
depth. That is what `collision_cage_ground_truth::the_originals_contacts_against_our_mesh`
reads through `OAG_CONTACTS_FLAT`.

    python3 -I scripts/psp-contacts-flat.py <run>.contacts.jsonl > <run>.flat
"""

import json
import sys


def main():
    for line in open(sys.argv[1]):
        r = json.loads(line)
        v = [r["tick"], *r["centre"], *(x for row in r["rows"] for x in row),
             *(x for s in r["samples"] for x in s), len(r["contacts"])]
        for c in r["contacts"]:
            v += [*c["point"], *c["normal"], c["depth"]]
        print(" ".join(str(x) if isinstance(x, int) else repr(float(x)) for x in v))


if __name__ == "__main__":
    main()
