use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::utils::{
	AtomicDevice,
	Position, POSITION_SIZE,
	Zoom, ZOOM_SIZE,
	WindowSize, WINDOW_SIZE_SIZE,
	Iterations, ITERATIONS_SIZE,
	VERTEX_SIZE,
	Julia, JULIA_SIZE
};

use super::utils::ZOOM_SENSITIVITY;
use std::ops::Deref;

pub struct Buffers {
	pub window_size: wgpu::Buffer,
	pub position: wgpu::Buffer,
	pub zoom: wgpu::Buffer,
	pub iterations: wgpu::Buffer,
	pub vertex: wgpu::Buffer,
	pub julia: wgpu::Buffer,
	pub generator: wgpu::Buffer,
}

pub struct FractalViewData {
	pub frag_shader_module: Arc<Mutex<wgpu::ShaderModule>>,
	pub render_pipeline: Arc<Mutex<wgpu::RenderPipeline>>,
	pub bind_group: Arc<Mutex<wgpu::BindGroup>>,
	pub bufs: Buffers,
	pub vs_module: Arc<wgpu::ShaderModule>,
	pub pipeline_layout: Arc<wgpu::PipelineLayout>,

	pub prev_position: Position,
	pub pos: Position,
	pub first_drag_pos_received: bool,
	pub left_button_pressed: bool,
	pub zoom: Zoom,
	pub iterations: Iterations,
}

impl FractalViewData {
	fn set_fs(&mut self, sm: wgpu::ShaderModule) {
		self.frag_shader_module = Arc::new(Mutex::new(sm));
	}
}

pub trait FractalViewManager {
	fn new(device: &wgpu::Device, size: winit::dpi::LogicalSize) -> Self;

	fn render(
		&mut self,
		device: &AtomicDevice,
		frame: &wgpu::SwapChainOutput,
	) -> Vec<wgpu::CommandBuffer>;

	fn resized(
		&mut self,
		device: &AtomicDevice,
		window_size: &WindowSize
	) -> Vec<wgpu::CommandBuffer>;

	fn load_fs(path: &Path) -> Option<Vec<u32>> {
		log::info!("Loading fragment shader {:?}", path);
		let buffer = std::fs::read_to_string(
			path
		).unwrap();

		let spirv = glsl_to_spirv::compile(
			&buffer,
			glsl_to_spirv::ShaderType::Fragment
		);
		match spirv {
			Ok(spirv) => {
				// Load fragment shader
				Some(wgpu::read_spirv(spirv).unwrap())
			}
			Err(err) => {
				log::error!("Spirv compilation error: {:?}", err);
				None
			}
		}
	}

	fn mouse_input(&mut self, button: winit::event::MouseButton, state: winit::event::ElementState);

	fn iterations(&mut self, device: &AtomicDevice, y_delta: f32) -> Vec<wgpu::CommandBuffer>;

	fn set_julia(&mut self, device: &AtomicDevice, state: bool) -> Option<Vec<wgpu::CommandBuffer>>;

	fn zoom(&mut self, device: &AtomicDevice, y_delta: f32) -> Vec<wgpu::CommandBuffer>;

	fn new_position(&mut self, device: &AtomicDevice, x: f32, y: f32, active: bool) -> Option<Vec<wgpu::CommandBuffer>>;

	fn create_render_pipeline(&mut self, device: &wgpu::Device);

	fn reload_fs(&mut self, device: &AtomicDevice);
}


pub trait FractalViewable {
	fn new(device: &wgpu::Device, size: winit::dpi::LogicalSize) -> Self;

	fn data(&mut self) -> &mut FractalViewData;

	fn render(
		&mut self,
		device: &AtomicDevice,
		frame: &wgpu::SwapChainOutput,
	) -> wgpu::CommandBuffer {
		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
		{
			let mut rpass = encoder.begin_render_pass(
				&wgpu::RenderPassDescriptor {
					color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
						attachment: &frame.view,
						resolve_target: None,
						load_op: wgpu::LoadOp::Load,
						store_op: wgpu::StoreOp::Store,
						clear_color: wgpu::Color::BLACK
					}],
					depth_stencil_attachment: None,
				}
			);
			rpass.set_pipeline(self.data().render_pipeline.lock().unwrap().deref());
			rpass.set_bind_group(0, self.data().bind_group.lock().unwrap().deref(), &[]);
			rpass.set_vertex_buffers(0, &[(&self.data().bufs.vertex, 0)]);
			rpass.draw(0..4, 0..1);
		}

		encoder.finish()
	}
	fn resized(
		&mut self,
		device: &AtomicDevice,
		window_size: &WindowSize
	) -> wgpu::CommandBuffer {
		let temp_buf = device.lock().unwrap().create_buffer_mapped(
			1,
			wgpu::BufferUsage::COPY_SRC
		).fill_from_slice(&[window_size.clone()]);

		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

		encoder.copy_buffer_to_buffer(
			&temp_buf,
			0,
			&self.data().bufs.window_size,
			0,
			*WINDOW_SIZE_SIZE
		);

		encoder.finish()
	}


