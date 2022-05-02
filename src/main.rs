mod app;
mod db;
mod widgets;

use crate::app::MyApp;
use eframe::egui::Vec2;
use tracing::metadata::LevelFilter;

fn main() {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();
    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    let app = MyApp::default();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2 {
            x: 1000.0,
            y: 500.0,
        }),
        ..Default::default()
    };

    eframe::run_native(Box::new(app), native_options);
}
