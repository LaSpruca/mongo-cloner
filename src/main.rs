#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod db;
mod widgets;

use crate::app::MongoClonerApp;
use eframe::{egui::Vec2, IconData};
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

    let icon = image::load_from_memory(include_bytes!("../logos/logo-128x128.png"))
        .unwrap()
        .to_rgba8();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2 {
            x: 1000.0,
            y: 750.0,
        }),
        icon_data: Some(IconData {
            height: icon.height(),
            width: icon.width(),
            rgba: icon.to_vec(),
        }),
        ..Default::default()
    };

    eframe::run_native(
        "Mongo Cloner",
        native_options,
        Box::new(|ctx| Box::new(MongoClonerApp::new(ctx))),
    );
}
