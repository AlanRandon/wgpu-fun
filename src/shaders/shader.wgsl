struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) position: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;

    var out: VertexOutput;
    out.position = vec2f(x, y);
    out.clip_position = vec4f(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(0.1, in.position, 1.0);
}