//	fn render(
//		&mut self,
//		device: &AtomicDevice,
//		frame: &wgpu::SwapChainOutput,
//	) -> wgpu::CommandBuffer;

	fn load_fs(path: &Path) -> Option<Vec<u32>> {
		log::info!("Loading fragment shader {:?}", path);
		let buffer = std::fs::read_to_string(
			path
		).unwrap();

		let spirv = glsl_to_spirv::compile(
			&buffer,
			glsl_to_spirv::ShaderType::Fragment
		);
		match spirv {
			Ok(spirv) => {
				// Load fragment shader
				Some(wgpu::read_spirv(spirv).unwrap())
			}
			Err(err) => {
				log::error!("Spirv compilation error: {:?}", err);
				None
			}
		}
	}

	fn mouse_input(&mut self, button: winit::event::MouseButton, state: winit::event::ElementState) {
		use winit::event;
		if button != event::MouseButton::Left {
			return;
		}
		match state {
			event::ElementState::Pressed => {
				log::info!("Pressed left mouse button.");
				self.data().left_button_pressed = true;
			}
			event::ElementState::Released => {
				log::info!("Released left mouse button.");
				self.data().left_button_pressed = false;
			}
		}
	}

	fn iterations(&mut self, device: &AtomicDevice, y_delta: f32) -> wgpu::CommandBuffer {
		let mut iterations = self.data().iterations;

		iterations.iterations *= 0.99f32.powi(y_delta.signum() as i32);
		if iterations.iterations < 0.0 {
			iterations.iterations = 0.0;
		} else if iterations.iterations > 800.0 {
			iterations.iterations = 800.0;
		}
		log::info!("Iterations: {:#?}", iterations);
		self.data().iterations = iterations;

		let temp_buf = device.lock().unwrap().create_buffer_mapped(
			1,
			wgpu::BufferUsage::COPY_SRC
		).fill_from_slice(&[iterations]);

		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

		encoder.copy_buffer_to_buffer(
			&temp_buf,
			0,
			&self.data().bufs.iterations,
			0,
			*ITERATIONS_SIZE
		);

		encoder.finish()
	}

	fn set_julia(&mut self, device: &AtomicDevice, state: bool) -> wgpu::CommandBuffer {
		log::info!("Setting is_julia to: {:?}", state);
		let temp_buf = device.lock().unwrap().create_buffer_mapped(
			1,
			wgpu::BufferUsage::COPY_SRC
		).fill_from_slice(&[Julia{is_julia: state}]);

		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

		encoder.copy_buffer_to_buffer(
			&temp_buf,
			0,
			&self.data().bufs.julia,
			0,
			*JULIA_SIZE
		);

		encoder.finish()
	}

	fn zoom(&mut self, device: &AtomicDevice, y_delta: f32) -> wgpu::CommandBuffer {
		let mut zoom = self.data().zoom;
		zoom.zoom *= (ZOOM_SENSITIVITY as f32).powi(y_delta.signum() as i32);

		self.data().zoom = zoom;
		log::info!("Zoom now of value: {:?}", zoom.zoom);

		let temp_buf = device.lock().unwrap().create_buffer_mapped(
			1,
			wgpu::BufferUsage::COPY_SRC
		).fill_from_slice(&[zoom]);

		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

		encoder.copy_buffer_to_buffer(
			&temp_buf,
			0,
			&self.data().bufs.zoom,
			0,
			*ZOOM_SIZE
		);

		encoder.finish()
	}

	fn new_position(&mut self, device: &AtomicDevice, x: f32, y: f32, active: bool) -> Option<wgpu::CommandBuffer> {
		let mut prev_position = self.data().prev_position;
		let mut pos = self.data().pos;

		if !self.data().first_drag_pos_received {
			prev_position.pos = [x, y];
			self.data().first_drag_pos_received = true;
		}

		if active {
			log::info!("Initial: {:?} Current: {:?},{:?}", prev_position, x, y);
			let delta_x = x - prev_position.pos[0];
			let delta_y = y - prev_position.pos[1];

			let zoom = self.data().zoom;

			pos.pos[0] += delta_x * zoom.zoom;
			pos.pos[1] += delta_y * zoom.zoom;
			log::info!("New position: {:?}", pos);
		}
		prev_position.pos = [x, y];

		self.data().pos = pos;
		self.data().prev_position = prev_position;
		if !active {
			return None;
		}

		let temp_buf = device.lock().unwrap().create_buffer_mapped(
			1,
			wgpu::BufferUsage::COPY_SRC
		).fill_from_slice(&[pos]);


		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

		encoder.copy_buffer_to_buffer(
			&temp_buf,
			0,
			&self.data().bufs.position,
			0,
			*POSITION_SIZE
		);

		Some(encoder.finish())
	}

	fn frag_shader_path(&self) -> &'static Path;

	fn create_render_pipeline(&mut self, device: &wgpu::Device) {
		log::info!("Creating render pipeline");
		let pipeline_layout = Arc::clone(&self.data().pipeline_layout);
		let vs_module = Arc::clone(&self.data().vs_module);
		let fs_module = {
			let fs_module = self.data().frag_shader_module.lock().unwrap();
			Arc::new(
				Mutex::new(
					device.create_render_pipeline(
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
					)
				)
			)
		};
		self.data().render_pipeline = fs_module;
	}

	fn reload_fs(&mut self, device: &AtomicDevice) {
		if let Some(fs) = Self::load_fs(self.frag_shader_path()) {
			log::info!("Setting fs");
			self.data().set_fs(device.lock().unwrap().create_shader_module(&fs));
			self.create_render_pipeline(&device.lock().unwrap());
		} else {
			log::info!("Spirv compilation failed! Ignoring tho");
		}
	}
}
