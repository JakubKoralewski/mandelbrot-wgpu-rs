extern crate winit;
extern crate wgpu;
extern crate env_logger;
extern crate log;
extern crate image;
#[macro_use]
extern crate lazy_static;

use winit::{
	event::{self, VirtualKeyCode},
	event_loop::{ControlFlow, EventLoop},
	window::Fullscreen
};
use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::{mpsc, Arc, Mutex};
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::path::PathBuf;
use wgpu_glyph::{Section, GlyphBrushBuilder, Scale};
use std::time::Instant;
use std::thread;
use std::ops::DerefMut;

const ZOOM_SENSITIVITY: f32 = 0.9;
const TITLE: &str = "Ah shit here we go again";

trait DigitsCountable {
	fn count_digits(self) -> usize;
}

impl DigitsCountable for usize {
	fn count_digits(self) -> usize {
		let mut number = self;
		let mut count: usize = 0;

		while number > 0 {
			number /= 10;
			count += 1;
		}

		count
	}
}
lazy_static! {
	static ref ICON: winit::window::Icon = {
		let icon_image =
			image::load_from_memory_with_format(
				include_bytes!("../res/gta_sa.ico"),
				image::ImageFormat::ICO
			).expect("Error decoding icon");

		winit::window::Icon::from_rgba(icon_image.raw_pixels(), 256, 256)
			.expect("Error creating Icon in winit")
	};
	/// Load vertex shader
	static ref VS: Vec<u32> = wgpu::read_spirv(
		glsl_to_spirv::compile(
			include_str!("../shaders/shader.vert"),
			glsl_to_spirv::ShaderType::Vertex
		).unwrap()
	).unwrap();
	static ref ABSOLUTE_PATH: PathBuf = std::env::current_dir().unwrap();
	static ref FRAG_SHADER_PATH: PathBuf = {
		let mut frag_shader_path_buf: PathBuf = ABSOLUTE_PATH.clone();
		let x = ["shaders", "shader.frag"].iter().collect();
		frag_shader_path_buf.push::<PathBuf>(x);

		log::info!("Frag shader path: {:?}", frag_shader_path_buf);
		frag_shader_path_buf
	};
}

