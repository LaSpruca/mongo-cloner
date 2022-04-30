mod app;
pub mod widgets;

use crate::app::MyApp;
use eframe::egui::Vec2;

fn main() {
    tracing_subscriber::fmt::init();
    let app = MyApp::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2 {
        x: 1000.0,
        y: 500.0,
    });
    eframe::run_native(Box::new(app), native_options);
}
