#!/usr/bin/env python3
"""Check every target's export of demos/feature_lab against the map, feature by feature.

Run export_feature_lab.py first, then:

    python3 examples/engine-checks/check_feature_lab.py                # all targets
    python3 examples/engine-checks/check_feature_lab.py godot bevy      # some of them
    python3 examples/engine-checks/check_feature_lab.py --markdown out.md --json out.json

It reads only the exported files (scenes, level JSON, GLB/glTF, OBJ/MTL, PNG, the
generated scripts) with the Python standard library, and reports PASS, FAIL or
N/A for each feature. N/A means the target does not claim the feature; the reason
says where that is documented. A file-level PASS shows the data is there and right.
It does not show that an engine imports, draws or simulates it; see the runtime
column of docs/feature-matrix.md for what still needs an engine run.

Exit status is 1 when any check fails.
"""
import argparse
import json
import math
import re
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE_TEXTURES = ROOT / "demos/feature_lab/textures"
TARGETS = ["godot", "defold", "three", "bevy", "raylib", "unity", "unreal", "flax",
           "stride", "cocos", "gdevelop", "obj", "gltf", "glb"]
TARGET_NAMES = {"godot": "Godot", "defold": "Defold", "three": "three.js", "bevy": "Bevy",
                "raylib": "raylib", "unity": "Unity", "unreal": "Unreal", "flax": "Flax",
                "stride": "Stride", "cocos": "Cocos", "gdevelop": "GDevelop", "obj": "OBJ",
                "gltf": "glTF", "glb": "GLB"}
NAME = "feature_lab"
FEATURES = [
    ("textures", "Textures and UVs"),
    ("colours", "Face colours"),
    ("gradients", "Gradients"),
    ("surface", "Surface materials"),
    ("normal", "Brush normal maps"),
    ("solid", "Solid collision"),
    ("clip", "Clip brushes"),
    ("skip", "Skip faces"),
    ("hidden", "Export-hidden faces"),
    ("trigger", "Trigger volumes"),
    ("sky", "Sky and fog"),
    ("animation", "Keyframe animation"),
    ("door", "Door, button and trigger links"),
    ("properties", "Typed properties"),
    ("links", "Target links"),
    ("lights", "Lights"),
    ("water", "Water"),
    ("models", "Imported models, shared"),
    ("model_pbr", "Model PBR maps"),
]
PORTABLE = {"obj", "gltf", "glb"}
# Documented scope, with the place that says so. A feature listed here for a target
# is reported N/A rather than FAIL when it is missing.
def scope(targets, reason):
    return {t: reason for t in targets}


SKY = "Target::exports_sky is false; docs/sky-export.md says this target does not carry the sky"
NO_PLAYER = ("the export warns that this target has no keyframe playback controller "
             "(docs/keyframe-animation.md)")
UNREAL_COLLISION = ("docs/common-entities.md: the Unreal importer spawns meshes and lights; "
                    "doors, triggers and collision need project setup")
OBJ_ONLY = "OBJ is geometry only (Target::Obj)"
DEFOLD_SAMPLER = "Defold's .model binds only the colour sampler; the GLB keeps the map for a project material"
NOT_CLAIMED = {
    "sky": scope({"unity", "unreal", "flax", "obj", "gltf", "glb"}, SKY),
    "animation": scope({"unreal", "obj", "gltf", "glb"}, NO_PLAYER),
    "door": scope({"unreal", "obj", "gltf", "glb"}, NO_PLAYER),
    "water": scope({"unreal", "obj", "gltf", "glb"}, "not in the adapter list of docs/water-export.md"),
    "normal": {
        "obj": "the OBJ writer writes no normal maps",
        "defold": DEFOLD_SAMPLER,
        "stride": "docs/stride-export.md: normal maps are not transferred to Stride materials",
    },
    "model_pbr": {
        "obj": "docs/model-pbr.md: the standalone OBJ writer does not carry PBR maps",
        "defold": DEFOLD_SAMPLER,
        "stride": "docs/stride-export.md: normal maps and emission are not transferred to Stride materials",
    },
    "properties": scope({"obj"}, OBJ_ONLY),
    "links": scope({"obj"}, OBJ_ONLY),
    "solid": scope({"unreal"}, UNREAL_COLLISION),
    "skip": scope({"unreal"}, UNREAL_COLLISION),
}

# What the map says, in its own units and colours. See demos/feature_lab/build.py.
TEXTURE_FACES = {
    # texture: (u span, v span) over the 64 x 64 unit face, 64 x 64 pixel image.
    "wood": (1.0, 1.0),
    "metal": (1.0, 1.0),
    "crate": (math.sqrt(2), math.sqrt(2)),
    "wall": (2.0, 0.5),
    "dirt": (0.5, 0.5),
    "hazard": (1.0, 1.0),
}
OPAQUE_COLOURS = [(220, 30, 40), (30, 200, 60), (40, 60, 230), (240, 220, 30), (255, 255, 255)]
TRANSLUCENT = ((30, 200, 220), 128)
GRADIENTS = {
    "two-colour red to blue": [(255, 0, 0), (0, 0, 255)],
    "three-stop linear": [(255, 255, 0), (0, 255, 0), (0, 0, 0)],
    "radial": [(255, 255, 255), (64, 0, 128)],
    "black to white": [(0, 0, 0), (255, 255, 255)],
}
SURFACES = [(0.2, 0.9, 0.0), (0.9, 0.0, 0.0), (0.65, 0.0, 3.0), (0.05, 1.0, 0.0)]
FOG = {"density": 0.02, "colour": (180, 190, 200)}


# ---------------------------------------------------------------- small readers

