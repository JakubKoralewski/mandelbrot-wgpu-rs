use crate::views::prelude::*;
use crate::views::utils::new;
use wgpu::{Device, SwapChainOutput, CommandBuffer};
use winit::event::{MouseButton, ElementState};
use winit::dpi::{LogicalSize, PhysicalSize};

pub struct MandelbrotViewManager {
	view: MandelbrotOnlyView
}

impl FractalViewManager for MandelbrotViewManager {
	fn new(device: &Device, size: LogicalSize) -> Self {
		Self {
			view: MandelbrotOnlyView::new(device, size)
		}
	}

	fn render(&mut self, device: &Arc<Mutex<Device>>, frame: &SwapChainOutput) -> Vec<CommandBuffer> {
		vec![self.view.render(device, frame)]
	}

	fn resized(&mut self, device: &Arc<Mutex<Device>>, window_size: &WindowSize) -> Vec<CommandBuffer> {
		vec![self.view.resized(device, window_size)]
	}

	fn mouse_input(&mut self, button: MouseButton, state: ElementState) {
		self.view.mouse_input(button, state)
	}

	fn iterations(&mut self, device: &Arc<Mutex<Device>>, y_delta: f32) -> Vec<CommandBuffer> {
		vec![self.view.iterations(device, y_delta)]
	}

	fn set_julia(&mut self, device: &Arc<Mutex<Device>>, state: bool) -> Option<Vec<CommandBuffer>> {
		Some(vec![self.view.set_julia(device, state)])
	}

	fn zoom(&mut self, device: &Arc<Mutex<Device>>, y_delta: f32) -> Vec<CommandBuffer> {
		vec![self.view.zoom(device, y_delta)]
	}

	fn new_position(&mut self, device: &Arc<Mutex<Device>>, x: f32, y: f32, active: bool) -> Option<Vec<CommandBuffer>> {
		if let Some(pos) = self.view.new_position(device, x, y, active) {
			Some(vec![pos])
		} else {
			None
		}
	}

	fn create_render_pipeline(&mut self, device: &Device) {
		self.view.create_render_pipeline(device)
	}

	fn reload_fs(&mut self, device: &Arc<Mutex<Device>>) {
		self.view.reload_fs(device)
	}
}

struct MandelbrotOnlyView {
	data: FractalViewData,
}

impl FractalViewable for MandelbrotOnlyView {

	fn new(device: &wgpu::Device, size: dpi::LogicalSize) -> Self {
		let data
			= new(device, size, false, (*WHOLE_VERTICES).clone());

		Self {
			data,
		}
	}

	fn data(&mut self) -> &mut FractalViewData {
		&mut self.data
	}

	fn frag_shader_path(&self) -> &'static Path {
		&*FRAG_SHADER_PATH
	}

}
