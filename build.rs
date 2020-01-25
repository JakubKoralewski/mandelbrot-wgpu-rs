extern crate image;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

fn main() {
	let mut icon_save_path = std::env::current_dir().unwrap();
	icon_save_path.push::<PathBuf>(["res", "gta_sa_icon"].iter().collect());
	if icon_save_path.exists() {
		return;
	}
	let icon: Vec<u8> = {
		let icon_image =
			image::load_from_memory_with_format(
				include_bytes!("res/gta_sa.ico"),
				image::ImageFormat::ICO
			).expect("Error decoding icon");

		icon_image.raw_pixels()
	};

	let mut icon_save_file = File::create(icon_save_path).unwrap();
	icon_save_file.write(&icon);
}
