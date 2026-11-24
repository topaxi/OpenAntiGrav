"""The image rows of an exported art section (docs/tools/blender.md, "Textures").

A **host** image (custom properties `oag_host_entry` and `oag_host_name`) is a
reference to a `Texture` node on a title's disc: no pixels travel, and none are
written. Every other image is the author's own and becomes a PNG beside the manifest,
whatever Blender holds it as:

- a PNG file on disk, untouched: copied byte for byte;
- a PNG packed in the .blend, untouched: its bytes written out;
- anything else (generated, painted, a JPG, TGA or EXR, or a PNG edited since it
  loaded): its pixels, encoded here, in the colour space a PNG is read in (sRGB).

**A host image is never packed.** With a "Host texture folder" set, an import links each host
image to the PNG `export_host_textures` wrote from the user's own disc, as an external file;
`unpack_host_images` keeps those pixels out of every saved .blend (a packed disc texture would
travel with the file into a mod or a bug report).

Two images that would take one file name get distinct names (the first keeps it); one
image used twice is one row. An image whose file is gone is refused by name.
"""

import os
import re

import bpy
from bpy.app.handlers import persistent

from . import pngwrite
from .artfile import ArtError
from .model import Texture

HOST_ENTRY, HOST_NAME = "oag_host_entry", "oag_host_name"
RAW_SPACES = ("Non-Color", "Raw")


_linked_roots = set()


def _roots():
    """The folders host textures are linked from: this session's links and the preference."""
    roots = set(_linked_roots)
    addon = bpy.context.preferences.addons.get(__package__)
    folder = "" if addon is None else addon.preferences.host_texture_folder
    if folder:
        roots.add(os.path.normcase(os.path.abspath(bpy.path.abspath(folder))))
    return roots


def _under(path, root):
    path = os.path.normcase(os.path.abspath(bpy.path.abspath(path)))
    return os.path.commonpath([path, root]) == root


def has_host_marks(image):
    """Whether the image still carries the host properties, even emptied (the panel shows
    them for exactly this)."""
    return HOST_ENTRY in image or HOST_NAME in image


def is_host(image):
    """A host image: it carries the host properties (empty or not), or it is linked from the
    host texture folder (a user who deletes the properties does not make it an own texture)."""
    if has_host_marks(image):
        return True
    if not image.filepath or image.source != "FILE":
        return False
    return any(_under(image.filepath, root) for root in _roots())


def _srgb_encode(linear):
    import numpy

    v = numpy.clip(linear, 0.0, 1.0)
    return numpy.where(v <= 0.0031308, v * 12.92, 1.055 * numpy.power(v, 1.0 / 2.4) - 0.055)


def pixels_png(image):
    """`image`'s pixels as a PNG. A byte image keeps the values it stores; a float image
    (an EXR, a generated float buffer) holds scene-linear light and is encoded to sRGB,
    unless it is data (Non-Color), which is written as it is. A premultiplied image is
    un-premultiplied first, so the PNG's alpha is straight."""
    import numpy

    width, height = image.size
    if width == 0 or height == 0:
        raise ArtError(f"image {image.name}: it holds no pixels")
    buf = numpy.empty(width * height * 4, dtype=numpy.float32)
    image.pixels.foreach_get(buf)
    px = buf.reshape(height, width, 4)[::-1].copy()  # Blender's rows run bottom up
    if image.alpha_mode == "PREMUL":  # an EXR loads premultiplied
        alpha = px[..., 3:4]
        px[..., :3] = numpy.divide(px[..., :3], alpha, out=numpy.zeros_like(px[..., :3]), where=alpha > 0.0)
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
                raise ArtError(
                    f"image {image.name}: a host texture needs {HOST_ENTRY} and {HOST_NAME} "
                    "(it is linked from the host texture folder, or marked as a host image)"
                )
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


