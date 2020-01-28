extern crate winit;
extern crate wgpu;
extern crate env_logger;
extern crate log;
#[macro_use]
extern crate lazy_static;
extern crate wgpu_glyph;
extern crate notify;
extern crate byteorder;
extern crate glsl_to_spirv;
extern crate zerocopy;

use winit::{
	event::{self, VirtualKeyCode},
	event_loop::{ControlFlow, EventLoop},
	window::Fullscreen
};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::path::PathBuf;
use std::time::Instant;
use std::thread;

mod views;

use crate::views::{MandelbrotViewManager, DoubleViewManager, SwitchableViewManager, FractalViewManager, FRAG_SHADER_PATH};

pub mod utils;

use crate::utils::{ABSOLUTE_PATH, WindowSize, Changed, create_watcher, CurrentView};
use utils::fps_command;
use wgpu::CommandBuffer;
use std::sync::atomic::{AtomicBool, Ordering};

pub const TITLE: &str = "Ah shit here we go again";
pub const ZOOM_SENSITIVITY: f32 = 0.9;

lazy_static! {
	static ref ICON: winit::window::Icon = {
		use std::io::Read;
		let mut icon_file_path = ABSOLUTE_PATH.clone();
		icon_file_path.push::<PathBuf>(["res", "gta_sa_icon"].iter().collect());

		let mut icon_file = File::open(icon_file_path).unwrap();
		let mut raw_pixels = Vec::new();
		icon_file.read_to_end(&mut raw_pixels).unwrap();
			winit::window::Icon::from_rgba(raw_pixels, 256, 256)
				.expect("Error creating Icon in winit")
	};
}