fn main() {
	env_logger::init();

let load_fs = move || -> Result<Vec<u32>, std::io::Error> {
	log::info!("Loading fragment shader");
	let mut buffer = String::new();
	let mut f = File::open(&*FRAG_SHADER_PATH)?;
	f.read_to_string(&mut buffer)?;

	// Load fragment shader
	wgpu::read_spirv(
		glsl_to_spirv::compile(
			&buffer,
			glsl_to_spirv::ShaderType::Fragment
		).expect("Compilation failed")
	)
};
	let fs = load_fs().expect("error loading fs");

	let event_loop = EventLoop::new();

	let icon: winit::window::Icon = (*ICON).clone();

	let init_window = |window: &winit::window::Window| {
		window.set_title("Initializing Vulkan...");
		window.set_window_icon(Some(icon));
	};

	#[cfg(not(feature = "gl"))]
		let (window, mut hidpi_factor, size, surface) = {
		let window = winit::window::Window::new(&event_loop).unwrap();
		init_window(&window);
		let hidpi_factor = window.hidpi_factor();
		let size = window.inner_size().to_physical(hidpi_factor);

		let surface = wgpu::Surface::create(&window);
		(window, hidpi_factor, size, surface)
	};

	#[cfg(feature = "gl")]
		let (window, hidpi_factor, instance, size, surface) = {
		init_window(&window);
		let wb = winit::WindowBuilder::new();
		let cb = wgpu::glutin::ContextBuilder::new().with_vsync(true);
		let context = cb.build_windowed(wb, &event_loop).unwrap();
		let hidpi_factor = context.window().get_hidpi_factor();
		let size = context
			.window()
			.get_inner_size()
			.unwrap()
			.to_physical(hidpi_factor);

		let (context, window) = unsafe {
			context.make_current().unwrap().split()
		};

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

	let (mut device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
		extensions: wgpu::Extensions {
			anisotropic_filtering: true,
		},
		limits: wgpu::Limits::default(),
	});

	let vs_module =
		device.create_shader_module(&*VS);

	let load_fs_module
		= move |device: &wgpu::Device, fs: &[u32]| device.create_shader_module(fs);

	let fs_module = load_fs_module(&device, &fs);

	let (tx, rx) = mpsc::channel();
	let mut watcher: RecommendedWatcher =
		Watcher::new(tx, Duration::from_millis(500)).unwrap();

	log::info!("Starting watcher on {:?}", *FRAG_SHADER_PATH);
	watcher.watch((*FRAG_SHADER_PATH).clone(), RecursiveMode::NonRecursive).unwrap();

	#[repr(C)]
	#[derive(Clone, Copy)]
	struct WindowSize {
		size: [f32; 2]
	}

	let mut window_size = WindowSize {
		size: [size.width as f32, size.height as f32]
	};

	let window_size_size = std::mem::size_of::<WindowSize>() as wgpu::BufferAddress;

	let window_size_buf = device.create_buffer_mapped(
		1,
		wgpu::BufferUsage::UNIFORM
			| wgpu::BufferUsage::COPY_DST
	).fill_from_slice(&[window_size]);

	#[repr(C)]
	#[derive(Clone, Copy)]
	struct Zoom {
		zoom: f32
	}

	let mut zoom = Zoom {
		zoom: 0.003
	};

	let zoom_size = std::mem::size_of_val(&zoom) as wgpu::BufferAddress;

	let zoom_buf = device.create_buffer_mapped(
		1,
		wgpu::BufferUsage::UNIFORM
			| wgpu::BufferUsage::COPY_DST
	).fill_from_slice(&[zoom]);

	#[repr(C)]
	#[derive(Debug, Clone, Copy)]
	struct Position {
		pos: [f32; 2]
	}

	let mut pos = Position {
		pos: [0.0, 0.0]
	};

	let pos_size = std::mem::size_of_val(&pos) as wgpu::BufferAddress;

	let position_buf = device.create_buffer_mapped(
		1,
		wgpu::BufferUsage::UNIFORM
			| wgpu::BufferUsage::COPY_DST
	).fill_from_slice(&[pos]);

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
					buffer: &window_size_buf,
					range: 0..window_size_size
				}
			},
			wgpu::Binding {
				binding: 1,
				resource: wgpu::BindingResource::Buffer {
					buffer: &zoom_buf,
					range: 0..zoom_size
				}
			},
			wgpu::Binding {
				binding: 2,
				resource: wgpu::BindingResource::Buffer {
					buffer: &position_buf,
					range: 0..pos_size
				}
			},
		],
	});

	let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		bind_group_layouts: &[&bind_group_layout],
	});

	let create_render_pipeline = move |device: &wgpu::Device, fs_module: &wgpu::ShaderModule| {
		log::info!("Creating render pipeline");
		device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
		})
	};

	let render_pipeline = create_render_pipeline(&device, &fs_module);
	let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
	let mut sc_desc = wgpu::SwapChainDescriptor {
		usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
		format: render_format,
		width: size.width.round() as u32,
		height: size.height.round() as u32,
		present_mode: wgpu::PresentMode::Vsync,
	};

	let swap_chain = device.create_swap_chain(
		&surface,
		&sc_desc
	);

	let font: &[u8] = include_bytes!("../fonts/impact.ttf");
	let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(font)
		.build(&mut device, render_format);

	let mut is_left_button_pressed = false;
	let mut is_cursor_on_screen = false;

	let mut prev_position = pos;
	let mut first_drag_pos_received = false;

	window.set_title(TITLE);
	let mut past = Instant::now();
	let mut is_full_screen = false;

	let rx = Arc::new(Mutex::new(rx));
	let window = Arc::new(Mutex::new(window));
	let fs = Arc::new(Mutex::new(fs));
	let fs_module = Arc::new(Mutex::new(fs_module));
	let render_pipeline = Arc::new(Mutex::new(render_pipeline));
	let device = Arc::new(Mutex::new(device));

	let load_fs_module
		= move |device: Arc<Mutex<wgpu::Device>>, fs: &[u32]| device.lock().unwrap().create_shader_module(fs);

	let create_render_pipeline = Arc::new(create_render_pipeline);

	let create_render_pipeline_multithreaded = move |device: Arc<Mutex<wgpu::Device>>,
	                                                 fs_module: Arc<Mutex<wgpu::ShaderModule>>| {
		let create_render_pipeline = create_render_pipeline.clone();
		create_render_pipeline(&device.lock().unwrap(), &fs_module.lock().unwrap())
	};
	let create_render_pipeline_multithreaded = Arc::new(create_render_pipeline_multithreaded);


	let bind_group = Arc::new(Mutex::new(bind_group));
	let swap_chain = Arc::new(Mutex::new(swap_chain));
	let queue = Arc::new(Mutex::new(queue));

	let render = {
		let bind_group = Arc::clone(&bind_group);
		let render_pipeline = Arc::clone(&render_pipeline);
		let swap_chain = Arc::clone(&swap_chain);
		let device = Arc::clone(&device);
		let queue = Arc::clone(&queue);

		Arc::new(Mutex::new(move || {
			let mut swap_chain = swap_chain.lock().unwrap();
			let frame = swap_chain
				.get_next_texture();
			let mut encoder1 =
				device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

			{
				let mut rpass = encoder1.begin_render_pass(
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
				rpass.set_pipeline(&render_pipeline.lock().unwrap());
				rpass.set_bind_group(0, &bind_group.lock().unwrap(), &[]);
				rpass.draw(0..4, 0..1);
			}
			let mut encoder2 =
				device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
			{
				let now = Instant::now();
				let time = now - past;
				past = now;
				let fps = (1.0 / time.as_secs_f32()).round() as usize;

				let number_section = Section {
					text: &format!("{}", fps),
					screen_position: (size.width as f32 / 100.0, size.height as f32 / 100.0),
					scale: Scale::uniform(32.0),
					color: [1.0f32, 1.0f32, 1.0f32, 1.0f32],
					..Section::default() // color, position, etc
				};

				let mut number_section_outline = number_section;
				number_section_outline.color = [0.0f32, 0.0f32, 0.0f32, 1.0f32];
				number_section_outline.scale = Scale::uniform(38.0);
				number_section_outline.screen_position.0 -= 3.0f32;
				number_section_outline.screen_position.1 -= 3.0f32;

				let fps_section = Section {
					text: "fps",
					screen_position: (size.width as f32 / 100.0 + 16.0 * fps.count_digits() as f32, size.height as f32 / 100.0),
					scale: Scale::uniform(32.0),
					color: [1.0f32, 1.0f32, 1.0f32, 1.0f32],
					..Section::default() // color, position, etc
				};

				let mut fps_section_outline = fps_section;
				fps_section_outline.color = [0.0f32, 0.0f32, 0.0f32, 1.0f32];
				fps_section_outline.scale = Scale::uniform(38.0);
				fps_section_outline.screen_position.0 -= 3.0f32;
				fps_section_outline.screen_position.1 -= 3.0f32;

				glyph_brush.queue(fps_section_outline);
				glyph_brush.queue(fps_section);
				glyph_brush.queue(number_section_outline);
				glyph_brush.queue(number_section);

				glyph_brush.draw_queued(
					&mut device.lock().unwrap(),
					&mut encoder2,
					&frame.view,
					size.width.round() as u32,
					size.height.round() as u32,
				).expect("error drawing text");
			}

			queue.lock().unwrap().submit(&[encoder1.finish(), encoder2.finish()]);
		}))
	};

	{
		let rx = Arc::clone(&rx);
		let fs = Arc::clone(&fs);
		let fs_module = Arc::clone(&fs_module);
		let render_pipeline = Arc::clone(&render_pipeline);
		let device = Arc::clone(&device);
		let window = Arc::clone(&window);
		let render = Arc::clone(&render);

		thread::spawn(move || {
			log::info!("Shader watcher thread spawned");
			loop {
				if let Ok(notify::DebouncedEvent::Write(..)) = rx.lock().unwrap().recv() {
					log::info!("Write event in fragment shader");
					window.lock().unwrap().set_title("Loading shader.frag...");
					*fs.lock().unwrap() = load_fs().unwrap();
					*fs_module.lock().unwrap() = load_fs_module(Arc::clone(&device), &Arc::clone(&fs).lock().unwrap());
					*render_pipeline.lock().unwrap() = create_render_pipeline_multithreaded(Arc::clone(&device), Arc::clone(&fs_module));
					render.lock().unwrap().deref_mut()();
					window.lock().unwrap().set_title(TITLE);
				};
			}
		});
	}
	{
		let render = Arc::clone(&render);
		event_loop.run(move |event, _, control_flow| {
			*control_flow = if cfg!(feature = "metal-auto-capture") {
				ControlFlow::Exit
			} else {
				ControlFlow::Poll
			};
			match event {
				event::Event::WindowEvent {
					event,
					..
				} => match event {
					event::WindowEvent::RedrawRequested => {
						render.lock().unwrap()();
					}
					event::WindowEvent::Resized(size) => {
						let physical = size.to_physical(hidpi_factor);
						log::info!("Resizing to {:?}", physical);
						if physical.width == 0. || physical.height == 0. {
							return;
						}
						sc_desc.width = physical.width.round() as u32;
						sc_desc.height = physical.height.round() as u32;
						*swap_chain.lock().unwrap() = device.lock().unwrap().create_swap_chain(&surface, &sc_desc);

						window_size.size = [physical.width as f32, physical.height as f32];

						let temp_buf = device.lock().unwrap().create_buffer_mapped(
							1,
							wgpu::BufferUsage::COPY_SRC
						).fill_from_slice(&[window_size]);

						let mut encoder =
							device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

						encoder.copy_buffer_to_buffer(
							&temp_buf,
							0,
							&window_size_buf,
							0,
							window_size_size
						);

						let command_buf = encoder.finish();
						queue.lock().unwrap().submit(&[command_buf]);
					}
					event::WindowEvent::CursorLeft { .. } => {
						log::info!("Cursor left screen");
						is_cursor_on_screen = false;
					}
					event::WindowEvent::CursorEntered { .. } => {
						log::info!("Cursor back on screen");
						is_cursor_on_screen = true;
					}
					event::WindowEvent::CursorMoved {
						position: winit::dpi::LogicalPosition {
							x, y
						},
						..
					} => {
						if is_left_button_pressed && is_cursor_on_screen {
							if !first_drag_pos_received {
								prev_position.pos = [x as f32, y as f32];
								first_drag_pos_received = true;
							}
							log::info!("Initial: {:?} Current: {:?},{:?}", prev_position, x, y);
							let delta_x = x as f32 - prev_position.pos[0];
							let delta_y = y as f32 - prev_position.pos[1];

							prev_position.pos = [x as f32, y as f32];
							log::info!("Deltas, x: {:?}; y: {:?}", delta_x, delta_y);
							pos.pos[0] += delta_x * zoom.zoom;
							pos.pos[1] += delta_y * zoom.zoom;

							log::info!("New position: {:?}", pos);
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
							&position_buf,
							0,
							pos_size
						);

						let command_buf = encoder.finish();
						queue.lock().unwrap().submit(&[command_buf]);
					}
					event::WindowEvent::MouseInput {
						button,
						state,
						..
					} => {
						if button != event::MouseButton::Left {
							return;
						}
						match state {
							event::ElementState::Pressed => {
								log::info!("Pressed left mouse button.");
								is_left_button_pressed = true;
								first_drag_pos_received = false;
							}
							event::ElementState::Released => {
								log::info!("Released left mouse button.");
								is_left_button_pressed = false;
							}
						}
					}
					event::WindowEvent::MouseWheel {
						delta,
						..
					} => {
						let y_delta = {
							match delta {
								event::MouseScrollDelta::LineDelta(_, y) => {
									y
								}
								event::MouseScrollDelta::PixelDelta(pos) => {
									pos.y as f32
								}
							}
						};
						log::info!("MouseWheel moved delta: {:?}", y_delta);
						// https://github.com/danyshaanan/mandelbrot/blob/master/docs/glsl/index.htm#L149
						zoom.zoom *= (ZOOM_SENSITIVITY as f32).powi(y_delta.signum() as i32);

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
							&zoom_buf,
							0,
							zoom_size
						);

						let command_buf = encoder.finish();
						queue.lock().unwrap().submit(&[command_buf]);
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
					event::WindowEvent::KeyboardInput {
						input: event::KeyboardInput {
							virtual_keycode: Some(key),
							state: event::ElementState::Pressed,
							..
						},
						..
					} => {
						match key {
							VirtualKeyCode::F12 => {
								is_full_screen = !is_full_screen;
								let video_mode = window.lock().unwrap().current_monitor().video_modes().next().unwrap();
								if is_full_screen {
									window.lock().unwrap().set_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
								} else {
									window.lock().unwrap().set_fullscreen(None);
								}
							},
							VirtualKeyCode::Numpad1 | VirtualKeyCode::Key1 => {}
							_ => ()
						}
					}
					event::WindowEvent::HiDpiFactorChanged(hdpif) => {
						hidpi_factor = hdpif;
					}
					_ => {}
				},
				event::Event::EventsCleared => {
					window.lock().unwrap().request_redraw();
				}
				_ => (),
			}
		});
	}
}