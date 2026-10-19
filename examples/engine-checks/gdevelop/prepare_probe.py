"""Add a falling physics ball to a Heron Brush GDevelop export, for a native check."""
import json, sys
path, level = sys.argv[1], sys.argv[2]
x, y, z = (float(v) for v in sys.argv[3:6])
p = json.load(open(path))
layout = next(l for l in p["layouts"] if l["name"] == level)
layout["objects"] = [o for o in layout["objects"] if o["name"] != "ProbeBall"]
layout["instances"] = [i for i in layout["instances"] if i["name"] != "ProbeBall"]
content = {"width": 50, "height": 50, "depth": 50, "enableTextureTransparency": False, "facesOrientation": "Y",
           "backFaceUpThroughWhichAxisRotation": "X", "materialType": "StandardWithoutMetalness", "tint": "230;60;40",
           "isCastingShadow": True, "isReceivingShadow": True}
for f in ["front", "back", "left", "right", "top", "bottom"]:
    content.update({f + "FaceResourceName": "", f + "FaceVisible": True, f + "FaceResourceRepeat": False})
layout["objects"].append({"assetStoreId": "", "name": "ProbeBall", "type": "Scene3D::Cube3DObject", "variables": [],
    "effects": [], "content": content, "behaviors": [{"name": "Physics3D", "type": "Physics3D::Physics3DBehavior", "object3D": "Object3D",
    "bodyType": "Dynamic", "bullet": False, "fixedRotation": False, "shape": "Sphere", "meshShapeResourceName": "", "shapeOrientation": "Z",
    "shapeDimensionA": 25, "shapeDimensionB": 0, "shapeDimensionC": 0, "shapeOffsetX": 0, "shapeOffsetY": 0, "shapeOffsetZ": 0,
    "massCenterOffsetX": 0, "massCenterOffsetY": 0, "massCenterOffsetZ": 0, "density": 1, "massOverride": 0, "friction": 0.3,
    "restitution": 0.0, "linearDamping": 0.1, "angularDamping": 0.1, "gravityScale": 1, "layers": 16, "masks": 17}]})
layout["instances"].append({"angle": 0, "customSize": False, "height": 0, "width": 0, "layer": "", "locked": False, "name": "ProbeBall",
    "persistentUuid": "00000000-0000-4000-a000-00000000ba11", "x": x, "y": y, "z": z, "zOrder": 999,
    "numberProperties": [], "stringProperties": [], "initialVariables": []})
json.dump(p, open(path, "w"), indent=1)
