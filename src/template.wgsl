struct FragmentData {
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vert_main([[builtin(vertex_index)]] vert_index: u32) -> FragmentData {
    var data: FragmentData;
    let x = f32(1 - i32(vert_index)) * 0.5;
    let y = f32(i32(vert_index & 1u) * 2 - 1) * 0.5;
    data.position = vec4<f32>(x, y, 0.0, 1.0);
    return data;
}

[[stage(fragment)]]
fn frag_main(data: FragmentData) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 0.1, 0.2, 1.0);
}
