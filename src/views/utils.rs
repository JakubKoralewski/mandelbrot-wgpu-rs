pub use crate::utils::{
	AtomicDevice,
	ABSOLUTE_PATH, Position, POSITION_SIZE,
	WindowSize, WINDOW_SIZE_SIZE,
	Zoom, ZOOM_SIZE,
	Iterations, ITERATIONS_SIZE,
	Vertex, VERTEX_SIZE,
	Julia, JULIA_SIZE,
};

pub const ZOOM_SENSITIVITY: f32 = 0.9;

pub fn create_buffer<T: 'static + Copy>(device: &wgpu::Device, value: T) -> wgpu::Buffer {
	device.create_buffer_mapped(
		1,
		wgpu::BufferUsage::UNIFORM
			| wgpu::BufferUsage::COPY_DST
	).fill_from_slice(&[value])
}

lazy_static! {
	pub static ref WHOLE_VERTICES: Vec<Vertex> = vec![
		Vertex{pos: [1f32, 1f32]},
		Vertex{pos: [-1f32, 1f32]},
		Vertex{pos: [1f32, -1f32]},
		Vertex{pos: [-1f32, -1f32]},
	];

	pub static ref LEFT_HALF_VERTICES: Vec<Vertex> = vec![
		Vertex{pos: [0f32, 1f32]},
		Vertex{pos: [-1f32, 1f32]},
		Vertex{pos: [0f32, -1f32]},
		Vertex{pos: [-1f32, -1f32]},
	];

	pub static ref RIGHT_HALF_VERTICES: Vec<Vertex> = vec![
		Vertex{pos: [1f32, 1f32]},
		Vertex{pos: [0f32, 1f32]},
		Vertex{pos: [1f32, -1f32]},
		Vertex{pos: [0f32, -1f32]},
	];
}
use super::prelude::*;

pub fn new(
	device: &wgpu::Device,
	size: dpi::LogicalSize,
	is_julia: bool,
	vertices: Vec<Vertex>
) -> FractalViewData {
	let window_size = WindowSize {
		size: [size.width as f32, size.height as f32]
	};
	let window_size_buf = create_buffer(&device, window_size);

	let zoom = Zoom::default();
	let zoom_buf = create_buffer(&device, zoom);

	let pos = Position::default();
	let position_buf = create_buffer(&device, pos);

	let iterations = Iterations::default();
	let iterations_buf = create_buffer(&device, iterations);

	let julia = Julia { is_julia };
	let julia_buf = create_buffer(&device, julia);

	let vertices_data = vertices;

	let generator = Position { pos: [size.width as f32/2f32, size.width as f32/2f32]};
	let generator_buf = create_buffer(&device, generator);

	let vertex_buf = device.create_buffer_mapped(
		4,
		wgpu::BufferUsage::VERTEX
	).fill_from_slice(&vertices_data);

	let bind_group_layout =
		device.create_bind_group_layout(
			&wgpu::BindGroupLayoutDescriptor {
				bindings: &[
					wgpu::BindGroupLayoutBinding {
						binding: 0,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false
						}
					},
					wgpu::BindGroupLayoutBinding {
						binding: 1,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false
						}
					},
					wgpu::BindGroupLayoutBinding {
						binding: 2,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false
						}
					},
					wgpu::BindGroupLayoutBinding {
						binding: 3,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false
						}
					},
					wgpu::BindGroupLayoutBinding {
						binding: 4,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false
						}
					},
					wgpu::BindGroupLayoutBinding {
						binding: 5,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false
						}
					},
				]
			}
		);

	let bind_group = device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &bind_group_layout,
			bindings: &[
				wgpu::Binding {
					binding: 0,
					resource: wgpu::BindingResource::Buffer {
						buffer: &window_size_buf,
						range: 0..*WINDOW_SIZE_SIZE
					}
				},
				wgpu::Binding {
					binding: 1,
					resource: wgpu::BindingResource::Buffer {
						buffer: &zoom_buf,
						range: 0..*ZOOM_SIZE
					}
				},
				wgpu::Binding {
					binding: 2,
					resource: wgpu::BindingResource::Buffer {
						buffer: &position_buf,
						range: 0..*POSITION_SIZE
					}
				},
				wgpu::Binding {
					binding: 3,
					resource: wgpu::BindingResource::Buffer {
						buffer: &iterations_buf,
						range: 0..*ITERATIONS_SIZE
					}
				},
				wgpu::Binding {
					binding: 4,
					resource: wgpu::BindingResource::Buffer {
						buffer: &julia_buf,
						range: 0..*JULIA_SIZE
					}
				},
				wgpu::Binding {
					binding: 5,
					resource: wgpu::BindingResource::Buffer {
						buffer: &generator_buf,
						range: 0..*POSITION_SIZE
					}
				},
			],
		});

	let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		bind_group_layouts: &[&bind_group_layout],
	});

	let vs_module =
		device.create_shader_module(&*VERT_SHADER);

	let fs = &*FRAG_SHADER_INIT;
	let fs_module = device.create_shader_module(&fs);

	log::info!("Creating render pipeline");
	let render_pipeline = device.create_render_pipeline(
		&wgpu::RenderPipelineDescriptor {
			layout: &pipeline_layout,
			vertex_stage: wgpu::ProgrammableStageDescriptor {
				module: &vs_module,
				entry_point: "main",
			},
			fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
				module: &fs_module,
				entry_point: "main",
			}),
			rasterization_state: Some(wgpu::RasterizationStateDescriptor {
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: wgpu::CullMode::None,
				depth_bias: 0,
				depth_bias_slope_scale: 0.0,
				depth_bias_clamp: 0.0,
			}),
			primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
			color_states: &[wgpu::ColorStateDescriptor {
				format: wgpu::TextureFormat::Bgra8UnormSrgb,
				color_blend: wgpu::BlendDescriptor::REPLACE,
				alpha_blend: wgpu::BlendDescriptor::REPLACE,
				write_mask: wgpu::ColorWrite::ALL,
			}],
			depth_stencil_state: None,
			index_format: wgpu::IndexFormat::Uint32,
			vertex_buffers: &[wgpu::VertexBufferDescriptor {
				stride: *VERTEX_SIZE,
				step_mode: wgpu::InputStepMode::Vertex,
				attributes: &[wgpu::VertexAttributeDescriptor {
					format: wgpu::VertexFormat::Float2,
					offset: 0,
					shader_location: 0,
				}],
			}],
			sample_count: 1,
			sample_mask: !0,
			alpha_to_coverage_enabled: false,
		}
	);

	FractalViewData {
			bufs: Buffers {
				window_size: window_size_buf,
				position: position_buf,
				zoom: zoom_buf,
				iterations: iterations_buf,
				vertex: vertex_buf,
				julia: julia_buf,
				generator: generator_buf
			},
			vs_module: Arc::new(vs_module),
			pipeline_layout: Arc::new(pipeline_layout),
			frag_shader_module: Arc::new(Mutex::new(fs_module)),
			render_pipeline: Arc::new(Mutex::new(render_pipeline)),
			bind_group: Arc::new(Mutex::new(bind_group)),
			pos,
			prev_position: Position::default(),
			first_drag_pos_received: false,
			left_button_pressed: false,
			zoom,
			iterations: Iterations::default(),
	}
}