// 1. Input vertex data (position and texture coordinates)
struct VertexInput {
    @location(0) position: vec3<f32>,     // Where to place the vertex
    @location(1) tex_coords: vec2<f32>,   // Which part of the texture to use
};

// 2. Output to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // Final screen position
    @location(0) tex_coords: vec2<f32>,           // Pass texture coords to fragment shader
};

// 3. Texture bindings
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;  // The actual texture
@group(0) @binding(1) var s_diffuse: sampler;          // How to sample the texture

// 4. Vertex shader - positions vertices
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Convert from pixel coordinates to clip space (-1 to 1)
    out.clip_position = vec4<f32>(
        (in.position.x / 512.0) * 2.0 - 1.0,  // X position
        -((in.position.y / 512.0) * 2.0 - 1.0), // Y position (flipped)
        0.0,                                    // Z position (2D, so always 0)
        1.0                                     // W component
    );
    out.tex_coords = in.tex_coords;
    return out;
}

// 5. Fragment shader - colors pixels
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);  // Sample the texture
}