def host_file(folder, slug, entry, name):
    """Where `export_host_textures` put a host texture:
    `<folder>/<title slug>/<circuit entry>/<texture name>.png`, lowercase, `\\` a
    directory separator (`oag_raceplay::foreign::art::host_export::texture_file`). None when
    the names would leave the folder: a `..` or `.` part, a drive prefix (`c:`), a NUL, or an
    entry or name with no part."""
    split = lambda path: [p.lower() for p in re.split(r"[\\/]", path) if p]
    if not split(entry) or not split(name):
        return None
    parts = split(entry) + split(name)
    if any(p in (".", "..") or ":" in p or "\0" in p for p in parts):
        return None
    parts[-1] = os.path.splitext(parts[-1])[0] + ".png"
    root = os.path.abspath(bpy.path.abspath(folder))
    path = os.path.join(root, slug.lower(), *parts)
    return path if os.path.commonpath([os.path.abspath(path), root]) == root else None


def link_host_images(folder, slug):
    """Links every host image to its exported PNG under `folder` as an external file, never
    packed. Returns (linked, missing): the count linked and the names with no file (or a name
    that would leave the folder), which keep whatever they showed (an 8 x 8 placeholder after
    an import)."""
    linked, missing = 0, []
    if not folder:
        return linked, missing
    _linked_roots.add(os.path.normcase(os.path.abspath(bpy.path.abspath(folder))))
    for image in bpy.data.images:
        if not has_host_marks(image):
            continue
        entry, name = image.get(HOST_ENTRY), image.get(HOST_NAME)
        if not (entry and name):
            continue
        path = host_file(folder, slug, str(entry), str(name))
        if path is None or not os.path.isfile(path):
            missing.append(str(name))
            continue
        if image.packed_file is not None:
            image.unpack(method="REMOVE")
        image.source = "FILE"
        image.filepath = path
        image.reload()
        linked += 1
    return linked, missing


def unpack_hosts():
    """Takes every host image's pixels out of memory's packed data; returns how many it
    unpacked. Run on a timer and on every dependency graph update, because Blender's
    autosave and `quit.blend` write the session without a `save_pre`: a host image
    must not stay packed between a Pack and the next tick."""
    count = 0
    for image in bpy.data.images:
        if image.packed_file is not None and is_host(image):
            try:
                image.unpack(method="REMOVE")
                count += 1
            except RuntimeError:
                pass  # library data: not part of this file
    return count


AUTOPACK_MARK = "oag_autopack"
_autopack = {}


def _own_images_to_pack():
    for image in bpy.data.images:
        if image.library is not None or image.packed_file is not None or is_host(image):
            continue
        if image.source == "FILE" and image.filepath and os.path.isfile(bpy.path.abspath(image.filepath)):
            yield image
        elif image.source == "GENERATED" and image.is_dirty:
            yield image


@persistent
def unpack_host_images(*_):
    """`save_pre`: no host image goes into a .blend packed. Blender's *Automatically Pack
    Resources* packs after this handler runs, so a save that holds a host image is made with
    it off: the switch goes off and the host images are unpacked FIRST, then the own images
    are packed one by one as autopack would (a missing file skips that image, it cannot stop
    the save's protection). The user's setting is recorded on the scene (`oag_autopack`)
    because the file stores the switch as it was at the write; `restore_autopack` brings it
    back in the session and `apply_autopack_mark` on load."""
    data = bpy.data
    restore_autopack()
    if not any(is_host(i) for i in data.images):
        return
    was = data.use_autopack
    try:
        if was:
            _autopack["was"] = True
            data.use_autopack = False
            for scene in data.scenes:
                scene[AUTOPACK_MARK] = True
    finally:
        unpack_hosts()
    if was:
        for image in list(_own_images_to_pack()):
            try:
                image.pack()
            except RuntimeError:
                pass


@persistent
def restore_autopack(*_):
    """`save_post`, and the start of the next save when a failed one left the switch off."""
    if _autopack.pop("was", False):
        bpy.data.use_autopack = True
        for scene in bpy.data.scenes:
            scene.pop(AUTOPACK_MARK, None)


@persistent
def apply_autopack_mark(*_):
    """`load_post`: a file saved with host images keeps *Automatically Pack* off on disk;
    the mark says the user had it on."""
    _autopack.clear()
    if any(s.get(AUTOPACK_MARK) for s in bpy.data.scenes):
        bpy.data.use_autopack = True
        for scene in bpy.data.scenes:
            scene.pop(AUTOPACK_MARK, None)


@persistent
def unpack_on_update(*_):
    unpack_hosts()


def _tick():
    unpack_hosts()
    return 1.0
