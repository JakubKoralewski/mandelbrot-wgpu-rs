//! Stuff that gets imported by a lot of other files.

pub use winit::dpi;
pub use std::path::{PathBuf, Path};
pub use std::sync::{Arc, Mutex, mpsc};

// spirv is littleendian
pub use byteorder::{ByteOrder, LittleEndian};

pub use crate::views::view::{FractalViewable, FractalViewData, FractalViewManager};

pub use crate::views::utils::{
	create_buffer,
	WHOLE_VERTICES, RIGHT_HALF_VERTICES, LEFT_HALF_VERTICES
};

pub use crate::utils::{
	AtomicDevice,
	ABSOLUTE_PATH, Position, POSITION_SIZE,
	WindowSize, WINDOW_SIZE_SIZE,
	Zoom, ZOOM_SIZE,
	Iterations, ITERATIONS_SIZE,
	Vertex, VERTEX_SIZE,
	Julia, JULIA_SIZE
};

pub use views::view::Buffers;
pub use notify::{RecommendedWatcher, DebouncedEvent};
pub use std::ops::Deref;

lazy_static! {
	/// Vertex shader compiled at build and loaded lazily
	pub static ref VERT_SHADER: Vec<u32> = {
		let bytes = include_bytes!("../../shaders/vertices.vert.spv");
		let mut rs = vec![0; bytes.len()/4];
		LittleEndian::read_u32_into(bytes, &mut rs);
		log::info!("Read bytes len originally {:?}, to {:?}", bytes.len(), rs.len());
		rs
	};
	/// Pre-compiled shader
	pub static ref FRAG_SHADER_INIT: Vec<u32> = {
		let bytes = include_bytes!("../../shaders/mandelbrot.frag.spv");
		let mut rs = vec![0; bytes.len()/4];
		LittleEndian::read_u32_into(bytes, &mut rs);
		rs
	};
	/// Path to shader file which gets reloaded in `main`.
	pub static ref FRAG_SHADER_PATH: PathBuf = {
		let mut frag_shader_path_buf: PathBuf = ABSOLUTE_PATH.clone();
		let x = ["shaders", "mandelbrot.frag"].iter().collect();
		frag_shader_path_buf.push::<PathBuf>(x);

		log::info!("Frag shader path: {:?}", frag_shader_path_buf);
		frag_shader_path_buf
	};

}
