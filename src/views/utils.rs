use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

pub const ZOOM_SENSITIVITY: f32 = 0.9;

pub fn create_watcher(path: &PathBuf) -> (RecommendedWatcher, mpsc::Receiver<notify::DebouncedEvent>) {
	let (tx, rx) = mpsc::channel();
	let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(500)).unwrap();

	watcher.watch(path.clone(), RecursiveMode::NonRecursive).unwrap();
	log::info!("Starting watcher on {:?}", path);

	(watcher, rx)
}

pub fn create_buffer<T: 'static + Copy>(device: &wgpu::Device, value: T) -> wgpu::Buffer {
	device.create_buffer_mapped(
		1,
		wgpu::BufferUsage::UNIFORM
			| wgpu::BufferUsage::COPY_DST
	).fill_from_slice(&[value])
}

