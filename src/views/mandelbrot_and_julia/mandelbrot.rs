use crate::views::prelude::*;
use crate::views::utils::new;

pub struct MandelbrotDoubleView {
	data: FractalViewData,
}

impl FractalViewable for MandelbrotDoubleView {
	fn new(device: &wgpu::Device, size: dpi::LogicalSize) -> Self {
		let data
			= new(device, size, false, (*LEFT_HALF_VERTICES).clone());

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
