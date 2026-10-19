#version 300 es

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

// Metallic-roughness PBR for the exported glTF materials: vertex stage.
// raylib binds these attribute and matrix names itself. vertexTangent is the
// glTF TANGENT attribute (xyz, w = bitangent sign); it reads as zero for a
// mesh without one, and the fragment stage then derives a frame itself.

in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;
in vec4 vertexTangent;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal; // inverse transpose of matModel, for nonuniform scale

out vec3 fragWorld;
out vec2 fragTexCoord;
out vec3 fragNormal;
out vec4 fragTangent;

void main()
{
    fragWorld = vec3(matModel*vec4(vertexPosition, 1.0));
    fragTexCoord = vertexTexCoord;
    fragNormal = normalize(mat3(matNormal)*vertexNormal);
    fragTangent = vec4(mat3(matModel)*vertexTangent.xyz, vertexTangent.w);
    gl_Position = mvp*vec4(vertexPosition, 1.0);
}
