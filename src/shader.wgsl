// Uniforms
struct CameraUniform {
	view_proj: mat4x4<f32>,	
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

// Textures
@group(1) @binding(0) var texture: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

struct VertexInput {
	@location(0) position: vec3<f32>,	
	@location(1) normal: vec3<f32>,
	@location(2) texture_ambient: u32,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
	@location(1) texture_index: u32,
	@location(2) ambient: f32
};

@vertex
fn vs_main(
	input: VertexInput,
	@builtin(vertex_index) vertex_id: u32
) -> VertexOutput {
	var out: VertexOutput;

	var tex_coords = array<vec2<f32>, 4>(
		vec2<f32>(0.0, 0.0),
		vec2<f32>(0.0, 1.0),
		vec2<f32>(1.0, 1.0),
		vec2<f32>(1.0, 0.0),
	);

	out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
	out.uv = tex_coords[vertex_id % 4];

	out.texture_index = (input.texture_ambient >> 16);
	out.ambient = f32((input.texture_ambient << 16) >> 16) / 3.0;

	return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
	var diffuse = textureSample(texture, texture_sampler, input.uv, input.texture_index);
	var ambient = vec4<f32>(vec3<f32>(1.0) * input.ambient, 1.0);

	return ambient * diffuse;
}