fn main() {
	env_logger::init();

	let event_loop = EventLoop::new();

	let icon: winit::window::Icon = (*ICON).clone();

	let init_window = |window: &winit::window::Window| {
		window.set_title("Initializing Vulkan...");
		window.set_window_icon(Some(icon));
	};

	#[cfg(not(feature = "gl"))]
	let (window, mut hidpi_factor, lsize, psize, surface) = {
		let window = winit::window::Window::new(&event_loop).unwrap();
		init_window(&window);
		let hidpi_factor = window.hidpi_factor();
		let lsize = window.inner_size();
		let psize = lsize.to_physical(hidpi_factor);

		let surface = wgpu::Surface::create(&window);
		(window, hidpi_factor, lsize, psize, surface)
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

	let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
	let mut sc_desc = wgpu::SwapChainDescriptor {
		usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
		format: render_format,
		width: psize.width.round() as u32,
		height: psize.height.round() as u32,
		present_mode: wgpu::PresentMode::Vsync,
	};
	let (_watcher, frag_file_change_receiver) = create_watcher(&*FRAG_SHADER_PATH);

	let single_view = MandelbrotViewManager::new(&device, lsize.clone());
	let double_view = DoubleViewManager::new(&device, lsize.clone());

	let swap_chain = device.create_swap_chain(
		&surface,
		&sc_desc
	);

	let mut fps_glyph_brush = {
		let font: &[u8] = include_bytes!("../fonts/impact.ttf");
		wgpu_glyph::GlyphBrushBuilder::using_font_bytes(font)
			.build(&mut device, render_format)
	};

	let mut is_left_button_pressed = false;
	let mut is_cursor_on_screen = false;

	let mut window_size = WindowSize {
		size: [lsize.width as f32, lsize.height as f32]
	};

	window.set_title(TITLE);
	let mut past = Instant::now();
	let mut is_full_screen = false;

	let single_view = Arc::new(Mutex::new(single_view));
	let double_view = Arc::new(Mutex::new(double_view));
	let current_view = Arc::new(Mutex::new(SwitchableViewManager::init(
		single_view,
		double_view,
		CurrentView::Single
	)));

	let device = Arc::new(Mutex::new(device));
	let swap_chain = Arc::new(Mutex::new(swap_chain));
	let queue = Arc::new(Mutex::new(queue));
	let window: Arc<Mutex<winit::window::Window>> = Arc::new(Mutex::new(window));
	let changed = Arc::new(Mutex::new(Changed { 0: true }));
	let please_set_title_back = Arc::new(AtomicBool::new(false));

	{
		let device = Arc::clone(&device);
		let window = Arc::clone(&window);
		let view = Arc::clone(&current_view);
		let changed = Arc::clone(&changed);
		let please_set_title_back = Arc::clone(&please_set_title_back);

		thread::spawn(move || {
			log::info!("Shader watcher thread spawned");
			loop {
				if let Ok(notify::DebouncedEvent::Write(..)) = frag_file_change_receiver.recv() {
					log::info!("Write event in fragment shader");
//					let window = window.lock().unwrap();
					window.lock().unwrap().set_title("Loading fragment shader...");
					view.lock().unwrap().reload_fs(&device);
					changed.lock().unwrap().set(true, "Write to shader");
					please_set_title_back.store(true, Ordering::SeqCst);
					log::info!("Requesting redraw");
					window.lock().unwrap().request_redraw();
				}
			}
		});
	}
//	let mut render = {
//		let device = Arc::clone(&device);
//		let window = Arc::clone(&window);
//		let view = Arc::clone(&single_view);
//		let changed = Arc::clone(&changed);
//		let please_set_title_back = Arc::clone(&please_set_title_back);
//		let swap_chain = Arc::clone(&swap_chain);
//		let queue = Arc::clone(&queue);
//		let is_double = Arc::clone(&is_double);
//		move || {
//			log::info!("Redraw requested");
//			let mut bufs: Vec<CommandBuffer>;
//			if please_set_title_back.load(Ordering::SeqCst) {
//				log::info!("Setting title back");
//				window.lock().unwrap().set_title(TITLE);
//				please_set_title_back.store(false, Ordering::SeqCst);
//			}
//			let mut swap_chain = swap_chain.lock().unwrap();
//			let frame = swap_chain.get_next_texture();
//			if is_double.load(Ordering::SeqCst) {
//				log::info!("Double rendering");
//				let buf1 = left_view.lock().unwrap().render(
//					&device, &frame
//				);
//				let buf2 = right_view.lock().unwrap().render(
//					&device, &frame
//				);
//				bufs = vec![buf1, buf2];
//			} else {
//				log::info!("Rendering single");
//				let render_buf = view.lock().unwrap().render(
//					&device, &frame
//				);
//				bufs = vec![render_buf];
//			}
//
//			let fps_buf = fps_command(
//				&device,
//				&mut fps_glyph_brush,
//				&size,
//				&frame,
//				&mut past
//			);
//			bufs.push(fps_buf);
//			queue.lock().unwrap().submit(&bufs);
//			changed.lock().unwrap().set(false, "Just rendered so false.");
//		}
//	};

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
					log::info!("Redraw requested");
					if please_set_title_back.load(Ordering::SeqCst) {
						log::info!("Setting title back");
						window.lock().unwrap().set_title(TITLE);
						please_set_title_back.store(false, Ordering::SeqCst);
					}
					let mut swap_chain = swap_chain.lock().unwrap();
					let frame = swap_chain.get_next_texture();
					let bufs = current_view.lock().unwrap().render(&device, &frame);

					let fps_buf = fps_command(
						&device,
						&mut fps_glyph_brush,
						&psize,
						&frame,
						&mut past
					);
					let mut queue = queue.lock().unwrap();
					queue.submit(&bufs);
					queue.submit(&[fps_buf]);
					changed.lock().unwrap().set(false, "Just rendered so false.");
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

					window_size.size = [size.width as f32, size.height as f32];

					let command_buf = current_view.lock().unwrap().resized(
						&device,
						&window_size
					);
					changed.lock().unwrap().set(true, "resize");
					queue.lock().unwrap().submit(&command_buf);
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
					log::info!("Cursor moved");
					let x = x as f32;
					let y = y as f32;
					let command_buf: Option<Vec<CommandBuffer>>;
					let mut current_view = current_view.lock().unwrap();
					if is_left_button_pressed && is_cursor_on_screen {
						log::info!("New active position: {:?}, {:?}", x, y);
						command_buf =
							current_view.new_position(&device, x, y, true);
						changed.lock().unwrap().set(true, "cursor moved");
					} else {
						log::info!("New passive position: {:?}, {:?}", x, y);
						command_buf =
							current_view.new_position(&device, x, y, false);

						if current_view.current == CurrentView::Double {
							changed.lock().unwrap().set(true, "Updated julia generator");
						}
					}
					if let Some(command_buf) = command_buf {
						queue.lock().unwrap().submit(&command_buf);
					}
				}
				event::WindowEvent::MouseInput {
					button,
					state,
					..
				} => {
					log::info!("Mouse input");
					if button == winit::event::MouseButton::Left {
						if state == winit::event::ElementState::Pressed {
							is_left_button_pressed = true;
						} else {
							is_left_button_pressed = false;
						}
					}
					current_view.lock().unwrap().mouse_input(button, state);
				}
				event::WindowEvent::MouseWheel {
					delta,
					modifiers,
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
					let command_buf: Vec<CommandBuffer>;
					if modifiers.alt {
						command_buf = current_view.lock().unwrap().iterations(&device, y_delta);
						changed.lock().unwrap().set(true, "iterations");
					} else {
// https://github.com/danyshaanan/mandelbrot/blob/master/docs/glsl/index.htm#L149
						command_buf = current_view.lock().unwrap().zoom(&device, y_delta);
						changed.lock().unwrap().set(true, "zoom");
					}

					queue.lock().unwrap().submit(&command_buf);
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
						VirtualKeyCode::F11 => {
							is_full_screen = !is_full_screen;
							let video_mode = window.lock().unwrap().current_monitor().video_modes().next().unwrap();
							if is_full_screen {
								window.lock().unwrap().set_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
							} else {
								window.lock().unwrap().set_fullscreen(None);
							}
						},
						keycode => {
							let mut command_buf: Option<Vec<CommandBuffer>> = None;
							let mut current_view = current_view.lock().unwrap();
							match keycode {
								VirtualKeyCode::Numpad1 | VirtualKeyCode::Key1 => {
									current_view.current = CurrentView::Single;
									let buf = current_view.set_julia(&device, false);
									command_buf = buf;
								}
								VirtualKeyCode::Numpad2 | VirtualKeyCode::Key2 => {
									current_view.current = CurrentView::Single;
									let buf = current_view.set_julia(&device, true);
									command_buf = buf;
								}
								VirtualKeyCode::Numpad3 | VirtualKeyCode::Key3 => {
									current_view.current = CurrentView::Double;
									window.lock().unwrap().request_redraw();
								}
								_ => ()
							};
							if let Some(cmd_buf) = command_buf {
								queue.lock().unwrap().submit(&cmd_buf);
								window.lock().unwrap().request_redraw();
							}
						}
					}
				}
				event::WindowEvent::HiDpiFactorChanged(hdpif) => {
					hidpi_factor = hdpif;
				}
				_ => {}
			},
			event::Event::EventsCleared => {
				if changed.lock().unwrap().0 {
					window.lock().unwrap().request_redraw();
				}
			}
			_ => (),
		}
	});
}