def png_decode(data):
    """(width, height, rows of RGBA tuples) of an 8-bit, non-interlaced PNG."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, palette, alpha = 8, b"", None, None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif kind == b"PLTE":
            palette = [tuple(body[i:i + 3]) for i in range(0, len(body), 3)]
        elif kind == b"tRNS":
            alpha = body
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    if depth != 8 or interlace:
        raise ValueError(f"unsupported PNG depth {depth} or interlace")
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[colour]
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, previous, offset = [], bytearray(stride), 0
    for _ in range(height):
        kind = raw[offset]
        line = bytearray(raw[offset + 1:offset + 1 + stride])
        offset += 1 + stride
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            corner = previous[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + left) & 255
            elif kind == 2:
                line[i] = (line[i] + up) & 255
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 255
            elif kind == 4:
                p = left + up - corner
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
                line[i] = (line[i] + (left if pa <= pb and pa <= pc else up if pb <= pc else corner)) & 255
        previous = line
        row = []
        for x in range(width):
            px = line[x * channels:(x + 1) * channels]
            if colour == 0:
                row.append((px[0], px[0], px[0], 255))
            elif colour == 2:
                row.append((px[0], px[1], px[2], 255))
            elif colour == 3:
                r, g, b = palette[px[0]]
                row.append((r, g, b, alpha[px[0]] if alpha and px[0] < len(alpha) else 255))
            elif colour == 4:
                row.append((px[0], px[0], px[0], px[1]))
            else:
                row.append(tuple(px))
        rows.append(row)
    return width, height, rows


class Image:
    """A texture picture: a file or embedded bytes, decoded on first use."""

    def __init__(self, data=None, path=None, label=""):
        self.path = Path(path) if path else None
        self._data = data
        self.label = label or (str(path) if path else "embedded image")
        self._decoded = None

    def exists(self):
        return self._data is not None or (self.path is not None and self.path.is_file())

    def data(self):
        if self._data is None:
            self._data = self.path.read_bytes()
        return self._data

    def decoded(self):
        if self._decoded is None:
            self._decoded = png_decode(self.data())
        return self._decoded

    def size(self):
        w, h, _ = self.decoded()
        return w, h

    def pixels(self):
        return [p for row in self.decoded()[2] for p in row]


def srgb_byte(linear):
    linear = max(0.0, min(1.0, linear))
    s = linear * 12.92 if linear <= 0.0031308 else 1.055 * linear ** (1 / 2.4) - 0.055
    return round(s * 255)


class Material:
    def __init__(self, name):
        self.name = name
        self.image = None          # Image of the albedo
        self.factor = None         # (r, g, b, a) bytes, sRGB, when a flat colour
        self.roughness = None
        self.metallic = None
        self.emission = None       # strength
        self.normal = None         # Image or True when present without pixels
        self.maps = set()          # "mr", "ao", "emissive", "normal" for model PBR
        self.wrap = None           # "repeat" or "clamp" where the file says


class Primitive:
    def __init__(self, material, triangles, uvs):
        self.material = material
        self.triangles = triangles  # [(p0, p1, p2)]
        self.uvs = uvs              # [(uv0, uv1, uv2)] or None


class Collision:
    def __init__(self, kind, sensor=False, triangles=None, hulls=None, body=""):
        self.kind = kind            # "trimesh", "convex" or "box"
        self.sensor = sensor
        self.triangles = triangles or []
        self.hulls = hulls or []
        self.body = body

    def points(self):
        out = [p for t in self.triangles for p in t]
        for hull in self.hulls:
            out.extend(hull)
        return out


class Entity:
    def __init__(self, name, classname):
        self.name = name
        self.classname = classname
        self.props = {}             # key -> (type, value)
        self.targets = []           # resolved target entity names
        self.render = None          # [Primitive] or None
        self.collision = None
        self.animation = None       # parsed _heron_animation payload
        self.trigger = None         # parsed _heron_trigger payload
        self.mesh_ref = None        # the shared-mesh identity, for instancing
        self.extra = {}

    @property
    def targetname(self):
        value = self.props.get("targetname")
        return value[1] if value else None


class Level:
    def __init__(self, target, root):
        self.target = target
        self.root = root
        self.entities = []
        self.environment = None
        self.notes = []

    def by_targetname(self):
        return {e.targetname: e for e in self.entities if e.targetname}

    def by_name(self):
        return {e.name: e for e in self.entities}


def read_json(path):
    return json.loads(Path(path).read_text())


class Gltf:
    def __init__(self, document, buffers, base):
        self.json = document
        self.buffers = buffers
        self.base = base

    @staticmethod
    def load(path):
        path = Path(path)
        data = path.read_bytes()
        if data[:4] == b"glTF":
            offset, document, binary = 12, None, b""
            while offset < len(data):
                length, kind = struct.unpack_from("<II", data, offset)
                chunk = data[offset + 8:offset + 8 + length]
                if kind == 0x4E4F534A:
                    document = json.loads(chunk)
                else:
                    binary = chunk
                offset += 8 + length
            buffers = [binary]
        else:
            document = json.loads(data)
            buffers = [(path.parent / b["uri"]).read_bytes() for b in document.get("buffers", [])]
        return Gltf(document, buffers, path.parent)

    def view(self, index):
        view = self.json["bufferViews"][index]
        buffer = self.buffers[view.get("buffer", 0)]
        start = view.get("byteOffset", 0)
        return buffer[start:start + view["byteLength"]], view.get("byteStride")

    def accessor(self, index):
        accessor = self.json["accessors"][index]
        width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor["type"]]
        fmt, size = {5126: ("f", 4), 5125: ("I", 4), 5123: ("H", 2), 5121: ("B", 1)}[
            accessor["componentType"]]
        data, stride = self.view(accessor["bufferView"])
        stride = stride or size * width
        start = accessor.get("byteOffset", 0)
        out = []
        for i in range(accessor["count"]):
            values = struct.unpack_from("<" + fmt * width, data, start + i * stride)
            out.append(values if width > 1 else values[0])
        return out

    def wrap(self, texture_index):
        sampler = self.json["textures"][texture_index].get("sampler")
        if sampler is None:
            return "repeat"
        mode = self.json["samplers"][sampler].get("wrapS", 10497)
        return {10497: "repeat", 33071: "clamp", 33648: "mirror"}.get(mode, str(mode))

    def image(self, texture_index):
        texture = self.json["textures"][texture_index]
        image = self.json["images"][texture["source"]]
        if "bufferView" in image:
            return Image(data=self.view(image["bufferView"])[0], label=f"embedded image {texture['source']}")
        return Image(path=self.base / image["uri"])

    def material(self, index):
        source = self.json["materials"][index]
        m = Material(source.get("name", f"material {index}"))
        pbr = source.get("pbrMetallicRoughness", {})
        if "baseColorTexture" in pbr:
            m.image = self.image(pbr["baseColorTexture"]["index"])
            m.wrap = self.wrap(pbr["baseColorTexture"]["index"])
        elif "baseColorFactor" in pbr:
            m.factor = tuple(srgb_byte(c) for c in pbr["baseColorFactor"][:3]) + (
                round(pbr["baseColorFactor"][3] * 255),)
        m.roughness = pbr.get("roughnessFactor", 1.0)
        m.metallic = pbr.get("metallicFactor", 1.0)
        if "emissiveFactor" in source and any(source["emissiveFactor"]):
            m.emission = source.get("extensions", {}).get(
                "KHR_materials_emissive_strength", {}).get("emissiveStrength", 1.0)
        if "normalTexture" in source:
            m.normal = self.image(source["normalTexture"]["index"])
            m.maps.add("normal")
        if "metallicRoughnessTexture" in pbr:
            m.maps.add("mr")
        if "occlusionTexture" in source:
            m.maps.add("ao")
        if "emissiveTexture" in source:
            m.maps.add("emissive")
        return m

    def primitives(self, mesh_index, matrix=None):
        out = []
        for p in self.json["meshes"][mesh_index]["primitives"]:
            positions = self.accessor(p["attributes"]["POSITION"])
            if matrix:
                positions = [transform(matrix, v) for v in positions]
            uvs = self.accessor(p["attributes"]["TEXCOORD_0"]) if "TEXCOORD_0" in p["attributes"] else None
            indices = self.accessor(p["indices"]) if "indices" in p else list(range(len(positions)))
            triangles = [tuple(positions[i] for i in indices[k:k + 3]) for k in range(0, len(indices) - 2, 3)]
            tri_uvs = [tuple(uvs[i] for i in indices[k:k + 3]) for k in range(0, len(indices) - 2, 3)] if uvs else None
            material = self.material(p["material"]) if "material" in p else Material("none")
            out.append(Primitive(material, triangles, tri_uvs))
            out[-1].tangents = "TANGENT" in p["attributes"]
        return out


def matrix_of(node):
    """Column-major 4x4 of a glTF node."""
    if "matrix" in node:
        return node["matrix"]
    t = node.get("translation", [0, 0, 0])
    x, y, z, w = node.get("rotation", [0, 0, 0, 1])
    s = node.get("scale", [1, 1, 1])
    r = [1 - 2 * (y * y + z * z), 2 * (x * y + z * w), 2 * (x * z - y * w),
         2 * (x * y - z * w), 1 - 2 * (x * x + z * z), 2 * (y * z + x * w),
         2 * (x * z + y * w), 2 * (y * z - x * w), 1 - 2 * (x * x + y * y)]
    return [r[0] * s[0], r[1] * s[0], r[2] * s[0], 0, r[3] * s[1], r[4] * s[1], r[5] * s[1], 0,
            r[6] * s[2], r[7] * s[2], r[8] * s[2], 0, t[0], t[1], t[2], 1]


def multiply(a, b):
    return [sum(a[k * 4 + r] * b[c * 4 + k] for k in range(4)) for c in range(4) for r in range(4)]


def transform(m, v):
    return tuple(m[r] * v[0] + m[4 + r] * v[1] + m[8 + r] * v[2] + m[12 + r] for r in range(3))


def parse_obj(path):
    """{group name: (triangles, uv triangles)} in file order, and the mtllib names."""
    vs, vts, groups, mtllibs, current = [], [], {}, [], None
    for line in Path(path).read_text().splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "v":
            vs.append(tuple(float(x) for x in parts[1:4]))
        elif parts[0] == "vt":
            vts.append(tuple(float(x) for x in parts[1:3]))
        elif parts[0] == "mtllib":
            mtllibs.append(line.split(None, 1)[1])
        elif parts[0] in ("usemtl",):
            current = parts[1]
            groups.setdefault(current, ([], []))
        elif parts[0] == "f":
            corners = []
            for c in parts[1:]:
                bits = c.split("/")
                v = vs[int(bits[0]) - 1 if int(bits[0]) > 0 else int(bits[0])]
                t = vts[int(bits[1]) - 1] if len(bits) > 1 and bits[1] else None
                corners.append((v, t))
            for k in range(1, len(corners) - 1):
                tri = (corners[0], corners[k], corners[k + 1])
                groups.setdefault(current, ([], []))
                groups[current][0].append(tuple(c[0] for c in tri))
                groups[current][1].append(tuple(c[1] for c in tri))
    return groups, mtllibs


def parse_obj_objects(path):
    """[(object name, {material: (triangles, uvs)})] for an OBJ with `o` blocks."""
    vs, vts, objects = [], [], []
    material = None
    for line in Path(path).read_text().splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "v":
            vs.append(tuple(float(x) for x in parts[1:4]))
        elif parts[0] == "vt":
            vts.append(tuple(float(x) for x in parts[1:3]))
        elif parts[0] == "o":
            objects.append((parts[1], {}))
        elif parts[0] == "usemtl":
            material = parts[1]
        elif parts[0] == "f":
            corners = []
            for c in parts[1:]:
                bits = c.split("/")
                corners.append((vs[int(bits[0]) - 1], vts[int(bits[1]) - 1] if len(bits) > 1 and bits[1] else None))
            groups = objects[-1][1]
            for k in range(1, len(corners) - 1):
                tri = (corners[0], corners[k], corners[k + 1])
                entry = groups.setdefault(material, ([], []))
                entry[0].append(tuple(c[0] for c in tri))
                entry[1].append(tuple(c[1] for c in tri))
    return objects


def parse_mtl(path):
    materials, current = {}, None
    for line in Path(path).read_text().splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "newmtl":
            current = Material(parts[1])
            materials[parts[1]] = current
        elif current is None:
            continue
        elif parts[0] == "map_Kd":
            current.image = Image(path=Path(path).parent / line.split(None, 1)[1])
        elif parts[0] == "Kd":
            current.kd = tuple(float(x) for x in parts[1:4])
        elif parts[0] == "d":
            current.dissolve = float(parts[1])
        elif parts[0] == "Pr":
            current.roughness = float(parts[1])
        elif parts[0] == "Pm":
            current.metallic = float(parts[1])
        elif parts[0] == "Ke":
            current.emission = max(float(x) for x in parts[1:4])
        elif parts[0] in ("map_Bump", "bump", "norm", "map_bump"):
            current.normal = Image(path=Path(path).parent / parts[-1])
    for m in materials.values():
        if m.image is None and hasattr(m, "kd"):
            m.factor = tuple(round(c * 255) for c in m.kd) + (255,)
    return materials


def typed(value):
    """(type, value) of a neutral-JSON property."""
    kind, raw = value["type"], value["value"]
    if kind == "array":
        raw = [typed(v)[1] for v in raw]
    return kind, raw


# ---------------------------------------------------------------- adapters

def json_family(target, root):
    """The neutral level JSON most targets write, with GLB (or OBJ) meshes beside it."""
    path = root / (f"{NAME}.level.json" if target == "flax" else f"{NAME}.json")
    level_json = read_json(path)
    level = Level(target, root)
    level.environment = level_json.get("environment")
    textures = {t["name"]: t for t in level_json.get("textures", [])}
    gameplay = {}
    if target == "unity":
        gameplay = {e["name"]: e for e in read_json(root / f"{NAME}.gameplay.json")["entities"]}
        mtl = parse_mtl(root / NAME / f"{NAME}_paint.mtl")
    raylib_collision = raylib_level(root) if target == "raylib" else {}
    for source in level_json["entities"]:
        e = Entity(source["name"], source["classname"])
        e.props = {k: typed(v) for k, v in source.get("properties", {}).items()}
        e.targets = list(source.get("targets", []))
        e.mesh_ref = source.get("mesh")
        e.extra["tags"] = source.get("tags", [])
        e.extra["transform"] = source.get("transform")
        if "_heron_animation" in e.props:
            e.animation = json.loads(e.props["_heron_animation"][1])
        if "_heron_trigger" in e.props:
            e.trigger = json.loads(e.props["_heron_trigger"][1])
        mesh = source.get("mesh")
        gltf = None
        if mesh and mesh.endswith(".glb") and (root / mesh).is_file():
            gltf = Gltf.load(root / mesh)
        if source.get("surfaces"):
            if gltf:
                e.render = gltf.primitives(0)
            elif mesh and mesh.endswith(".obj"):
                groups, _ = parse_obj(root / mesh)
                e.render = []
                for name, (tris, uvs) in groups.items():
                    material = mtl.get(name, Material(name))
                    info = textures.get(name, {})
                    if info.get("surface"):
                        material.roughness, material.metallic, material.emission = info["surface"]
                    e.render.append(Primitive(material, tris, uvs))
            # The level JSON names each primitive's texture, image and surface.
            for surface in source["surfaces"]:
                if not e.render or surface["primitive"] >= len(e.render):
                    continue
                prim = e.render[surface["primitive"]]
                info = textures.get(surface["texture"], {})
                prim.material.json_name = surface["texture"]
                prim.material.json_surface = info.get("surface")
                if prim.material.image is None and info.get("image"):
                    prim.material.image = Image(path=root / info["image"])
        collision = source.get("collision")
        if collision:
            kind = collision.get("kind")
            col = Collision(kind, bool(collision.get("sensor")))
            if "hulls" in collision:
                col.hulls = [list(zip(*[iter(h)] * 3)) for h in collision["hulls"]]
            if "triangles" in collision:
                flat = list(zip(*[iter(collision["triangles"])] * 3))
                col.triangles = [tuple(flat[i:i + 3]) for i in range(0, len(flat), 3)]
            if not col.points() and gltf and collision.get("mesh", 0) < len(gltf.json["meshes"]):
                for prim in gltf.primitives(collision["mesh"]):
                    if kind == "convex":
                        col.hulls.append([p for t in prim.triangles for p in t])
                    else:
                        col.triangles.extend(prim.triangles)
            e.collision = col
        if target == "unity":
            e.collision = unity_collision(gameplay.get(e.name))
            e.extra["light"] = (gameplay.get(e.name) or {}).get("light")
        if target == "defold":
            e.collision = defold_collision(root / NAME / f"{e.name}.collisionobject")
            defold_materials(root, e)
        if target == "stride":
            stride_materials(root, e)
        if target == "raylib":
            e.collision = raylib_collision.get(e.name)
        level.entities.append(e)
    if target == "unreal":
        # What the generated import script spawns, entity by entity.
        script = (root / f"import_{NAME}.py").read_text()
        spawned = dict(re.findall(r'^    \("(\w+)", "\w+", (None|"[^"]+"),', script, re.M))
        for e in level.entities:
            e.extra["unreal_mesh"] = spawned.get(e.name, "missing")
    if target == "raylib":
        # raylib's level JSON leaves animation to the C header; check what C reads.
        animations, triggers = raylib_header(root)
        for e in level.entities:
            e.animation = animations.get(e.name, e.animation)
            e.trigger = triggers.get(e.name, e.trigger)
    names = level.by_name()
    for e in level.entities:
        e.targets = [names[t].targetname if t in names else t for t in e.targets]
        if e.trigger:
            e.trigger["targets"] = [names[t].targetname if t in names else t for t in e.trigger["targets"]]
    return level


def unity_collision(entry):
    if not entry or not entry.get("shapes"):
        return None
    col = Collision("trimesh", bool(entry.get("sensor")), body=entry.get("bodyType") or "")
    for shape in entry["shapes"]:
        points = list(zip(*[iter(shape["points"])] * 3))
        if shape.get("convex"):
            col.kind = "convex"
            col.hulls.append(points)
        else:
            col.triangles.extend(tuple(points[i:i + 3]) for i in range(0, len(points), 3))
    return col


def defold_materials(root, e):
    """Defold draws what the .model binds, not what the GLB embeds: use its textures."""
    path = root / NAME / f"{e.name}.model"
    if not e.render or not path.is_file():
        return
    bound = dict(re.findall(r'name: "([^"]+)"\n\s*material: "[^"]+"\n\s*textures \{\n\s*sampler: "\w+"\n'
                            r'\s*texture: "([^"]+)"', path.read_text()))
    for prim in e.render:
        texture = bound.get(prim.material.name)
        prim.material.glb_image = prim.material.image
        prim.material.image = Image(path=root / texture.lstrip("/")) if texture else None
        prim.material.factor = None
        prim.material.normal = None
        prim.material.maps = set()
        prim.material.wrap = None


def stride_materials(root, e):
    """Stride draws its .sdmat materials: their texture, glossiness and metalness."""
    model = root / NAME / f"{e.name}.sdm3d"
    if not e.render or not model.is_file():
        return
    urls = dict(re.findall(r'Name: "([^"]+)"\n\s*MaterialInstance:\n\s*Material: [\w-]+:Levels/\w+/Materials/(\w+)',
                           model.read_text()))
    for prim in e.render:
        m = prim.material
        sdmat = root / NAME / "Materials" / f"{urls.get(m.name, m.name)}.sdmat"
        if not sdmat.is_file():
            prim.material = Material(m.name)
            continue
        text = sdmat.read_text()
        texture = re.search(r"Texture: [\w-]+:Levels/\w+/Textures/(\w+)", text)
        m.image = Image(path=root / NAME / "Textures" / f"{texture.group(1)}.png") if texture else None
        colour = re.search(r"DiffuseMap: !ComputeColor\n\s*Value: \{R: ([\d.]+), G: ([\d.]+), B: ([\d.]+)", text)
        m.factor = tuple(srgb_byte(float(c)) for c in colour.groups()) + (255,) if colour and not texture else None
        gloss = re.search(r"GlossinessMap: !ComputeFloat\n\s*Value: ([\d.e-]+)", text)
        metal = re.search(r"MetalnessMap: !ComputeFloat\n\s*Value: ([\d.e-]+)", text)
        m.roughness = 1 - float(gloss.group(1)) if gloss else None
        m.metallic = float(metal.group(1)) if metal else None
        m.emission = None
        m.normal = None
        m.maps = set()
        wrap = re.search(r"AddressModeU: (\w+)", text)
        m.wrap = {"Wrap": "repeat", "Clamp": "clamp"}.get(wrap.group(1)) if wrap else None
        prim.material.json_surface = None


def defold_collision(path):
    if not path.is_file():
        return None
    text = path.read_text()
    kind = re.search(r"^type: (\w+)", text, re.M).group(1)
    data = [float(x) for x in re.findall(r"^\s*data: (-?[\d.e+-]+)", text, re.M)]
    col = Collision("convex", kind == "COLLISION_OBJECT_TYPE_TRIGGER", body=kind)
    for index, count in re.findall(r"index: (\d+)\s+count: (\d+)", text):
        start, n = int(index), int(count)
        col.hulls.append(list(zip(*[iter(data[start:start + n])] * 3)))
    if "TYPE_TRIANGLE_MESH" in text:
        col.kind = "trimesh"
    return col


def raylib_level(root):
    """Collision as raylib's C reader sees it, from the flat .level text."""
    out, current, hull = {}, None, None
    for line in (root / f"{NAME}.level").read_text().splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "entity":
            current = parts[2]
        elif parts[0] == "collision":
            out[current] = Collision(parts[1], "sensor" in parts)
        elif parts[0] == "ctri":
            v = [float(x) for x in parts[1:10]]
            out[current].triangles.append((tuple(v[0:3]), tuple(v[3:6]), tuple(v[6:9])))
        elif parts[0] == "chull":
            hull = []
        elif parts[0] == "chv":
            hull.append(tuple(float(x) for x in parts[1:4]))
        elif parts[0] == "chend":
            out[current].hulls.append(hull)
    return out


