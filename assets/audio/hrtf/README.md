# HRTF data

`sadie2-d1-48k-200.hrir` is derived from the SADIE II database (Kearney and
Doyle, University of York), subject D1 (Neumann KU100 dummy head), the
48 kHz / 256 tap HRIR SOFA set from Zenodo record 12092466 (version 2-2),
released under the Apache License 2.0 (`LICENSE-SADIE-II.txt`, copied from the
record).

The conversion is `scripts/sofa-to-hrtf-sphere.py`: 200 of the measured
directions chosen by farthest-point sampling, the triangles are their convex
hull, positions are in this engine's listener frame (x right, y up, z ahead).
The impulse responses are unmodified. Read by `oag_audio::binaural`.
