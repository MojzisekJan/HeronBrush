components {
  id: "player"
  component: "/main/player.script"
}
embedded_components {
  id: "collision"
  type: "collisionobject"
  data: "type: COLLISION_OBJECT_TYPE_KINEMATIC\nmass: 0.0\nfriction: 0.0\nrestitution: 0.0\ngroup: \"player\"\nmask: \"default\"\nlinear_damping: 0.0\nangular_damping: 0.0\nlocked_rotation: true\nembedded_collision_shape {\n  shapes {\n    shape_type: TYPE_CAPSULE\n    position {\n    }\n    rotation {\n    }\n    index: 0\n    count: 2\n  }\n  data: 0.4\n  data: 1.2\n}\n"
  position {
    x: 0.0
    y: 0.0
    z: 0.0
  }
  rotation {
    x: 0.0
    y: 0.0
    z: 0.0
    w: 1.0
  }
}