def raylib_header(root):
    """Animation and trigger payloads, rebuilt from the generated C header's tables."""
    text = (root / f"{NAME}_animation.h").read_text()
    number = r"(-?[\d.]+e[+-]?\d+)"
    keys = {}
    for name, body in re.findall(r"static const HbKey (hb_keys_\w+)\[\]=\{\n(.*?)\n\};", text, re.S):
        rows = re.findall(r"\{" + ",".join([number] * 4) + r",(\d)\}", body)
        keys[name] = [{"time": float(t), "value": float(v), "incoming": float(i), "outgoing": float(o),
                       "interpolation": ["linear", "hold", "bezier"][int(k)]} for t, v, i, o, k in rows]
    clips = []
    table = re.search(r"static const HbClip hb_clips\[\]=\{\n(.*?)\n\};", text, re.S).group(1)
    for name, duration, mode, channels in re.findall(r'\{"(\w*)",' + number + r",(\d),\{(.*?\})\}\}", table):
        channel_list = re.findall(r"\{(\w+),(\d+)\}", channels)
        clips.append({"name": name, "duration": float(duration), "mode": ["once", "loop", "pingpong"][int(mode)],
                      "channels": [{"keys": keys.get(k, []) if k != "0" else []} for k, _ in channel_list]})
    animations = {}
    table = re.search(r"static const HbAnimation hb_animations\[\]=\{\n(.*?)\n\};", text, re.S).group(1)
    for name, _, first, count, selected, autoplay in re.findall(r'\{"(\w+)",(\d+),(\d+),(\d+),(\d+),(\d+),', table):
        own = clips[int(first):int(first) + int(count)]
        animations[name] = {"autoplay": autoplay == "1", "selected": clips[int(selected)]["name"], "clips": own}
    names = [a for a in animations]
    triggers = {}
    table = re.search(r"static const HbTrigger hb_triggers\[\]=\{(.*?)\};", text, re.S).group(1)
    for name, volumes, _, targets, _, once, wait, action, clip in re.findall(
            r'\{"(\w+)",(hb_volumes_\d+),(\d+),(hb_targets_\d+),(\d+),(\d),' + number + r',"(\w*)","(\w*)"\}', table):
        indices = re.search(rf"static const int {targets}\[\]=\{{(.*?)\}};", text).group(1)
        triggers[name] = {"targets": [names[int(i)] for i in indices.split(",")], "action": action,
                          "clip": clip, "once": once == "1", "wait": float(wait),
                          "volumes": [volumes] if volumes in text else []}
    return animations, triggers


def godot_values(text):
    """Python values for a Godot metadata dictionary's lines."""
    out = {}
    for line in text.splitlines():
        m = re.match(r'^"([^"]+)": (.*?),?$', line)
        if not m:
            continue
        key, raw = m.groups()
        if raw.startswith('"'):
            out[key] = ("string", json.loads(raw))
        elif raw in ("true", "false"):
            out[key] = ("bool", raw == "true")
        elif raw.startswith("Color("):
            out[key] = ("color", [float(x) for x in raw[6:-1].split(",")])
        elif raw.startswith("["):
            out[key] = ("array", json.loads(raw))
        elif re.fullmatch(r"-?\d+", raw):
            out[key] = ("int", int(raw))
        else:
            try:
                out[key] = ("float", float(raw))
            except ValueError:
                out[key] = ("raw", raw)
    return out


def parse_tscn(path):
    text = Path(path).read_text()
    sections = []
    for block in re.split(r"\n(?=\[)", text):
        header = re.match(r"\[(\w+)([^\]]*)\]", block)
        if not header:
            continue
        attrs = dict(re.findall(r'(\w+)=("(?:[^"\\]|\\.)*"|\S+)', header.group(2)))
        attrs = {k: v.strip('"') for k, v in attrs.items()}
        body = block[header.end():]
        sections.append((header.group(1), attrs, body))
    return sections


def body_value(body, key):
    m = re.search(rf"^{re.escape(key)} = (.*)$", body, re.M)
    return m.group(1) if m else None


