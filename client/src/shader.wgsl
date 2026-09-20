/*
 * SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
 *
 * SPDX-License-Identifier: MPL-2.0
 */

struct VertexInput {
    @location(0) position: vec3<f32>,
}
struct InstanceInput {
    @location(5) position: vec2<f32>,
    @location(6) tile_idx: i32,
}
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) tile_idx: i32,
};

const colors = array<vec4<f32>, 7>(
    vec4<f32>(1.0, 0.0, 0.18823529411764706, 1.0),
    vec4<f32>(1.0, 0.4392156862745098, 0.0, 1.0),
    vec4<f32>(1.0, 0.7647058823529411, 0.0, 1.0),
    vec4<f32>(0.4588235294117647, 0.9333333333333333, 0.2235294117647059, 1.0),
    vec4<f32>(0.0, 0.9411764705882353, 0.8274509803921568, 1.0),
    vec4<f32>(0.25098039215686274, 0.6235294117647059, 0.9725490196078431, 1.0),
    vec4<f32>(0.7137254901960784, 0.47058823529411764, 0.9607843137254902, 1.0),
);

@group(1) @binding(0)
var<uniform> matrix: mat4x4f;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var mtx: mat4x4f = transpose(mat4x4f(
        1.0, 0.0, 0.0, instance.position.x,
        0.0, 1.0, 0.0, instance.position.y,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ));
    var out: VertexOutput;
    out.clip_position = matrix * mtx * vec4<f32>(model.position, 1.0);
    out.tile_idx = instance.tile_idx;
    return out;
}

@group(0) @binding(0)
var text: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return colors[in.tile_idx];
    // return textureSample(text, samp, in.tex_coords);
}
