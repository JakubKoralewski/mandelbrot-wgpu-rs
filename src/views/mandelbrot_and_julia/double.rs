use super::JuliaDoubleView;
use super::MandelbrotDoubleView;

use crate::views::prelude::*;
use wgpu::{Device, CommandBuffer};
use winit::event::{MouseButton, ElementState};

pub struct DoubleViewManager {
	left: MandelbrotDoubleView,
	right: JuliaDoubleView,
	window_size: WindowSize,
	cursor_pos: Position,
	prev_cursor_pos: Position,
	ever_had_pos: bool
}

impl FractalViewManager for DoubleViewManager {
	fn new(device: &wgpu::Device, size: winit::dpi::LogicalSize) -> Self {
		Self {
			left: MandelbrotDoubleView::new(device, size.clone()),
			right: JuliaDoubleView::new(device, size),
			window_size: WindowSize{size: [size.width as f32, size.height as f32]},
			cursor_pos: Position{pos: [size.width as f32/2f32, size.height as f32/2f32]},
			prev_cursor_pos: Position{pos: [-100f32, -100f32]},
			ever_had_pos: false,
		}
	}

	fn render(
		&mut self,
		device: &AtomicDevice,
		frame: &wgpu::SwapChainOutput,
	) -> Vec<CommandBuffer> {
		let buf1 = self.left.render(device, frame);
		let buf2 = self.right.render(device, frame);

		vec![buf1, buf2]
	}

	fn resized(&mut self, device: &AtomicDevice, window_size: &WindowSize) -> Vec<CommandBuffer> {
		self.window_size = window_size.to_owned();
		let buf1 = self.left.resized(device, window_size);
		let buf2 = self.right.resized(device, window_size);

		vec![buf1, buf2]
	}

	fn mouse_input(&mut self, button: MouseButton, state: ElementState) {
		self.left.mouse_input(button, state);
		self.right.mouse_input(button, state);
	}

	fn iterations(&mut self, device: &AtomicDevice, y_delta: f32) -> Vec<CommandBuffer> {
		if self.cursor_pos.pos[0] < self.window_size.size[0] / 2f32 {
			vec![self.left.iterations(device, y_delta)]
		} else {
			vec![self.right.iterations(device, y_delta)]
		}
	}

	fn set_julia(&mut self, _device: &Arc<Mutex<Device>>, _state: bool) -> Option<Vec<CommandBuffer>> {
		None
	}

	fn zoom(&mut self, device: &Arc<Mutex<Device>>, y_delta: f32) -> Vec<CommandBuffer> {
		if self.cursor_pos.pos[0] < self.window_size.size[0] / 2f32 {
			let buf1 = self.left.zoom(device, y_delta);
			vec![buf1]
		} else {
			let buf2 = self.right.zoom(device, y_delta);
			vec![buf2]
		}
	}

	fn new_position(&mut self, device: &Arc<Mutex<Device>>, x: f32, y: f32, active: bool) -> Option<Vec<CommandBuffer>> {
		let mut buf = vec![];
		self.cursor_pos.pos = [x, y];
		if x > self.window_size.size[0] / 2f32 {
			log::info!("Sending new_position to right.");
			if let Some(ok) = self.right.new_position(device, x, y, active) {
				buf.push(ok);
			}
		} else {
			log::info!("Sending new_position to left.");
			let mut pos = self.left.data().pos;
			if let Some(ok) = self.left.new_position(device, x,y, active) {
				buf.push(ok);
			}
			if !active {
//				let mut pos = self.left.data().pos;
				let zoom = self.left.data().zoom;
				if !self.ever_had_pos {
					self.prev_cursor_pos.pos = [x, y];
					self.ever_had_pos = true;
				}
				let mut prev_position = self.prev_cursor_pos;
				let delta_x = x - prev_position.pos[0];
				let delta_y = y - prev_position.pos[1];

				let zoom = self.left.data().zoom;

				pos.pos[0] += delta_x * zoom.zoom;
				pos.pos[1] += delta_y * zoom.zoom;

				log::info!("Sending cursor pos {:?} to Julia", pos);
				let temp_buf = device.lock().unwrap().create_buffer_mapped(
					1,
					wgpu::BufferUsage::COPY_SRC
				).fill_from_slice(&[pos]);

				let mut encoder =
					device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

				encoder.copy_buffer_to_buffer(
					&temp_buf,
					0,
					&self.right.data().bufs.generator,
					0,
					*POSITION_SIZE
				);

				buf.push(encoder.finish());
			}
		}
		if buf.len() > 0 {
			Some(buf)
		} else {
			None
		}
	}

	fn create_render_pipeline(&mut self, device: &Device) {
		self.left.create_render_pipeline(device);
		self.right.create_render_pipeline(device);
	}

	fn reload_fs(&mut self, device: &Arc<Mutex<Device>>) {
		self.left.reload_fs(device);
		self.right.reload_fs(device);
	}
}