def godot(root):
    level = Level("godot", root)
    sections = parse_tscn(root / f"{NAME}.tscn")
    ext = {a["id"]: a["path"] for kind, a, _ in sections if kind == "ext_resource"}
    sub = {a["id"]: (a["type"], body) for kind, a, body in sections if kind == "sub_resource"}
    nodes = [(a, body) for kind, a, body in sections if kind == "node"]
    level.sections = sections

    def resource_path(ref):
        m = re.match(r'ExtResource\("([^"]+)"\)', ref or "")
        if not m:
            return None
        path = ext[m.group(1)]
        return root / path[6:] if path.startswith("res://") else root / path

    def material(ref):
        m = re.match(r'SubResource\("([^"]+)"\)', ref or "")
        kind, body = sub[m.group(1)]
        mat = Material(m.group(1))
        mat.kind = kind
        albedo = body_value(body, "albedo_texture") or body_value(body, "shader_parameter/albedo_texture")
        if albedo:
            mat.image = Image(path=resource_path(albedo))
            mat.wrap = "clamp" if body_value(body, "texture_repeat") == "false" else "repeat"
        color = body_value(body, "albedo_color")
        if color and not albedo:
            mat.factor = tuple(srgb_byte(float(x)) for x in color[6:-1].split(",")[:3]) + (255,)
        for key, attr in (("roughness", "roughness"), ("metallic", "metallic")):
            value = body_value(body, key)
            if value is not None:
                setattr(mat, attr, float(value))
        if body_value(body, "emission_enabled") == "true":
            mat.emission = float(body_value(body, "emission_energy_multiplier") or 1.0)
        if body_value(body, "normal_texture"):
            mat.normal = Image(path=resource_path(body_value(body, "normal_texture")))
            mat.maps.add("normal")
        if body_value(body, "roughness_texture"):
            mat.maps.add("mr")
        if body_value(body, "ao_texture"):
            mat.maps.add("ao")
        if body_value(body, "emission_texture"):
            mat.maps.add("emissive")
        return mat

    children = {}
    for attrs, body in nodes:
        children.setdefault(attrs.get("parent"), []).append((attrs, body))
    for attrs, body in children.get(".", []):
        e = Entity(attrs["name"], attrs["type"])
        e.extra["type"] = attrs["type"]
        e.extra["body"] = body
        e.extra["groups"] = attrs.get("groups", "")
        meta = re.search(r"metadata/properties = \{\n(.*?)\n\}", body, re.S)
        if meta:
            e.props = godot_values(meta.group(1))
            e.classname = e.props.get("classname", ("string", e.classname))[1]
        paths = re.search(r"metadata/target_paths = \[(.*?)\]$", body, re.M)
        if paths:
            e.targets = re.findall(r'NodePath\("\.\./([^"]+)"\)', paths.group(1))
        for child, child_body in children.get(attrs["name"], []):
            if child["type"] == "MeshInstance3D":
                mesh_ref = body_value(child_body, "mesh")
                e.mesh_ref = mesh_ref
                obj = resource_path(mesh_ref)
                objects = parse_obj_objects(obj)
                overrides = dict(re.findall(r"^surface_material_override/(\d+) = (.*)$", child_body, re.M))
                e.render = []
                for i, (_, groups) in enumerate(objects):
                    for name, (tris, uvs) in groups.items():
                        mat = material(overrides[str(i)]) if str(i) in overrides else Material(name)
                        e.render.append(Primitive(mat, tris, uvs))
            elif child["type"] == "CollisionShape3D":
                ref = re.match(r'SubResource\("([^"]+)"\)', body_value(child_body, "shape")).group(1)
                kind, shape = sub[ref]
                if e.collision is None:
                    sensor = attrs["type"] == "Area3D"
                    e.collision = Collision("trimesh" if kind == "ConcavePolygonShape3D" else "convex",
                                            sensor, body=attrs["type"])
                numbers = [float(x) for x in re.findall(r"-?[\d.]+(?:e-?\d+)?",
                                                        re.search(r"PackedVector3Array\((.*?)\)", shape).group(1))]
                points = list(zip(*[iter(numbers)] * 3))
                if kind == "ConcavePolygonShape3D":
                    e.collision.triangles.extend(tuple(points[i:i + 3]) for i in range(0, len(points), 3))
                else:
                    e.collision.hulls.append(points)
            elif child["name"] == "HeronAnimation":
                e.extra["animation_node"] = child_body
            elif child["name"] == "HeronAnimationTrigger":
                e.extra["trigger_node"] = child_body
            elif child["name"] == "heron_water":
                e.extra["water_node"] = child_body
        level.entities.append(e)
    return level


