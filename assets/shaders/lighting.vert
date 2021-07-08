#version 100

precision highp float;

attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying vec2 uv;
varying vec4 color;
varying vec3 lightColor;

uniform mat4 Model;
uniform mat4 Projection;
uniform sampler2D lights;

// Standard vertex shader
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;

    // Sample the light value here so it will be lerped smoother
    lightColor = texture2D(lights, uv).rgb;
}
