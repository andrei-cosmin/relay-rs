mod app;
mod components;
mod pages;
mod state;
#[cfg(feature = "standalone")]
mod title_bar;
#[cfg(not(feature = "standalone"))]
mod web_title_bar;

pub use app::App;
#[cfg(feature = "standalone")]
pub use title_bar::TitleBarWindow;
