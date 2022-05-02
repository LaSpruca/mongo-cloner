use crate::widgets::server_address::ServerAddress;
use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, Visuals};
use eframe::epi::{Frame, Storage};
use eframe::{egui, epi};
use mongodb::error::{Error as MongoError, Result as MongoResult};
use poll_promise::Promise;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, info};

use crate::db::DbClient;
use url::Url;

type CollectionsVec = Vec<((String, String), Vec<(String, String, bool)>)>;

pub struct MyApp {
    source: Url,
    target: Url,
    rt: Runtime,
    source_client: Option<Arc<DbClient>>,
    collections: Option<Promise<MongoResult<CollectionsVec>>>,
    mg_err: Option<(String, MongoError)>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            source: Url::parse("mongodb://username:password@localhost:27017").unwrap(),
            target: Url::parse("mongodb://username:password@localhost:27017/?ssl=false").unwrap(),
            rt: Runtime::new().unwrap(),
            source_client: None,
            collections: None,
            mg_err: None,
        }
    }
}

impl epi::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        if let Some((stage, ex)) = self.mg_err.clone() {
            egui::Window::new("Mongo Error").show(ctx, |ui| {
                ui.heading(&stage);
                ui.label(&format!("{ex:#?}"));
                if ui.button("Ok").clicked() {
                    self.mg_err = None;
                }
            });
        }

        if self.collections.is_none() {
            if let Some(client) = &self.source_client {
                let (sender, promise) = Promise::new();
                let client = client.clone();
                let ctx = ctx.clone();

                self.rt.spawn(async move {
                    let response = client.get_collections().await.map(|collections| {
                        collections
                            .into_iter()
                            .map(|(database, collections)| {
                                (
                                    (database.clone(), database),
                                    collections
                                        .into_iter()
                                        .map(|f| (f.clone(), f, true))
                                        .collect::<Vec<_>>(),
                                )
                            })
                            .collect::<Vec<_>>()
                    });
                    debug!("Got collections");
                    sender.send(response);
                    ctx.request_repaint();
                });

                self.collections = Some(promise);
            }
        }

        if let Some(collections) = &self.collections {
            if let Some(res) = collections.ready() {
                match res {
                    Ok(val) => {
                        info!("Found collections: {val:#?}");
                    }
                    Err(ex) => {
                        self.mg_err = Some(("Error getting collections".into(), ex.clone()));
                        self.collections = None;
                        self.source_client = None;
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let _size_width = ctx.available_rect().width() / 2.0 * 0.9;
            // let side_padding = ctx.available_rect().height() / 2.0 * 0.25;

            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    ui.heading("Source");
                    ui.add(&mut ServerAddress::new(&mut self.source));
                    if ui.button("Connect").clicked() {
                        match DbClient::new(&self.source, &self.rt) {
                            Ok(stream) => self.source_client = Some(Arc::new(stream)),
                            Err(ex) => self.mg_err = Some(("Error connecting to DB".into(), ex)),
                        };
                    }
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
