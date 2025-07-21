struct VertexOut {
    @builtin(position) position: vec4f,
    @location(10) color: vec4f, // Can be any location
}

@vertex
fn vertex_main(
    @location(15) position: vec4f,
    @location(14) color: vec4f,
) -> VertexOut {
    var output: VertexOut;
    output.position = position;
    output.color = color;
    return output;
}

struct FragmentOut {
    @location(0) color: vec4f,
    @location(1) color2: vec4f
}

@fragment
fn fragment_main(fragData: VertexOut) -> FragmentOut {
    var output: FragmentOut;
    output.color = 1.0 - fragData.color; // 反色
    output.color2 = fragData.color;
    return output;
}