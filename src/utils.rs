use std::path::PathBuf;
use std::time::Instant;
use wgpu_glyph::{Section, Scale};
use std::sync::{mpsc, Arc, Mutex};

lazy_static! {
	pub(crate) static ref ABSOLUTE_PATH: PathBuf = std::env::current_dir().unwrap();
	pub(crate) static ref WINDOW_SIZE_SIZE: wgpu::BufferAddress = std::mem::size_of::<WindowSize>() as wgpu::BufferAddress;
	pub(crate) static ref ZOOM_SIZE: wgpu::BufferAddress = std::mem::size_of::<Zoom>() as wgpu::BufferAddress;
	pub(crate) static ref POSITION_SIZE: wgpu::BufferAddress = std::mem::size_of::<Position>() as wgpu::BufferAddress;
	pub(crate) static ref ITERATIONS_SIZE: wgpu::BufferAddress = std::mem::size_of::<Iterations>() as wgpu::BufferAddress;
}

pub(crate) type AtomicDevice = Arc<Mutex<wgpu::Device>>;
//pub(crate) type AtomicReceiver = Arc<Mutex<mpsc::Receiver<notify::DebouncedEvent>>>;
pub(crate) type AtomicWindow = Arc<Mutex<winit::window::Window>>;


#[repr(C)]
#[derive(Clone, Copy)]
pub struct WindowSize {
	pub size: [f32; 2]
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Zoom {
	pub zoom: f32
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Iterations {
	pub iterations: f32
}

impl Default for Iterations {
	fn default() -> Self {
		Self {
			iterations: 100.0
		}
	}
}

impl Default for Zoom {
	fn default() -> Self {
		Self {
			zoom: 0.003
		}
	}
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
	pub pos: [f32; 2]
}

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

		count + 1
	}
}

pub struct Changed(pub bool);

impl Changed {
	pub fn set(&mut self, state: bool, desc: &str) {
		log::info!("Changed to {:?} from {:?}", state, desc);
		self.0 = state;
	}
}

pub fn fps_command(
	device: &AtomicDevice,
	glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
	size: &winit::dpi::PhysicalSize,
	frame: &wgpu::SwapChainOutput,
	past: &mut Instant
) -> wgpu::CommandBuffer {
	let mut encoder =
		device.lock().unwrap().create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
	let now = Instant::now();
	let time = now - *past;
	*past = now;
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
		&mut encoder,
		&frame.view,
		size.width.round() as u32,
		size.height.round() as u32,
	).expect("error drawing text");

	encoder.finish()
}
