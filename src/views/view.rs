use std::path::{PathBuf, Path};
use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

use crate::utils::{
	AtomicDevice, AtomicWindow,
	Position, POSITION_SIZE,
	Zoom, ZOOM_SIZE,
	WindowSize, WINDOW_SIZE_SIZE
};

use super::utils::{ZOOM_SENSITIVITY};
use std::ops::Deref;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::Ordering;
use notify::RecommendedWatcher;

pub struct Buffers {
	pub window_size: wgpu::Buffer,
	pub position: wgpu::Buffer,
	pub zoom: wgpu::Buffer,
}

pub struct FractalViewData {
	//	pub frag_file_change_receiver: mpsc::Receiver<notify::DebouncedEvent>,
	pub frag_shader_module: Arc<Mutex<wgpu::ShaderModule>>,
	pub render_pipeline: Arc<Mutex<wgpu::RenderPipeline>>,
	pub bind_group: Arc<Mutex<wgpu::BindGroup>>,
	pub bufs: Buffers,
	pub vs_module: Arc<wgpu::ShaderModule>,
	pub pipeline_layout: Arc<wgpu::PipelineLayout>,

	//	is_left_button_pressed: AtomicBool,
//	is_cursor_on_screen: AtomicBool,
	pub prev_position: Position,
	pub pos: Position,
	pub first_drag_pos_received: bool,
	pub left_button_pressed: bool,
	pub zoom: Zoom,
}

impl FractalViewData {
	fn set_fs(&mut self, sm: wgpu::ShaderModule) {
		self.frag_shader_module = Arc::new(Mutex::new(sm));
	}
}

pub trait FractalViewable {
	fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize) -> (RecommendedWatcher, mpsc::Receiver<notify::DebouncedEvent>, Self);

	fn data(&mut self) -> &mut FractalViewData;

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
						load_op: wgpu::LoadOp::Clear,
						store_op: wgpu::StoreOp::Store,
						clear_color: wgpu::Color::BLACK,
					}],
					depth_stencil_attachment: None,
				}
			);
			rpass.set_pipeline(self.data().render_pipeline.lock().unwrap().deref());
			rpass.set_bind_group(0, self.data().bind_group.lock().unwrap().deref(), &[]);
			rpass.draw(0..4, 0..1);
		}

		encoder.finish()
	}

	fn load_fs(path: &Path) -> Option<Vec<u32>> {
		log::info!("Loading fragment shader {:?}", path);
//		let mut buffer = String::new();
//		let mut f = File::open(path).unwrap();
//		f.read_to_string(&mut buffer).unwrap();
		let buffer = std::fs::read_to_string(
			path
		).unwrap();

//		log::info!("Reading:\n{:?}", buffer);
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

	fn new_position(&mut self, device: &AtomicDevice, x: f64, y: f64, active: bool) -> wgpu::CommandBuffer {
		let x = x as f32;
		let y = y as f32;
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
//		log::info!("Deltas, x: {:?}; y: {:?}", delta_x, delta_y);

			pos.pos[0] += delta_x * zoom.zoom;
			pos.pos[1] += delta_y * zoom.zoom;
			log::info!("New position: {:?}", pos);
		}
		prev_position.pos = [x, y];

		let temp_buf = device.lock().unwrap().create_buffer_mapped(
			1,
			wgpu::BufferUsage::COPY_SRC
		).fill_from_slice(&[pos]);

		self.data().pos = pos;
		self.data().prev_position = prev_position;

		let mut encoder =
			device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

		encoder.copy_buffer_to_buffer(
			&temp_buf,
			0,
			&self.data().bufs.position,
			0,
			*POSITION_SIZE
		);

		encoder.finish()
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
							vertex_buffers: &[],
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
