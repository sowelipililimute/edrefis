/*
 * SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
 *
 * SPDX-License-Identifier: MPL-2.0
 */

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> matrix: mat4x4f;

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = matrix * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    out.uv = model.uv;
    return out;
}

@group(0) @binding(0)
var text: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    return in.color * textureSample(text, samp, in.uv);
}
