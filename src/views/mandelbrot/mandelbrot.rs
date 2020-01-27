use winit::dpi;
use std::path::{PathBuf, Path};
use std::sync::{Arc, Mutex, mpsc};

// spirv is littleendian I think
use byteorder::{ByteOrder, LittleEndian};

use crate::views::view::{FractalViewable, FractalViewData};

use crate::views::utils::{
	create_watcher,
	create_buffer
};

use crate::utils::{
	ABSOLUTE_PATH, Position, POSITION_SIZE,
	WindowSize, WINDOW_SIZE_SIZE,
	Zoom, ZOOM_SIZE,
	Iterations, ITERATIONS_SIZE
};
use views::view::Buffers;
use notify::{RecommendedWatcher, DebouncedEvent};

lazy_static! {
	/// Vertex shader compiled at build and loaded lazily
	static ref VERT_SHADER: Vec<u32> = {
		let bytes = include_bytes!("../../../shaders/full.vert.spv");
		let mut rs = vec![0; bytes.len()/4];
		LittleEndian::read_u32_into(bytes, &mut rs);
		log::info!("Read bytes len originally {:?}, to {:?}", bytes.len(), rs.len());
		rs
	};
	static ref FRAG_SHADER_INIT: Vec<u32> = {
		let bytes = include_bytes!("../../../shaders/mandelbrot.frag.spv");
		let mut rs = vec![0; bytes.len()/4];
		LittleEndian::read_u32_into(bytes, &mut rs);
		rs
	};
	static ref FRAG_SHADER_PATH: PathBuf = {
		let mut frag_shader_path_buf: PathBuf = ABSOLUTE_PATH.clone();
		let x = ["shaders", "mandelbrot.frag"].iter().collect();
		frag_shader_path_buf.push::<PathBuf>(x);

		log::info!("Frag shader path: {:?}", frag_shader_path_buf);
		frag_shader_path_buf
	};
}

pub struct MandelbrotOnlyView {
	data: FractalViewData,
}

impl FractalViewable for MandelbrotOnlyView {
	fn new(device: &wgpu::Device, size: dpi::PhysicalSize) -> (RecommendedWatcher, mpsc::Receiver<DebouncedEvent>, Self) {
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
						}
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
				vertex_buffers: &[],
				sample_count: 1,
				sample_mask: !0,
				alpha_to_coverage_enabled: false,
			}
		);

		let (watcher, rx) =
				create_watcher(
					&*FRAG_SHADER_PATH
				);

		(watcher, rx, Self {
			data: FractalViewData {
				bufs: Buffers {
					window_size: window_size_buf,
					position: position_buf,
					zoom: zoom_buf,
					iterations: iterations_buf
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
				iterations: Iterations::default()
			},
		})
	}

	fn data(&mut self) -> &mut FractalViewData {
		&mut self.data
	}

	fn frag_shader_path(&self) -> &'static Path {
		&*FRAG_SHADER_PATH
	}

}
