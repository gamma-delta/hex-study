#version 100
// Post-processing shader where we "punch holes" for the light

precision highp float;

varying vec2 uv;
varying vec4 color;
varying vec3 lightColor;

uniform sampler2D Texture;
uniform sampler2D lights;

// The light texture starts black, and gets splotches drawn on top of it.

void main() {
    gl_FragColor = texture2D(Texture, uv) * texture2D(lights, uv);
}
