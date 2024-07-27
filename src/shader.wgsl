// Uniforms
struct CameraUniform {
	view_proj: mat4x4<f32>,	
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

// Textures
@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

struct VertexInput {
	@location(0) position: vec3<f32>,	
	@location(1) normal: vec3<f32>,
	@location(2) uv: vec2<f32>,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
	input: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;

	out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
	out.uv = input.uv;

	return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(texture, texture_sampler, input.uv);
}

