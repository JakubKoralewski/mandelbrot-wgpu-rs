//! Home of the split view that displays both Mandelbrot and Julia at the same time.

mod mandelbrot;
mod julia;
mod double;

pub use self::double::DoubleViewManager;
pub use self::julia::JuliaDoubleView;
pub use self::mandelbrot::MandelbrotDoubleView;
