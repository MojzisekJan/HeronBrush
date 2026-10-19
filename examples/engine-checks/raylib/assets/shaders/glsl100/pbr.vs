#version 100


// Metallic-roughness PBR for the exported glTF materials: vertex stage.
// raylib binds these attribute and matrix names itself. vertexTangent is the
// glTF TANGENT attribute (xyz, w = bitangent sign); it reads as zero for a
// mesh without one, and the fragment stage then derives a frame itself.

attribute vec3 vertexPosition;
attribute vec2 vertexTexCoord;
attribute vec3 vertexNormal;
attribute vec4 vertexTangent;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal; // inverse transpose of matModel, for nonuniform scale

varying vec3 fragWorld;
varying vec2 fragTexCoord;
varying vec3 fragNormal;
varying vec4 fragTangent;

void main()
{
    fragWorld = vec3(matModel*vec4(vertexPosition, 1.0));
    fragTexCoord = vertexTexCoord;
    fragNormal = normalize(mat3(matNormal[0].xyz, matNormal[1].xyz, matNormal[2].xyz)*vertexNormal);
    fragTangent = vec4(mat3(matModel[0].xyz, matModel[1].xyz, matModel[2].xyz)*vertexTangent.xyz, vertexTangent.w);
    gl_Position = mvp*vec4(vertexPosition, 1.0);
}
