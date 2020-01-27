extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate glsl_to_spirv;

use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{Write, Read};

lazy_static! {
	pub static ref ABSOLUTE_PATH: PathBuf = std::env::current_dir().unwrap();
}

fn create_gta_icon() {
	let mut icon_save_path = ABSOLUTE_PATH.clone();
	icon_save_path.push::<PathBuf>(["res", "gta_sa_icon"].iter().collect());
	if icon_save_path.exists() {
		return;
	}
	let icon: Vec<u8> = {
		let icon_image =
			image::load_from_memory_with_format(
				include_bytes!("../res/gta_sa.ico"),
				image::ImageFormat::ICO
			).expect("Error decoding icon");

		icon_image.raw_pixels()
	};

	let mut icon_save_file = File::create(icon_save_path).unwrap();
	icon_save_file.write(&icon).unwrap();
}

fn pre_compile_shaders() {
	println!("Precompiling shaders.");
	let mut shaders_path: PathBuf = ABSOLUTE_PATH.clone();
	shaders_path.push("shaders");
	if !shaders_path.is_dir() {
		panic!("Shaders path not a directory");
	}

	for file in fs::read_dir(shaders_path).unwrap() {
		let file = file.unwrap();
		let mut path = file.path();
		if let Some(os_ext) = path.extension() {
			let ext = os_ext.to_str().unwrap();
			let shader_type = match ext {
				"frag" => glsl_to_spirv::ShaderType::Fragment,
				"vert" => glsl_to_spirv::ShaderType::Vertex,
				_ => continue
			};

			let mut shader_text = String::new();
			let mut file = File::open(&path).unwrap();
			file.read_to_string(&mut shader_text).unwrap();

			let mut shader = glsl_to_spirv::compile(
				&shader_text,
				shader_type
			).unwrap();

			let mut os_ext = os_ext.to_owned();
			os_ext.push(".spv");
			path.set_extension(&os_ext);
			let mut file = File::create(&path).unwrap();
			let mut spirv = Vec::new();
			shader.read_to_end(&mut spirv).unwrap();
			println!("Compiled spirv {:?} of length {:?}", &path.as_os_str(), spirv.len());
			file.write_all(&spirv).unwrap();
		}
	}
}

fn main() {
	create_gta_icon();
	pre_compile_shaders();
}
