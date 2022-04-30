use crate::widgets::server_address::ServerAddress;
use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, Visuals};
use eframe::epi::{Frame, Storage};
use eframe::{egui, epi};
use url::Url;

pub struct MyApp {
    source: Url,
    target: Url,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            source: Url::parse("mongodb+srv://username:password@localhost:27017").unwrap(),
            target: Url::parse("mongodb+srv://username:password@localhost:27017/?ssl=false")
                .unwrap(),
        }
    }
}

impl epi::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let size_width = ctx.available_rect().width() / 2.0 * 0.9;
            // let side_padding = ctx.available_rect().height() / 2.0 * 0.25;

            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.heading("Source");
                    ui.add(&mut ServerAddress::new(&mut self.source));
                });

                columns[1].vertical(|ui| {
                    ui.heading("Target");
                    ui.add(&mut ServerAddress::new(&mut self.target));
                });
            });
        });
    }

    fn setup(&mut self, ctx: &Context, _frame: &Frame, _storage: Option<&dyn Storage>) {
        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "Roboto-Regular".to_owned(),
            FontData::from_static(include_bytes!("../fonts/Roboto/Roboto-Regular.ttf")),
        );

        fonts.font_data.insert(
            "Roboto-Bold".to_owned(),
            FontData::from_static(include_bytes!("../fonts/Roboto/Roboto-Bold.ttf")),
        );

        fonts.font_data.insert(
            "Fira-Code-Regular".to_owned(),
            FontData::from_static(include_bytes!(
                "../fonts/Fira_Code/static/FiraCode-Regular.ttf"
            )),
        );

        fonts.families.insert(
            FontFamily::Proportional,
            vec!["Roboto-Regular".into(), "Roboto-Bold".into()],
        );

        fonts
            .families
            .insert(FontFamily::Monospace, vec!["Fira-Code-Regular".into()]);

        ctx.set_fonts(fonts);

        ctx.set_visuals(Visuals::dark());
    }

    fn name(&self) -> &str {
        "Mongo Cloner"
    }
}
