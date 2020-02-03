//! `SwitchableViewManager` owns all view managers, is a view manager itself,
//! and passes events to the manager dependent on its `current` property.

use super::prelude::*;
use super::{FractalViewManager, MandelbrotViewManager, DoubleViewManager};
use wgpu::{Device, SwapChainOutput, CommandBuffer};
use winit::event::{MouseButton, ElementState};
use winit::dpi::{PhysicalSize, LogicalSize};
use crate::utils::CurrentView;


pub struct SwitchableViewManager {
	single: Arc<Mutex<MandelbrotViewManager>>,
	double: Arc<Mutex<DoubleViewManager>>,
	pub current: CurrentView
}

impl SwitchableViewManager {
	pub fn init(single: Arc<Mutex<MandelbrotViewManager>>, double: Arc<Mutex<DoubleViewManager>>, current: CurrentView) -> Self {
		Self {
			single,
			double,
			current
		}
	}
}

impl FractalViewManager for SwitchableViewManager where {
	fn new(_device: &Device, _size: LogicalSize) -> Self {
		unimplemented!()
	}

	fn render(&mut self, device: &Arc<Mutex<Device>>, frame: &SwapChainOutput) -> Vec<CommandBuffer> {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().render(device, frame)
		} else {
			self.single.lock().unwrap().render(device, frame)
		}
	}

	fn resized(&mut self, device: &Arc<Mutex<Device>>, window_size: &WindowSize) -> Vec<CommandBuffer> {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().resized(device, window_size)
		} else {
			self.single.lock().unwrap().resized(device, window_size)
		}
	}

	fn mouse_input(&mut self, button: MouseButton, state: ElementState) {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().mouse_input(button, state)
		} else {
			self.single.lock().unwrap().mouse_input(button, state)
		}
	}

	fn iterations(&mut self, device: &Arc<Mutex<Device>>, y_delta: f32) -> Vec<CommandBuffer> {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().iterations(device, y_delta)
		} else {
			self.single.lock().unwrap().iterations(device, y_delta)
		}
	}

	fn set_julia(&mut self, device: &Arc<Mutex<Device>>, state: bool) -> Option<Vec<CommandBuffer>> {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().set_julia(device, state)
		} else {
			self.single.lock().unwrap().set_julia(device, state)
		}
	}

	fn zoom(&mut self, device: &Arc<Mutex<Device>>, y_delta: f32) -> Vec<CommandBuffer> {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().zoom(device, y_delta)
		} else {
			self.single.lock().unwrap().zoom(device, y_delta)
		}
	}

	fn new_position(&mut self, device: &Arc<Mutex<Device>>, x: f32, y: f32, active: bool) -> Option<Vec<CommandBuffer>> {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().new_position(device, x, y, active)
		} else {
			self.single.lock().unwrap().new_position(device, x, y, active)
		}
	}

	fn create_render_pipeline(&mut self, device: &Device) {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().create_render_pipeline(device)
		} else {
			self.single.lock().unwrap().create_render_pipeline(device)
		}
	}

	fn reload_fs(&mut self, device: &Arc<Mutex<Device>>) {
		if self.current == CurrentView::Double {
			self.double.lock().unwrap().reload_fs(device)
		} else {
			self.single.lock().unwrap().reload_fs(device)
		}
	}
}


