extern crate winit;
extern crate wgpu;
extern crate env_logger;
extern crate log;

use winit::{
	event,
	event_loop::{ControlFlow, EventLoop},
};


fn main() {
	env_logger::init();
	let event_loop = EventLoop::new();

	#[cfg(not(feature = "gl"))]
		let (_window, hidpi_factor, size, surface) = {
		let window = winit::window::Window::new(&event_loop).unwrap();
		let hidpi_factor = window.hidpi_factor();
		let size = window.inner_size().to_physical(hidpi_factor);

		let surface = wgpu::Surface::create(&window);
		(window, hidpi_factor, size, surface)
	};

	#[cfg(feature = "gl")]
		let (_window, hidpi_factor, instance, size, surface) = {
		let wb = winit::WindowBuilder::new();
		let cb = wgpu::glutin::ContextBuilder::new().with_vsync(true);
		let context = cb.build_windowed(wb, &event_loop).unwrap();

		let hidpi_factor = context.window().get_hidpi_factor();
		let size = context
			.window()
			.get_inner_size()
			.unwrap()
			.to_physical(hidpi_factor);

		let (context, window) = unsafe { context.make_current().unwrap().split() };

		let instance = wgpu::Instance::new(context);
		let surface = instance.get_surface();

		(window, hidpi_factor, instance, size, surface)
	};

	let adapter = wgpu::Adapter::request(
		&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::Default,
			backends: wgpu::BackendBit::PRIMARY,
		},
	).unwrap();

	let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
		extensions: wgpu::Extensions {
			anisotropic_filtering: false,
		},
		limits: wgpu::Limits::default(),
	});

	let vs = wgpu::read_spirv(
		glsl_to_spirv::compile(
			include_str!("shader.vert"),
			glsl_to_spirv::ShaderType::Vertex
		).unwrap()
	).unwrap();

	let vs_module =
		device.create_shader_module(&vs);

	let fs = wgpu::read_spirv(
		glsl_to_spirv::compile(
			include_str!("shader.frag"),
			glsl_to_spirv::ShaderType::Fragment
		).unwrap()
	).unwrap();

	let fs_module =
		device.create_shader_module(&fs);

	//	let size_slice = &[size.width, size.height];
	//	const SIZE_SLICE_LEN: usize = 2;
	//	let window_size_size
	//: wgpu::BufferAddress = std::mem::size_of_val(size_slice) as wgpu::BufferAddress;
	#[repr(C)]
	#[derive(Clone, Copy)]
	struct WindowSize {
		pos: [f32; 2]
	}

	let window_size = WindowSize {
		pos: [size.width as f32, size.height as f32]
	};

	let window_size_size = std::mem::size_of::<WindowSize>() as wgpu::BufferAddress;

	log::info!("Window size array is: {}", window_size_size);

	let uniform_buf = device.create_buffer_mapped(
		1,
		wgpu::BufferUsage::UNIFORM
			| wgpu::BufferUsage::COPY_DST
	).fill_from_slice(&[window_size]);

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
					}
				]
			}
		);

	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &bind_group_layout,
		bindings: &[
			wgpu::Binding {
				binding: 0,
				resource: wgpu::BindingResource::Buffer {
					buffer: &uniform_buf,
					range: 0..window_size_size
				}
			}
		],
	});
	let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		bind_group_layouts: &[&bind_group_layout],
	});

	let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
	});

	let mut sc_desc = wgpu::SwapChainDescriptor {
		usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
		format: wgpu::TextureFormat::Bgra8UnormSrgb,
		width: size.width.round() as u32,
		height: size.height.round() as u32,
		present_mode: wgpu::PresentMode::Vsync,
	};

	let mut swap_chain = device.create_swap_chain(
		&surface,
		&sc_desc
	);

	event_loop.run(move |event, _, control_flow| {
		*control_flow = if cfg!(feature = "metal-auto-capture") {
			ControlFlow::Exit
		} else {
			ControlFlow::Poll
		};
		match event {
			event::Event::WindowEvent { event, .. } => match event {
				event::WindowEvent::Resized(size) => {
					let physical = size.to_physical(hidpi_factor);
					log::info!("Resizing to {:?}", physical);
					if physical.width == 0. || physical.height == 0. {
						return;
					}
					sc_desc.width = physical.width.round() as u32;
					sc_desc.height = physical.height.round() as u32;
					swap_chain = device.create_swap_chain(&surface, &sc_desc);

					let window_size = WindowSize {
						pos: [physical.width as f32, physical.height as f32]
					};

					let temp_buf = device.create_buffer_mapped(
						1,
						wgpu::BufferUsage::COPY_SRC
					).fill_from_slice(&[window_size]);

					let mut encoder =
						device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

					encoder.copy_buffer_to_buffer(
						&temp_buf,
						0,
						&uniform_buf,
						0,
						window_size_size
					);

					let command_buf = encoder.finish();
					queue.submit(&[command_buf]);
				}
				event::WindowEvent::KeyboardInput {
					input:
					event::KeyboardInput {
						virtual_keycode: Some(event::VirtualKeyCode::Escape),
						state: event::ElementState::Pressed,
						..
					},
					..
				}
				| event::WindowEvent::CloseRequested => {
					*control_flow = ControlFlow::Exit;
				}
				_ => {}
			},
			event::Event::EventsCleared => {
				let frame = swap_chain
					.get_next_texture();
				let mut encoder =
					device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
				{
					let mut rpass = encoder.begin_render_pass(
						&wgpu::RenderPassDescriptor {
							color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
								attachment: &frame.view,
								resolve_target: None,
								load_op: wgpu::LoadOp::Clear,
								store_op: wgpu::StoreOp::Store,
								clear_color: wgpu::Color::GREEN,
							}],
							depth_stencil_attachment: None,
						}
					);
					rpass.set_pipeline(&render_pipeline);
					rpass.set_bind_group(0, &bind_group, &[]);
					rpass.draw(0..4, 0..1);
				}

				queue.submit(&[encoder.finish()]);
			}
			_ => (),
		}
	});
}