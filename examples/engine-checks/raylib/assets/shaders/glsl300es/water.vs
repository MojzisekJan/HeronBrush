#version 300 es

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

// A stylized water surface: vertex stage. See water.fs.

in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;

out vec2 fragTexCoord;
out vec3 vWorld;
out vec3 vNormal;

void main()
{
    fragTexCoord = vertexTexCoord;
    vWorld = vec3(matModel*vec4(vertexPosition, 1.0));
    vNormal = normalize(mat3(matNormal)*vertexNormal);
    gl_Position = mvp*vec4(vertexPosition, 1.0);
}