def gdevelop(root):
    level = Level("gdevelop", root)
    project = read_json(root / "game.json")
    layout = next(l for l in project["layouts"] if l["name"] == NAME)
    level.layout = layout
    objects = {o["name"]: o for o in layout["objects"]}
    by_name = {}
    for instance in layout["instances"]:
        variables = {v["name"]: v for v in instance.get("initialVariables", [])}
        name = variables["heron_name"]["value"]
        e = by_name.get(name)
        if e is None:
            e = Entity(name, variables["heron_classname"]["value"])
            for key, v in variables.items():
                if key.startswith("heron_"):
                    continue
                e.props[key] = gdevelop_value(v)
            e.targets = [c["value"] for c in variables.get("heron_targets", {}).get("children", [])]
            by_name[name] = e
            level.entities.append(e)
        obj = objects[instance["name"]]
        resource = obj.get("content", {}).get("modelResourceName")
        behaviours = obj.get("behaviors", [])
        physics = [b for b in behaviours if b.get("type") == "Physics3D::Physics3DBehavior"]
        if resource and instance["name"].endswith("_collision"):
            gltf = Gltf.load(root / resource)
            col = Collision("trimesh", False, body=physics[0]["bodyType"] if physics else "")
            for prim in gltf.primitives(0):
                col.triangles.extend(prim.triangles)
            e.collision = col if physics else None
        elif resource:
            e.mesh_ref = resource
            e.render = Gltf.load(root / resource).primitives(0)
            if physics:
                # A box body in GDevelop pixels, X, Y (the level's Z) and Z (its Y).
                body = physics[0]
                half = [body["shapeDimensionA"] / 200, body["shapeDimensionC"] / 200,
                        body["shapeDimensionB"] / 200]
                corners = [(x * half[0], y * half[1], z * half[2])
                           for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
                e.collision = Collision("box", False, hulls=[corners], body=body["bodyType"])
    # The generated event carries the animation and trigger payloads it runs.
    code = "\n".join(line for event in layout.get("events", []) for line in event.get("inlineCode", []))
    data = re.search(r"const DATA = (\{.*?\});\n", code, re.S)
    level.script = code
    names = level.by_name()
    for item in json.loads(data.group(1))["entities"] if data else []:
        e = names.get(item["name"])
        if e is None:
            continue
        if item.get("animation"):
            e.animation = json.loads(item["animation"])
        if item.get("trigger"):
            e.trigger = json.loads(item["trigger"])
            e.trigger["targets"] = [names[t].targetname if t in names else t
                                    for t in e.trigger["targets"]]
        e.extra["objects"] = item.get("objects", [])
    for e in level.entities:
        e.targets = [names[t].targetname if t in names else t for t in e.targets]
    return level


def gdevelop_value(v):
    kind = v.get("type")
    if kind == "number":
        return ("float" if isinstance(v["value"], float) else "int", v["value"])
    if kind == "boolean":
        return ("bool", v["value"])
    if kind == "array":
        return ("array", [gdevelop_value(c)[1] for c in v.get("children", [])])
    if kind == "structure":
        return ("structure", {c["name"]: gdevelop_value(c)[1] for c in v.get("children", [])})
    return ("string", v.get("value"))


def portable_gltf(target, root):
    gltf = Gltf.load(root / (f"{NAME}.gltf" if target == "gltf" else f"{NAME}.glb"))
    level = Level(target, root)
    level.gltf = gltf
    nodes = gltf.json["nodes"]
    parents = {}
    for i, node in enumerate(nodes):
        for c in node.get("children", []):
            parents[c] = i

    def world(i):
        m = matrix_of(nodes[i])
        while i in parents:
            i = parents[i]
            m = multiply(matrix_of(nodes[i]), m)
        return m

    for i, node in enumerate(nodes):
        extras = node.get("extras", {})
        if "heron" not in extras:
            continue
        heron = extras["heron"]
        e = Entity(node.get("name", f"node {i}"), heron["classname"])
        e.props = {k: typed(v) for k, v in heron.get("properties", {}).items()}
        e.extra["heron"] = heron
        e.extra["entity"] = extras.get("entity", {})
        meshes = [(c, nodes[c]["mesh"]) for c in node.get("children", []) if "mesh" in nodes[c]]
        if "mesh" in node:
            meshes.append((i, node["mesh"]))
        if meshes:
            e.render = []
            for index, mesh in meshes:
                e.render.extend(gltf.primitives(mesh, world(index)))
            e.mesh_ref = tuple(sorted(m for _, m in meshes))
        level.entities.append(e)
    by_targetname = level.by_targetname()
    for e in level.entities:
        target = e.props.get("target")
        if target and target[1] in by_targetname:
            e.targets = [target[1]]
        if "_heron_animation" in e.props:
            e.animation = json.loads(e.props["_heron_animation"][1])
    return level


def portable_obj(root):
    """The single OBJ: objects are `brush_N` in map order and `imported_N` per model."""
    level = Level("obj", root)
    materials = parse_mtl(root / f"{NAME}.mtl")
    objects = parse_obj_objects(root / f"{NAME}.obj")
    # Brush numbers count every brush of the map in order; the map says which
    # entity owns which.
    owners = map_brush_owners(ROOT / "demos/feature_lab/feature_lab.map")
    models = [name for name, cls in map_entities(ROOT / "demos/feature_lab/feature_lab.map")
              if cls == "heron_model"]
    entities = {}
    # Every map entity, so one the OBJ leaves out (a clip brush) is found as undrawn.
    for owner, classname in map_entities(ROOT / "demos/feature_lab/feature_lab.map"):
        e = Entity(owner, classname)
        e.props["targetname"] = ("string", owner)
        entities[owner] = e
        level.entities.append(e)
    imported = 0
    for name, groups in objects:
        if name.startswith("brush_"):
            owner = owners[int(name[6:]) - 1]
        else:
            owner = models[imported]
            imported += 1
        e = entities[owner]
        e.render = e.render or []
        for material, (tris, uvs) in groups.items():
            e.render.append(Primitive(materials.get(material, Material(material)), tris, uvs))
        e.mesh_ref = None
    return level


def map_entities(path):
    """[(targetname or classname, classname)] of a .map's entities, in order."""
    out = []
    for block in re.findall(r"^\{\n(.*?)^\}", Path(path).read_text(), re.S | re.M):
        props = dict(re.findall(r'^"([^"]+)" "([^"]*)"', block, re.M))
        out.append((props.get("targetname", props.get("classname")), props.get("classname")))
    return out


def map_brush_owners(path):
    owners = []
    text = Path(path).read_text()
    for entity in re.split(r"^// entity \d+\n", text, flags=re.M)[1:]:
        props = dict(re.findall(r'^"([^"]+)" "([^"]*)"', entity, re.M))
        owner = props.get("targetname", props.get("classname"))
        owners.extend([owner] * len(re.findall(r"^// brush \d+", entity, re.M)))
    return owners


def load(target, root):
    if target == "godot":
        return godot(root)
    if target == "gdevelop":
        return gdevelop(root)
    if target in ("gltf", "glb"):
        return portable_gltf(target, root)
    if target == "obj":
        return portable_obj(root)
    return json_family(target, root)


# ---------------------------------------------------------------- geometry helpers

def sub3(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def length(a):
    return math.sqrt(dot(a, a))


def normal(tri):
    n = cross(sub3(tri[1], tri[0]), sub3(tri[2], tri[0]))
    size = length(n)
    return (n[0] / size, n[1] / size, n[2] / size) if size > 1e-12 else (0, 0, 0)


def box_sides(triangles, bounds=None, tolerance=1e-4):
    """Which of the six sides of the bounding box the triangles cover."""
    points = [p for t in triangles for p in t]
    if not points:
        return set()
    lo = bounds[0] if bounds else [min(p[i] for p in points) for i in range(3)]
    hi = bounds[1] if bounds else [max(p[i] for p in points) for i in range(3)]
    sides = set()
    for t in triangles:
        for axis in range(3):
            for name, value in (("-", lo[axis]), ("+", hi[axis])):
                if all(abs(p[axis] - value) < tolerance for p in t):
                    sides.add(f"{name}{'xyz'[axis]}")
    return sides


def bounds(points):
    return ([min(p[i] for p in points) for i in range(3)], [max(p[i] for p in points) for i in range(3)])


def uv_gradient(tri, uv, coordinate):
    """The world-space direction in which one UV coordinate grows, per metre."""
    e1, e2 = sub3(tri[1], tri[0]), sub3(tri[2], tri[0])
    d1, d2 = uv[1][coordinate] - uv[0][coordinate], uv[2][coordinate] - uv[0][coordinate]
    n = cross(e1, e2)
    nn = dot(n, n)
    if nn < 1e-18:
        return (0, 0, 0)
    a = cross(e2, n)
    b = cross(n, e1)
    return tuple((d1 * a[i] + d2 * b[i]) / nn for i in range(3))


def colour_distance(a, b):
    return max(abs(a[i] - b[i]) for i in range(3))


def material_colours(material):
    """Every distinct pixel colour of a material, or its flat factor."""
    if material.factor is not None:
        return [material.factor[:3]]
    if material.image is not None and material.image.exists():
        return list({p[:3] for p in material.image.pixels()})
    return []


# ---------------------------------------------------------------- feature checks

class Result:
    def __init__(self, status, reason):
        self.status = status
        self.reason = reason


def PASS(reason):
    return Result("PASS", reason)


def FAIL(reason):
    return Result("FAIL", reason)


def NA(reason):
    return Result("N/A", reason)


def station(level, name):
    e = level.by_targetname().get(name)
    if e is None:
        raise LookupError(f"no entity with targetname {name}")
    return e


def all_triangles(prims):
    return [t for p in prims for t in p.triangles]


def material_name(prim):
    m = prim.material
    return getattr(m, "json_name", None) or m.name


def find_texture_prim(level, prims, texture):
    """The primitive of a brush texture, by its name or by its picture."""
    for p in prims:
        name = material_name(p)
        if name == texture or name.endswith(f"/{texture}") or name == f"Material_{texture}":
            return p
    source = (SOURCE_TEXTURES / f"{texture}.png").read_bytes()
    wanted = png_decode(source)
    for p in prims:
        image = p.material.image
        if image is not None and image.exists():
            try:
                if image.data() == source or image.decoded()[2] == wanted[2]:
                    return p
            except (ValueError, KeyError):
                continue
    return None


def check_textures(level):
    e = station(level, "fl_textures")
    if not e.render:
        return FAIL("fl_textures has no render mesh")
    problems, notes = [], []
    spans = {}
    gradients = {}
    corners = {}
    for texture, (want_u, want_v) in TEXTURE_FACES.items():
        prim = find_texture_prim(level, e.render, texture)
        if prim is None:
            problems.append(f"no surface uses {texture}")
            continue
        image = prim.material.image
        if image is None or not image.exists():
            problems.append(f"{texture}: image missing ({image.label if image else 'none'})")
            continue
        if image.decoded()[2] != png_decode((SOURCE_TEXTURES / f"{texture}.png").read_bytes())[2]:
            problems.append(f"{texture}: image pixels differ from textures/{texture}.png")
        if prim.material.wrap not in (None, "repeat"):
            problems.append(f"{texture}: sampler {prim.material.wrap}, want repeat")
        if prim.uvs is None or any(uv is None for t in prim.uvs for uv in t):
            problems.append(f"{texture}: no UVs")
            continue
        uvs = [uv for t in prim.uvs for uv in t]
        su = max(u for u, _ in uvs) - min(u for u, _ in uvs)
        sv = max(v for _, v in uvs) - min(v for _, v in uvs)
        spans[texture] = (su, sv)
        if abs(su - want_u) > 0.02 or abs(sv - want_v) > 0.02:
            problems.append(f"{texture}: UV span {su:.3f} x {sv:.3f}, want {want_u:.3f} x {want_v:.3f}")
        gradients[texture] = uv_gradient(prim.triangles[0], prim.uvs[0], 0)
        corners[texture] = {(round(p[0], 4), round(p[1], 4)): uv
                            for t, tu in zip(prim.triangles, prim.uvs) for p, uv in zip(t, tu)}
        corners[texture + "_all"] = [(p, uv) for t, tu in zip(prim.triangles, prim.uvs) for p, uv in zip(t, tu)]
    # The +X metal face shifts the -X wood face's mapping by its offset (16, 8) of 64.
    if "wood" in corners and "metal" in corners:
        shifts = set()
        wood = corners["wood_all"]
        for p, uv in corners["metal_all"]:
            for q, wuv in wood:
                same = [i for i in range(3) if abs(p[i] - q[i]) < 1e-4]
                if len(same) == 2:
                    shifts.add((round((uv[0] - wuv[0]) % 1, 3) % 1, round((uv[1] - wuv[1]) % 1, 3) % 1))
        if not shifts:
            problems.append("could not pair metal and wood corners")
        elif not all(du in (0.25, 0.75) and dv in (0.125, 0.875) for du, dv in shifts):
            problems.append(f"metal offset shift {sorted(shifts)}, want (0.25 or 0.75, 0.125 or 0.875)")
    # The top face (dirt, rotated 90) grows U along another axis than the plain grass top.
    grass = find_texture_prim(level, e.render, "grass")
    if grass and "dirt" in gradients and grass.uvs:
        # Every target's mesh frame is Y-up.
        tops = [(t, uv) for t, uv in zip(grass.triangles, grass.uvs) if normal(t)[1] > 0.99]
        if tops:
            g = uv_gradient(tops[0][0], tops[0][1], 0)
            d = gradients["dirt"]
            cos = abs(dot(g, d)) / (length(g) * length(d) or 1)
            if cos > 0.01:
                problems.append("rotated dirt face grows U along the same axis as the plain grass top")
        else:
            notes.append("grass top not found")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("6 textures with source pixels; UV spans, metal offset and dirt rotation match")


def check_colours(level):
    e = station(level, "fl_colours")
    if not e.render:
        return FAIL("fl_colours has no render mesh")
    found = []
    for prim in e.render:
        found.extend(material_colours(prim.material))
    missing = [c for c in OPAQUE_COLOURS if not any(colour_distance(c, f) <= 2 for f in found)]
    problems = [f"no surface coloured {c}" for c in missing]
    problems += paint_wrap_problems(e)
    # The translucent face is composited over wood: half wood, half paint.
    wood = png_decode((SOURCE_TEXTURES / "wood.png").read_bytes())
    paint, alpha = TRANSLUCENT
    a = alpha / 255
    blended = None
    for prim in e.render:
        image = prim.material.image
        if image is None or not image.exists() or image.size() != (wood[0], wood[1]):
            continue
        rows = image.decoded()[2]
        error = max(abs(rows[y][x][i] - (wood[2][y][x][i] * (1 - a) + paint[i] * a))
                    for y in range(0, wood[1], 7) for x in range(0, wood[0], 7) for i in range(3))
        blended = error if blended is None else min(blended, error)
    if blended is None:
        problems.append("no wood-sized image for the translucent face")
    elif blended > 4:
        problems.append(f"translucent face differs from wood blended with paint by up to {blended:.0f}")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("5 opaque colours exact; translucent paint composited over wood within 4/255")


def paint_wrap_problems(e):
    """Generated paint clamps (docs/engine-export-contract.md), where the file says."""
    wraps = {p.material.wrap for p in e.render if p.material.image is not None} - {None, "clamp"}
    return [f"paint sampler {sorted(wraps)}, want clamp"] if wraps else []


def check_gradients(level):
    e = station(level, "fl_gradients")
    if not e.render:
        return FAIL("fl_gradients has no render mesh")
    images = []
    for prim in e.render:
        m = prim.material
        if m.image is not None and m.image.exists():
            w, h = m.image.size()
            if w > 1 or h > 1:
                images.append((material_name(prim), m.image))
    if not images:
        return FAIL("no gradient image: gradients were not baked")
    problems = []
    for label, stops in GRADIENTS.items():
        ok = False
        for _, image in images:
            pixels = set(p[:3] for p in image.pixels())
            if all(min(colour_distance(s, p) for p in pixels) <= 24 for s in stops):
                ok = True
                break
        if not ok:
            problems.append(f"{label}: no image holds all its stop colours")
    # The plain linear one must change along one image axis only.
    for _, image in images:
        w, h, rows = image.decoded()
        if colour_distance(rows[0][0][:3], (255, 0, 0)) <= 24 or colour_distance(rows[0][0][:3], (0, 0, 255)) <= 24:
            across = colour_distance(rows[0][0], rows[0][w - 1]) > 100
            down = colour_distance(rows[0][0], rows[h - 1][0]) > 100
            if across == down:
                problems.append("red-to-blue image does not run along a single axis")
            break
    problems += paint_wrap_problems(e)
    if problems:
        return FAIL("; ".join(problems))
    return PASS(f"{len(images)} baked gradient images, all stop colours present")


def check_surface(level):
    e = station(level, "fl_surface")
    if not e.render:
        return FAIL("fl_surface has no render mesh")
    got = []
    for prim in e.render:
        m = prim.material
        surface = getattr(m, "json_surface", None)
        if surface:
            got.append(tuple(surface))
        elif m.roughness is not None and m.metallic is not None:
            got.append((m.roughness, m.metallic, m.emission or 0.0))
    # docs/stride-export.md: Stride materials take roughness and metalness, not emission.
    compared = 2 if level.target == "stride" else 3
    missing = [s for s in SURFACES
               if not any(all(abs(s[i] - g[i]) < 0.01 for i in range(compared)) for g in got)]
    if missing:
        return FAIL(f"missing roughness/metallic/emission {missing}; found {sorted(set(got))}")
    if level.target == "stride":
        return PASS("glossiness = 1 - roughness and metalness in .sdmat for all 4 overrides; "
                    "emission is not transferred, as docs/stride-export.md says")
    where = "level JSON surface entries" if any(getattr(p.material, "json_surface", None) for p in e.render) \
        else "material factors"
    note = {"defold": "; the default model material ignores them (export warning)",
            "raylib": "; the default shader ignores them (export warning)"}.get(level.target, "")
    return PASS(f"all 4 surface overrides in {where}{note}")


def check_normal(level):
    e = station(level, "fl_normal")
    if not e.render:
        return FAIL("fl_normal has no render mesh")
    prim = find_texture_prim(level, e.render, "brick_wall")
    if prim is None:
        return FAIL("no brick_wall surface")
    normal_map = prim.material.normal
    if normal_map is None:
        return FAIL("brick_wall material has no normal map")
    if not normal_map.exists():
        return FAIL(f"normal map missing: {normal_map.label}")
    if normal_map.size() != (512, 512):
        return FAIL(f"normal map is {normal_map.size()}, want 512 x 512")
    tangents = getattr(prim, "tangents", None)
    if tangents is False:
        return FAIL("normal-mapped primitive has no TANGENT attribute")
    return PASS("brick_wall carries its 512 x 512 normal map" + (" and tangents" if tangents else ""))


def check_solid(level):
    if level.target in PORTABLE:
        e = station(level, "fl_solid")
        build = e.extra.get("heron", {}).get("build", {})
        if build.get("collision"):
            return NA(f"no physics in the format; extras.heron.build says collision={build['collision']}")
        return NA("no physics in the format (geometry only)")
    problems = []
    for name in ("fl_solid", "worldspawn"):
        e = station(level, name) if name != "worldspawn" else next(
            x for x in level.entities if x.classname == "worldspawn")
        col = e.collision
        if col is None or not col.points():
            problems.append(f"{name}: no collision shape")
            continue
        if col.sensor:
            problems.append(f"{name}: collision is a sensor")
        lo, hi = bounds(col.points())
        size = [hi[i] - lo[i] for i in range(3)]
        if name == "fl_solid" and any(abs(s - 2.0) > 0.01 for s in size):
            problems.append(f"fl_solid collision is {size}, want a 2 m cube")
    if problems:
        return FAIL("; ".join(problems))
    kind = station(level, "fl_solid").collision.kind
    return PASS(f"func_wall has a solid 2 m {kind} shape; worldspawn floor collides")


def unreal_drawn(e):
    """The Unreal script imports any mesh it names as a StaticMeshActor."""
    mesh = e.extra.get("unreal_mesh")
    return f"import script spawns a StaticMeshActor from {mesh}" if mesh not in (None, "None") else None


def check_clip(level):
    e = station(level, "fl_clip")
    if e.render:
        return FAIL(f"clip brush is drawn ({len(all_triangles(e.render))} triangles)")
    if level.target == "unreal":
        drawn = unreal_drawn(e)
        if drawn:
            return FAIL(f"{drawn}, a GLB that holds only the clip's collision mesh")
        return NA(f"not drawn; {UNREAL_COLLISION}")
    if level.target in PORTABLE:
        return NA("not drawn, as required; the format has no collision to carry it")
    col = e.collision
    if col is None or not col.points():
        return FAIL("clip brush has no collision")
    if col.sensor:
        return FAIL("clip brush collision is a sensor")
    lo, hi = bounds(col.points())
    if abs(hi[1] - lo[1] - 3.0) > 0.01:
        return FAIL(f"clip collision height {hi[1] - lo[1]:.3f} m, want 3 m")
    return PASS("not drawn; solid 2 x 3 x 2 m collision")


def check_skip(level):
    e = station(level, "fl_skip")
    if not e.render:
        return FAIL("fl_skip has no render mesh")
    tris = all_triangles(e.render)
    # The collision, where there is one, gives the whole cube; the five drawn sides
    # of the render mesh span it as well.
    full = bounds(e.collision.points()) if e.collision and e.collision.points() else None
    rendered = box_sides(tris, full)
    if len(rendered) != 5:
        return FAIL(f"render covers {len(rendered)} sides, want 5 (skip top omitted)")
    if level.target in PORTABLE:
        return NA("skip face omitted from the mesh, as required; no collision in the format")
    col = e.collision
    if col is None or not col.points():
        return FAIL("fl_skip has no collision")
    if col.kind == "trimesh":
        sides = box_sides(col.triangles, full)
        if len(sides) != 5:
            return FAIL(f"trimesh collision covers {len(sides)} sides, want 5 (skip face is not solid)")
        return PASS("skip top not drawn and left out of the triangle collision")
    return PASS(f"skip top not drawn; {col.kind} collision keeps the whole brush, as a hull must")


def check_hidden(level):
    e = station(level, "fl_hidden")
    if not e.render:
        return FAIL("fl_hidden has no render mesh")
    tris = all_triangles(e.render)
    full = bounds(e.collision.points()) if e.collision and e.collision.points() else None
    if full is None:
        # Without collision, the five drawn sides still span the whole cube.
        full = bounds([p for t in tris for p in t])
    rendered = box_sides(tris, full)
    if len(rendered) != 5:
        return FAIL(f"render covers {len(rendered)} sides, want 5 (hidden face omitted)")
    if level.target in PORTABLE:
        return PASS("hidden face omitted from the mesh; the format has no collision")
    col = e.collision
    if col is None:
        return FAIL("fl_hidden has no collision")
    if col.kind == "trimesh" and len(box_sides(col.triangles, full)) != 6:
        return FAIL("hidden face missing from the triangle collision")
    return PASS(f"hidden face not drawn; {col.kind} collision keeps all 6 sides")


def check_trigger(level):
    e = station(level, "fl_trigger")
    if level.target in PORTABLE:
        drawn = len(all_triangles(e.render)) if e.render else 0
        return NA(f"no sensors in the format; the volume is {'drawn as ' + str(drawn) + ' triangles' if drawn else 'not drawn'}")
    problems = []
    if e.render:
        problems.append(f"trigger is drawn ({len(all_triangles(e.render))} triangles)")
    if level.target == "unreal":
        drawn = unreal_drawn(e)
        if drawn:
            return FAIL(f"{drawn}, a GLB that holds only the trigger's collision mesh")
        return NA(f"not drawn; {UNREAL_COLLISION}")
    if level.target == "gdevelop":
        # docs/gdevelop-export.md: GDevelop has no sensor body; the generated event
        # tests the volume itself.
        if e.collision is not None:
            problems.append("trigger has a physics body")
        if not e.trigger or not e.trigger.get("volumes"):
            problems.append("trigger volume not in the generated event's data")
        if problems:
            return FAIL("; ".join(problems))
        return PASS("not drawn; volume tested by the generated event (GDevelop has no sensors)")
    col = e.collision
    if col is None or not col.points():
        problems.append("no trigger collision")
    elif not col.sensor:
        problems.append("trigger collision is not a sensor")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("not drawn; sensor shape" + (f" ({col.body})" if col.body else ""))


def gdevelop_script(level):
    return level.script


def check_sky(level):
    t = level.target
    root = level.root
    problems = []
    env = level.environment
    if t == "godot":
        text = (root / f"{NAME}.tscn").read_text()
        if not (root / NAME / "sky.hdr").is_file():
            problems.append("sky.hdr missing")
        if "PanoramaSkyMaterial" not in text or 'res://feature_lab/sky.hdr' not in text:
            problems.append("no PanoramaSkyMaterial on sky.hdr")
        if "fog_enabled = true" not in text or "fog_density = 0.02" not in text:
            problems.append("fog not enabled at density 0.02")
        if "fog_light_color = Color(0.705882, 0.745098, 0.784314, 1.0)" not in text:
            problems.append("fog colour differs")
    elif t == "stride":
        for f in ("Sky/Sky.dds", "Sky/Skybox.sdsky", "Sky/Sky.sdtex"):
            if not (root / NAME / f).is_file():
                problems.append(f"{f} missing")
        return FAIL("; ".join(problems)) if problems else PASS("Sky.dds cube and Skybox asset written (Stride writes no fog, as documented)")
    elif t == "gdevelop":
        effects = {fx["effectType"]: fx for layer in level.layout["layers"] for fx in layer.get("effects", [])}
        if "Scene3D::Skybox" not in effects:
            problems.append("no Skybox effect")
        else:
            for key, value in effects["Scene3D::Skybox"]["stringParameters"].items():
                if key.endswith("ResourceName") and not (root / value).is_file():
                    problems.append(f"{value} missing")
        fog = effects.get("Scene3D::ExponentialFog")
        if fog is None:
            problems.append("no ExponentialFog effect")
        elif abs(fog["doubleParameters"]["density"] - FOG["density"] / 100) > 1e-6:
            # GDevelop's density is per pixel, at 100 pixels a metre.
            problems.append(f"fog density {fog['doubleParameters']['density']}, want 0.0002 a pixel")
    else:
        if env is None:
            return FAIL("level JSON has no environment")
        fog = env.get("fog")
        if not fog or abs(fog["density"] - FOG["density"]) > 1e-4:
            problems.append(f"fog {fog}, want density {FOG['density']}")
        elif colour_distance([round(c * 255) for c in fog["color"]], FOG["colour"]) > 2:
            problems.append(f"fog colour {fog['color']}")
        files = {"three": [f"{NAME}/sky.hdr"], "bevy": [f"{NAME}/sky.ktx2", "heron_sky.rs"],
                 "raylib": [f"{NAME}/sky.png"],
                 "defold": [f"{NAME}/sky.cubemap", f"{NAME}.render", f"{NAME}_sky.material"],
                 "cocos": [f"{NAME}/sky_{s}.png" for s in ("top", "bottom", "left", "right", "front", "back")]}
        for f in files.get(t, []):
            if not (root / f).is_file():
                problems.append(f"{f} missing")
        if t == "defold":
            cubemap = (root / NAME / "sky.cubemap").read_text() if (root / NAME / "sky.cubemap").is_file() else ""
            for face in re.findall(r'"(/[^"]+\.png)"', cubemap):
                if not (root / face.lstrip("/")).is_file():
                    problems.append(f"cubemap face {face} missing")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("sky image and fog density 0.02 written")


MOVER = {0: ([0, 2, 4], [0, 128, 0], "linear"), 2: ([0, 1, 3], [0, 32, 0], "hold"),
         5: ([0, 4], [0, 90], "bezier")}


def check_payload(animation, door=False):
    problems = []
    clips = {c["name"]: c for c in animation.get("clips", [])}
    if door:
        if animation.get("autoplay"):
            problems.append("door plays on start")
        for name, values in (("open", [0, 90]), ("close", [90, 0])):
            clip = clips.get(name)
            if clip is None:
                problems.append(f"no {name} clip")
                continue
            keys = clip["channels"][5]["keys"]
            if [k["value"] for k in keys] != values or clip["mode"] != "once":
                problems.append(f"{name} clip RZ {[k['value'] for k in keys]} {clip['mode']}")
        return problems
    clip = clips.get(animation.get("selected", "default"))
    if clip is None:
        return ["no selected clip"]
    if not animation.get("autoplay"):
        problems.append("mover does not play on start")
    if clip["mode"] != "loop" or abs(clip["duration"] - 4) > 1e-9:
        problems.append(f"mode {clip['mode']} duration {clip['duration']}")
    for channel, (times, values, interpolation) in MOVER.items():
        keys = clip["channels"][channel]["keys"]
        if [k["time"] for k in keys] != times or [k["value"] for k in keys] != values:
            problems.append(f"channel {channel} keys {[(k['time'], k['value']) for k in keys]}")
        if any(k["interpolation"] != interpolation for k in keys):
            problems.append(f"channel {channel} not {interpolation}")
    return problems


RUNTIME_FILES = {
    "three": ["heron_animation.js"], "bevy": ["heron_animation.rs"],
    "raylib": [f"{NAME}_animation.h"], "unity": ["HeronBrushAnimation.cs"],
    "flax": ["HeronBrushAnimation.cs"], "stride": [f"{NAME}/HeronBrushStrideRuntime.cs"],
    "cocos": ["HeronBrushLevel.ts"], "defold": [f"{NAME}/func_animate.script"],
}


def check_animation(level):
    t = level.target
    if t == "godot":
        return godot_animation(level, door=False)
    mover = station(level, "fl_mover")
    if mover.animation is None:
        return FAIL("fl_mover has no _heron_animation payload")
    problems = check_payload(mover.animation)
    problems += runtime_problems(level)
    if t == "raylib":
        header = (level.root / f"{NAME}_animation.h").read_text()
        if "{2.00000000000000000e0,1.28000000000000000e2" not in header:
            problems.append("C header lacks the X key at 2 s = 128")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("loop, linear X 0/128/0, held Z 0/32/0, bezier RZ 0/90, autoplay; runtime written")


def runtime_problems(level):
    t = level.target
    problems = [f"{f} missing" for f in RUNTIME_FILES.get(t, []) if not (level.root / f).is_file()]
    if t == "gdevelop" and "const DATA" not in gdevelop_script(level):
        problems.append("no generated animation event")
    if t == "unity":
        importer = (level.root / "Editor" / "HeronBrushLevelImporter.cs").read_text()
        for component, count in (("HeronBrushAnimation", 2), ("HeronBrushAnimationTrigger", 2)):
            if importer.count(f"AddComponent<{component}>") != count:
                problems.append(f"importer does not attach {count} {component}")
    if t == "defold":
        script = (level.root / NAME / "func_animate.script").read_text()
        if "hb_data = json.decode" not in script:
            problems.append("func_animate.script lacks the baked payload")
    return problems


def godot_animation(level, door):
    sections = level.sections
    clips = {}
    for kind, attrs, body in sections:
        if kind == "sub_resource" and attrs.get("type") == "Animation":
            name = body_value(body, "resource_name").strip('"')
            tracks = {}
            for path, keys in re.findall(r'tracks/\d+/path = NodePath\("\.:(\w+)"\)\n(?:[^\n]*\n){2}tracks/\d+/keys = \{\n(.*?)\n\}', body, re.S):
                times = [float(x) for x in re.search(r'"times": PackedFloat64Array\((.*?)\)', keys).group(1).split(",") if x.strip()]
                if '"points"' in keys:
                    points = [float(x) for x in re.search(r'"points": PackedFloat64Array\((.*?)\)', keys).group(1).split(",")]
                    values = points[0::5]
                else:
                    values = json.loads(re.search(r'"values": (\[.*?\])', keys).group(1))
                tracks[path] = (times, values)
            clips.setdefault(name, []).append((attrs["id"], body, tracks))
    entity = station(level, "fl_door" if door else "fl_mover")
    node = entity.extra.get("animation_node")
    if node is None:
        return FAIL(f"{entity.name} has no HeronAnimation node")
    problems = []
    libraries = [(attrs["id"], body) for kind, attrs, body in sections
                 if kind == "sub_resource" and attrs.get("type") == "AnimationLibrary"]
    player = next((b for k, a, b in sections if k == "node" and a.get("type") == "AnimationPlayer"
                   and a.get("parent") == f"{entity.name}/HeronAnimation"), "")
    library_id = re.search(r'SubResource\("(HeronLibrary_\d+)"\)', player)
    if not library_id:
        return FAIL("AnimationPlayer has no library")
    library = dict(libraries)[library_id.group(1)]
    named = dict(re.findall(r'"(\w+)": SubResource\("(HeronClip_\d+)"\)', library))
    by_id = {cid: (body, tracks) for name, items in clips.items() for cid, body, tracks in items}
    if door:
        for name, values in (("open", [0, 90]), ("close", [90, 0])):
            if name not in named:
                problems.append(f"no {name} clip")
                continue
            _, tracks = by_id[named[name]]
            if tracks.get("map_rz", ([], []))[1] != values:
                problems.append(f"{name} map_rz {tracks.get('map_rz')}")
        if "autoplay" in node and "true" in node:
            problems.append("door autoplays")
    else:
        body, tracks = by_id[named.get("default")]
        if body_value(body, "loop_mode") != "1":
            problems.append("default clip does not loop")
        if tracks.get("map_x") != ([0, 2, 4], [0, 128, 0]):
            problems.append(f"map_x {tracks.get('map_x')}")
        if tracks.get("step_z") != ([0, 1, 3], [0.0, 32.0, 0.0]):
            problems.append(f"step_z {tracks.get('step_z')}")
        if tracks.get("map_rz", ([], []))[1] != [0, 90]:
            problems.append(f"map_rz {tracks.get('map_rz')}")
        if entity.extra.get("type") != "AnimatableBody3D":
            problems.append(f"mover is a {entity.extra.get('type')}, want AnimatableBody3D")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("native AnimationPlayer: open/close clips" if door else
                "native AnimationPlayer: looping map_x 0/128/0, step_z hold 0/32/0, bezier map_rz 0/90, AnimatableBody3D")


def check_door(level):
    t = level.target
    if t == "godot":
        result = godot_animation(level, door=True)
        if result.status != "PASS":
            return result
        problems = []
        for name, action in (("fl_button", "toggle"), ("fl_trigger", "open")):
            e = station(level, name)
            node = e.extra.get("trigger_node", "")
            if not node or "func_animate_1" not in node and "fl_door" not in node:
                problems.append(f"{name}: no HeronAnimationTrigger aimed at the door")
            if e.props.get("animation_action", (None, None))[1] != action:
                problems.append(f"{name}: action {e.props.get('animation_action')}")
        if problems:
            return FAIL("; ".join(problems))
        return PASS("open/close clips; button (toggle) and trigger (open, wait 1.5) wired to the door")
    door = station(level, "fl_door")
    if door.animation is None:
        return FAIL("fl_door has no animation payload")
    problems = check_payload(door.animation, door=True)
    for name, action, wait in (("fl_button", "toggle", 0), ("fl_trigger", "open", 1.5)):
        e = station(level, name)
        trigger = e.trigger
        if trigger is None:
            problems.append(f"{name}: no _heron_trigger payload")
            continue
        if trigger["targets"] != ["fl_door"] or trigger["action"] != action or abs(trigger["wait"] - wait) > 1e-9:
            problems.append(f"{name}: {trigger['targets']} {trigger['action']} wait {trigger['wait']}")
        if not trigger.get("volumes"):
            problems.append(f"{name}: no activation volume")
    problems += runtime_problems(level)
    if t == "raylib":
        header = (level.root / f"{NAME}_animation.h").read_text()
        if '"open",""' not in header or '"toggle",""' not in header:
            problems.append("C trigger table lacks open/toggle")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("open/close clips; button toggles, trigger opens with wait 1.5, both aim at the door")


WANT_PROPS = {
    "fl_probe": {"charge": ("int", 7), "armed": ("bool", True), "sequence": ("array", [2, 3, 5]),
                 "label": ("string", "probe"), "note": ("string", "free text, with a comma")},
    "fl_light": {"light_energy": ("float", 2.5), "omni_range": ("float", 8.0)},
    "fl_camera": {"fov": ("float", 62.5), "current": ("int", 1)},
    "fl_water": {"swim_speed": ("float", 2.5), "water_priority": ("int", 3)},
}


def check_properties(level):
    problems = []
    for name, wanted in WANT_PROPS.items():
        e = station(level, name)
        for key, (kind, value) in wanted.items():
            got = e.props.get(key)
            if got is None:
                problems.append(f"{name}.{key} missing")
                continue
            got_kind, got_value = got
            if level.target == "gdevelop" and kind in ("int", "float"):
                got_kind = kind if got_kind in ("int", "float") else got_kind
            if got_kind == "float" and kind == "int" and float(got_value).is_integer():
                got_kind = "int"
            if got_kind != kind or got_value != value:
                problems.append(f"{name}.{key} = {got_kind} {got_value!r}, want {kind} {value!r}")
    colour = station(level, "fl_light").props.get("light_color")
    if colour is None or colour[0] != "color" and level.target != "gdevelop":
        problems.append(f"fl_light.light_color is {colour}, want a colour")
    if level.target == "defold":
        script = (level.root / NAME / "fl_probe.script").read_text()
        for line in ('go.property("charge", 7)', 'go.property("armed", true)'):
            if line not in script:
                problems.append(f"fl_probe.script lacks {line}")
    if level.target == "raylib":
        text = (level.root / f"{NAME}.level").read_text()
        for line in ("prop int charge 7", "prop bool armed 1", "prop array sequence 2 3 5"):
            if line not in text:
                problems.append(f".level lacks '{line}'")
    if problems:
        return FAIL("; ".join(problems))
    note = {"defold": "; scripts hold scalars, the JSON holds the array (documented)",
            "raylib": "; .level and JSON agree"}.get(level.target, "")
    return PASS("int, float, bool, array, colour and free-text values keep their types; "
                "project default overridden by the authored charge=7" + note)


def check_links(level):
    want = {"fl_wp_a": ["fl_wp_b"], "fl_probe": ["fl_wp_a"], "fl_trigger": ["fl_door"],
            "fl_button": ["fl_door"]}
    names = level.by_name()
    problems = []
    for source, targets in want.items():
        e = station(level, source)
        got = [names[t].targetname if t in names and names[t].targetname else t for t in e.targets]
        if got != targets:
            problems.append(f"{source} -> {got}, want {targets}")
        if e.props.get("target", (None, None))[1] != targets[0]:
            problems.append(f"{source}.target property {e.props.get('target')}")
    if problems:
        return FAIL("; ".join(problems))
    how = {"godot": "NodePaths", "gdevelop": "heron_targets variables",
           "gltf": "target properties in extras", "glb": "target properties in extras"}.get(
        level.target, "resolved targets lists")
    return PASS(f"4 links resolved as {how}")


def check_lights(level):
    t = level.target
    light = station(level, "fl_light")
    spot = station(level, "fl_spot")
    data_ok = light.props.get("light_energy", (None, None))[1] == 2.5 and \
        spot.props.get("spot_angle", (None, None))[1] in (30, 30.0)
    if t == "godot":
        problems = []
        if light.extra.get("type") != "OmniLight3D":
            problems.append(f"fl_light is a {light.extra.get('type')}")
        if spot.extra.get("type") != "SpotLight3D":
            problems.append(f"fl_spot is a {spot.extra.get('type')}")
        body = light.extra["body"]
        if "light_energy = 2.5" not in body or "omni_range = 8.0" not in body or \
                "light_color = Color(1.0, 0.784314, 0.470588, 1.0)" not in body:
            problems.append("OmniLight3D colour, energy or range differs")
        if "spot_angle = 30.0" not in spot.extra["body"]:
            problems.append("SpotLight3D angle differs")
        return FAIL("; ".join(problems)) if problems else PASS("OmniLight3D and SpotLight3D with colour, energy, range and angle")
    if t == "unity":
        problems = []
        point, cone = light.extra.get("light"), spot.extra.get("light")
        if not point or point["kind"] != "Point" or point["energy"] != 2.5 or point["range"] != 8.0:
            problems.append(f"point light {point}")
        if not cone or cone["kind"] != "Spot" or abs(cone["cone"] - 60.0) > 1e-6:
            problems.append(f"spot light {cone}")
        importer = (level.root / "Editor" / "HeronBrushLevelImporter.cs").read_text()
        for line in ("type = LightType.Point;", "range = 8.000000f;", "type = LightType.Spot;",
                     "spotAngle = 60.000000f;", "intensity = 2.500000f;"):
            if line not in importer:
                problems.append(f"importer lacks '{line}'")
        return FAIL("; ".join(problems)) if problems else PASS("importer adds Point and Spot Light components (cone 60 = 2 x 30 degrees, range 8, intensity 2.5)")
    if t == "unreal":
        py = (level.root / f"import_{NAME}.py").read_text()
        problems = []
        if "PointLight" not in py or "SpotLight" not in py:
            problems.append("importer creates no point or spot light actor")
        return FAIL("; ".join(problems)) if problems else PASS("importer spawns point and spot light actors")
    if t == "obj":
        return NA("OBJ is geometry only")
    if not data_ok:
        return FAIL("light properties missing from the entity data")
    return NA("no native light; light colour, energy and range stay in the entity data for game code "
              "(docs/common-entities.md lists native lights for Godot, Unity and Unreal only)")


def check_water(level):
    t = level.target
    e = station(level, "fl_water")
    if t == "godot":
        node = e.extra.get("water_node")
        problems = []
        if e.extra.get("type") != "Area3D":
            problems.append(f"water is a {e.extra.get('type')}")
        if not node or "data_json" not in node:
            problems.append("no heron_water node with data")
        elif '\\"swim_speed\\":2.5' not in node:
            problems.append("water data lacks swim_speed 2.5")
        if "HeronWater_water_volume_0" not in e.extra["body"] and not any(
                "HeronWater" in p.material.name for p in e.render or []):
            problems.append("no water surface material")
        if not (level.root / NAME / "heron_water_view.gd").is_file():
            problems.append("heron_water_view.gd missing")
        return FAIL("; ".join(problems)) if problems else PASS("Area3D with heron_water data, water shader surface and view script")
    water = e.props.get("_heron_water")
    if water is None:
        return FAIL("no _heron_water payload")
    payload = json.loads(water[1])
    problems = []
    text = json.dumps(payload)
    if '"swim_speed": 2.5' not in text and '"swim_speed":2.5' not in text:
        problems.append("payload lacks swim_speed 2.5")
    if not payload.get("volumes"):
        problems.append("payload has no volumes")
    if t != "gdevelop" and (e.collision is None or not e.collision.sensor):
        problems.append("water collision is not a sensor")
    # The baseline surface is the volume's upward faces only (docs/water-export.md).
    if e.render and any(normal(tri)[1] < 0.99 for tri in all_triangles(e.render)):
        problems.append("water surface mesh has faces that do not point up")
    files = {"three": ["heron_water.js"], "bevy": ["heron_water.rs", "heron_water_render.rs"],
             "raylib": [f"{NAME}_water.h"], "defold": [f"{NAME}_water.lua", f"{NAME}_water.material"],
             "unity": ["HeronBrushWater.cs", "HeronBrushWater.shader"], "flax": ["HeronBrushWater.cs"],
             "stride": [f"{NAME}/HeronBrushStrideRuntime.cs"], "cocos": ["HeronBrushLevel.ts"]}
    problems += [f"{f} missing" for f in files.get(t, []) if not (level.root / f).is_file()]
    if t == "unity" and "AddComponent<HeronBrushWater>" not in (
            level.root / "Editor" / "HeronBrushLevelImporter.cs").read_text():
        problems.append("importer does not attach HeronBrushWater")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("exact-volume _heron_water payload, sensor and the target's water adapter")


def check_models(level):
    a, b = station(level, "fl_model_a"), station(level, "fl_model_b")
    problems = []
    for e in (a, b):
        if not e.render or not all_triangles(e.render):
            problems.append(f"{e.targetname} has no mesh")
    if problems:
        return FAIL("; ".join(problems))
    for e in (a, b):
        for prim in e.render:
            m = prim.material
            if m.image is None or not m.image.exists():
                if m.factor is None:
                    problems.append(f"{e.targetname}: material {m.name} has no albedo")
    if level.target == "obj":
        if problems:
            return FAIL("; ".join(problems))
        return PASS("both models written with albedo (OBJ flattens instances, as documented)")
    if a.mesh_ref is None or a.mesh_ref != b.mesh_ref:
        problems.append(f"instances do not share a mesh ({a.mesh_ref} / {b.mesh_ref})")
    if level.target in ("gltf", "glb") and str(ROOT) in json.dumps(level.gltf.json):
        problems.append("the file names the model's absolute source folder")
    # Shared geometry puts each copy's placement on its own transform.
    ta, tb = a.extra.get("transform"), b.extra.get("transform")
    if ta and tb:
        if [round(x, 6) for x in ta["scale"]] != [32.0] * 3 or [round(x, 6) for x in tb["scale"]] != [48.0] * 3:
            problems.append(f"model scales {ta['scale']} / {tb['scale']}, want 32 / 48")
        if abs(abs(tb["rotation_yxz"][1] - ta["rotation_yxz"][1]) - math.pi / 4) > 1e-6:
            problems.append("fl_model_b is not turned 45 degrees from fl_model_a")
    if problems:
        return FAIL("; ".join(problems))
    return PASS("two instances share one mesh with albedo")


def check_model_pbr(level):
    a = station(level, "fl_model_a")
    if not a.render:
        return FAIL("fl_model_a has no mesh")
    maps = set()
    for prim in a.render:
        maps |= prim.material.maps
    if level.target == "unity":
        script = (level.root / "Editor" / "HeronBrushLevelImporter.cs").read_text()
        calls = re.findall(r"ApplyPbr\(root, here, \"(heron_import_\w+)\".*?(true|false), (true|false), (true|false), (true|false), \d", script)
        if not calls:
            return FAIL("importer has no ApplyPbr call for the model")
        flags = calls[0][1:]
        if flags != ("true", "true", "true", "true"):
            return FAIL(f"ApplyPbr mr/normal/ao/emissive flags {flags}")
        missing = [s for s in ("mr", "normal", "ao", "emissive")
                   if not (level.root / f"{NAME}_paint" / f"{calls[0][0]}_{s}.png").is_file()]
        if missing:
            return FAIL(f"companion images missing: {missing}")
        return PASS("ApplyPbr gets metallic-roughness, normal, AO and emissive maps; companion PNGs present")
    want = {"mr", "normal", "ao", "emissive"}
    if not want <= maps:
        return FAIL(f"model material maps {sorted(maps)}, want {sorted(want)}")
    return PASS("metallic-roughness, normal, occlusion and emissive maps on the model material")


CHECKS = {
    "textures": check_textures, "colours": check_colours, "gradients": check_gradients,
    "surface": check_surface, "normal": check_normal, "solid": check_solid, "clip": check_clip,
    "skip": check_skip, "hidden": check_hidden, "trigger": check_trigger, "sky": check_sky,
    "animation": check_animation, "door": check_door, "properties": check_properties,
    "links": check_links, "lights": check_lights, "water": check_water, "models": check_models,
    "model_pbr": check_model_pbr,
}


def run(target, root):
    try:
        level = load(target, root)
    except (OSError, ValueError, KeyError, IndexError, StopIteration) as error:
        return {f: FAIL(f"export could not be read: {error!r}") for f, _ in FEATURES}
    results = {}
    for feature, _ in FEATURES:
        not_claimed = NOT_CLAIMED.get(feature, {}).get(target)
        try:
            result = CHECKS[feature](level)
        except (LookupError, OSError, ValueError, AttributeError, TypeError) as error:
            result = FAIL(f"check could not run: {error!r}")
        if not_claimed and result.status != "N/A":
            seen = "data present" if result.status == "PASS" else "file check"
            result = NA(f"{not_claimed} ({seen}: {result.reason})")
        results[feature] = result
    return results


def markdown(all_results):
    targets = list(all_results)
    lines = ["| Feature | " + " | ".join(TARGET_NAMES[t] for t in targets) + " |",
             "| --- |" + " --- |" * len(targets)]
    for feature, title in FEATURES:
        lines.append(f"| {title} | " + " | ".join(all_results[t][feature].status for t in targets) + " |")
    lines.append("")
    for t in targets:
        lines.append(f"### {TARGET_NAMES[t]}")
        lines.append("")
        for feature, title in FEATURES:
            r = all_results[t][feature]
            lines.append(f"- {title}: {r.status}. {r.reason}")
        lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("targets", nargs="*", metavar="target")
    parser.add_argument("--root", type=Path, default=Path("/tmp"),
                        help="parent of the heron-feature-<target> folders")
    parser.add_argument("--markdown", type=Path, help="write the table and reasons here")
    parser.add_argument("--json", type=Path, help="write the results as JSON here")
    args = parser.parse_args()
    targets = args.targets or TARGETS
    unknown = sorted(set(targets) - set(TARGETS))
    if unknown:
        parser.error(f"unknown target {', '.join(unknown)}")
    results = {t: run(t, args.root / f"heron-feature-{t}") for t in targets}
    text = markdown(results)
    print(text)
    if args.markdown:
        args.markdown.write_text(text + "\n")
    if args.json:
        args.json.write_text(json.dumps({t: {f: {"status": r.status, "reason": r.reason}
                                             for f, r in rs.items()} for t, rs in results.items()},
                                        indent=1) + "\n")
    failed = sum(r.status == "FAIL" for rs in results.values() for r in rs.values())
    print(f"{failed} failures", file=sys.stderr)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
