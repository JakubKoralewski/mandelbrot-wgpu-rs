//! Home of view managers.

mod mandelbrot;
mod mandelbrot_and_julia;
mod view;
mod utils;
mod switchable;

pub use self::mandelbrot::MandelbrotViewManager;
pub use self::mandelbrot_and_julia::{DoubleViewManager, JuliaDoubleView, MandelbrotDoubleView};
pub use self::view::FractalViewManager;
pub use self::prelude::FRAG_SHADER_PATH;
pub use self::switchable::SwitchableViewManager;

mod prelude;

