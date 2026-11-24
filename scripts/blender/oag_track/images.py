"""The image rows of an exported art section (docs/tools/blender.md, "Textures").

A **host** image (custom properties `oag_host_entry` and `oag_host_name`) is a
reference to a `Texture` node on a title's disc: no pixels travel, and none are
written. Every other image is the author's own and becomes a PNG beside the manifest,
whatever Blender holds it as:

- a PNG file on disk, untouched: copied byte for byte;
- a PNG packed in the .blend, untouched: its bytes written out;
- anything else (generated, painted, a JPG, TGA or EXR, or a PNG edited since it
  loaded): its pixels, encoded here, in the colour space a PNG is read in (sRGB).

Two images that would take one file name get distinct names (the first keeps it); one
image used twice is one row. An image whose file is gone is refused by name.
"""

import os
import re

import bpy

from . import pngwrite
from .artfile import ArtError
from .model import Texture

HOST_ENTRY, HOST_NAME = "oag_host_entry", "oag_host_name"
RAW_SPACES = ("Non-Color", "Raw")


def is_host(image):
    return bool(image.get(HOST_ENTRY) or image.get(HOST_NAME))


def _srgb_encode(linear):
    import numpy

    v = numpy.clip(linear, 0.0, 1.0)
    return numpy.where(v <= 0.0031308, v * 12.92, 1.055 * numpy.power(v, 1.0 / 2.4) - 0.055)


def pixels_png(image):
    """`image`'s pixels as a PNG. A byte image keeps the values it stores; a float image
    (an EXR, a generated float buffer) holds scene-linear light and is encoded to sRGB,
    unless it is data (Non-Color), which is written as it is. Alpha is straight."""
    import numpy

    width, height = image.size
    if width == 0 or height == 0:
        raise ArtError(f"image {image.name}: it holds no pixels")
    buf = numpy.empty(width * height * 4, dtype=numpy.float32)
    image.pixels.foreach_get(buf)
    px = buf.reshape(height, width, 4)[::-1].copy()  # Blender's rows run bottom up
    if image.is_float and image.colorspace_settings.name not in RAW_SPACES:
        px[..., :3] = _srgb_encode(px[..., :3])
    out = numpy.floor(numpy.clip(px, 0.0, 1.0) * 255.0 + 0.5).astype(numpy.uint8)
    return pngwrite.encode_rgba(width, height, out.tobytes())


def _file_name(stem):
    return re.sub(r"[^A-Za-z0-9._-]+", "_", stem).strip("._") or "image"


def _png_name(name):
    return name if name.lower().endswith(".png") else os.path.splitext(name)[0] + ".png"


def _payload(image):
    """(file name wanted, PNG bytes, identity of the source) for an own image."""
    path = bpy.path.abspath(image.filepath) if image.filepath else ""
    packed = image.packed_file
    if not image.is_dirty and image.source == "FILE":
        if packed is not None:
            if pngwrite.is_png(packed.data):
                return _png_name(os.path.basename(path) or _file_name(image.name)), bytes(packed.data), ("image", image.name_full)
        elif not path or not os.path.isfile(path):
            raise ArtError(
                f"image {image.name}: its file {path!r} is missing; relink it or pack it, then export again"
            )
        else:
            with open(path, "rb") as f:
                data = f.read()
            if pngwrite.is_png(data):
                return _png_name(os.path.basename(path)), data, ("file", os.path.normcase(os.path.abspath(path)))
    return _png_name(_file_name(image.name)), pixels_png(image), ("image", image.name_full)


class TextureTable:
    """The `[[art.texture]]` rows of one export, and the own images' bytes."""

    def __init__(self):
        self.rows = []
        self.files = {}
        self._by_source = {}
        self._taken = set()

    def _unique(self, name):
        stem, ext = os.path.splitext(name)
        at, n = name, 1
        while at.lower() in self._taken:
            n += 1
            at = f"{stem}-{n}{ext}"
        return at

    def index_of(self, image):
        """The row for `image`, added when new."""
        if is_host(image):
            entry, name = image.get(HOST_ENTRY), image.get(HOST_NAME)
            if not (entry and name):
                raise ArtError(f"image {image.name}: a host texture needs {HOST_ENTRY} and {HOST_NAME}")
            row = Texture(host=(str(entry), str(name)))
        else:
            wanted, data, source = _payload(image)
            name = self._by_source.get(source)
            if name is None:
                name = self._by_source[source] = self._unique(wanted)
                self._taken.add(name.lower())
                self.files[name] = data
            row = Texture(image=name)
        if row not in self.rows:
            self.rows.append(row)
        return self.rows.index(row)
