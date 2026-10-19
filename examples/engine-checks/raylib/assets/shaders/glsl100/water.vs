#version 100


// A stylized water surface: vertex stage. See water.fs.

attribute vec3 vertexPosition;
attribute vec2 vertexTexCoord;
attribute vec3 vertexNormal;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;

varying vec2 fragTexCoord;
varying vec3 vWorld;
varying vec3 vNormal;

void main()
{
    fragTexCoord = vertexTexCoord;
    vWorld = vec3(matModel*vec4(vertexPosition, 1.0));
    vNormal = normalize(mat3(matNormal[0].xyz, matNormal[1].xyz, matNormal[2].xyz)*vertexNormal);
    gl_Position = mvp*vec4(vertexPosition, 1.0);
}
