struct VertexInput {
	@location(0) position: vec3<f32>,	
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
	input: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;

	out.clip_position = vec4<f32>(input.position, 1.0);

	return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
	return vec4<f32>(1.0);
}

