use crate::widgets::db_render::DbDisplay;
use crate::widgets::server_address::ServerAddress;
use eframe::{
    egui,
    egui::{
        FontData, FontDefinitions, FontFamily, ProgressBar, Rgba, RichText, ScrollArea, Visuals,
    },
    App, CreationContext, Frame,
};
use mongodb::error::{Error as MongoError, Result as MongoResult};
use poll_promise::Promise;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, error};

use crate::db::{ClusterClient, Db};
use url::Url;

/// The main application
pub struct MongoClonerApp {
    /// The Url of the cluster to be cloned
    source: Url,
    /// The Url of the cluster that `source` will be cloned to
    target: Url,
    /// A tokio runtime for doing async shit
    rt: Runtime,
    /// The connection to the `source` cluster
    source_client: Option<Arc<ClusterClient>>,
    /// The databases and their collections in the `source` cluster
    collections: Option<Promise<MongoResult<Vec<Db>>>>,
    /// All collections that have been uploaded
    uploaded_collections: Option<Vec<Promise<(String, String, MongoResult<()>)>>>,
    /// The number of collections that are scheduled to be uploaded,
    uploaded_count: Option<usize>,
    /// Any mongo errors that may have occurred
    mg_err: Option<(String, MongoError)>,
}

impl App for MongoClonerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Show any errors
        if let Some((stage, ex)) = self.mg_err.clone() {
            egui::Window::new("Mongo Error").show(ctx, |ui| {
                ui.heading(&stage);
                ui.label(&format!("{ex:#?}"));
                if ui.button("Ok").clicked() {
                    self.mg_err = None;
                }
            });
        }

        // Fetch the databases in the cluster if possible to do so
        if self.source_client.is_some() && self.collections.is_none() {
            if let Some(client) = &self.source_client {
                let (sender, promise) = Promise::new();
                let client = client.clone();
                let ctx = ctx.clone();

                self.rt.spawn(async move {
                    let response = client.get_collections().await.map(|collections| {
                        collections.into_iter().map(Db::from).collect::<Vec<_>>()
                    });
                    debug!("Got collections");
                    sender.send(response);
                    ctx.request_repaint();
                });

                self.collections = Some(promise);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Inputs for server addresses
            ui.columns(2, |columns| {
                // Source database, the one to be cloned from
                columns[0].vertical_centered(|ui| {
                    ui.heading("Source");
                    ui.add(&mut ServerAddress::new(&mut self.source));
                    if ui.button("Connect").clicked() {
                        match ClusterClient::new(&self.source, &self.rt) {
                            Ok(stream) => self.source_client = Some(Arc::new(stream)),
                            Err(ex) => self.mg_err = Some(("Error connecting to source cluster".into(), ex)),
                        };
                    }
                });

                // Target database, the one to be cloned to
                columns[1].vertical_centered(|ui| {
                    ui.heading("Target");
                    ui.add(&mut ServerAddress::new(&mut self.target));
                });
            });
            // Check to see if collections have loaded
            if let Some(collections) = &mut self.collections {
                // Check to see if done
                if let Some(res) = collections.ready_mut() {
                    match res {
                        // Show collections
                        Ok(dbs) => {
                            ui.vertical_centered(|ui| {
                                ui.set_width(ctx.available_rect().width());

                                // All collections
                                ScrollArea::vertical().show(ui, |ui| {
                                    for db in dbs.iter_mut() {
                                        ui.add(&mut DbDisplay::new(db));
                                    }
                                });
                                if self.uploaded_count.is_none() {
                                    // The upload button
                                    let btn = ui.button("Clone");

                                    if btn.clicked() {
                                        match ClusterClient::new(&self.target, &self.rt) {
                                            Ok(target) => {
                                                // Number of collections to upload
                                                let mut count = 0;
                                                let mut uploaded_collections = vec![];
                                                // Create arc for target
                                                let target = Arc::new(target);

                                                // Ohhh yeah, it's funkey cloney shit time, thanks borrow checker
                                                let dbs= dbs.clone();
                                                for db in dbs {
                                                    let db2 = db.clone();
                                                    for collection in db2.collections {
                                                        if collection.selected {
                                                            count += 1;
                                                            let source = self.source_client.clone().expect("For some unknown reason, we have collections, but no client, wtf");
                                                            let target = target.clone();
                                                            let (sender, promise) = Promise::new();
                                                            let db = db.clone();
                                                            let ctx = ctx.clone();

                                                            uploaded_collections.push(promise);

                                                            self.rt.spawn(async move {
                                                                debug!("Cloning collection {}.{}", &db.db_name.name, &collection.name);
                                                                match source.download_collection(db.db_name.name.clone(), collection.name.clone()).await {
                                                                    Ok(docs) => {
                                                                        sender.send((
                                                                            format!("{}.{}", &db.db_name.name, &collection.name), 
                                                                            format!("{}.{}", &db.db_name.rename, &collection.rename),
                                                                            target.upload_collection(db.db_name.rename.clone(), collection.rename.clone(), docs).await)
                                                                        );
                                                                        ctx.request_repaint();
                                                                        debug!("Done upload for {}.{}", &db.db_name.rename, &collection.rename);
                                                                    }
                                                                    Err(err) => {
                                                                        sender.send((
                                                                            format!("{}.{}", &db.db_name.name, &collection.name),
                                                                            format!("{}.{}", &db.db_name.rename, &collection.rename),
                                                                            Err(err))
                                                                        );
                                                                        ctx.request_repaint();
                                                                    }
                                                                };
                                                            });

                                                        }
                                                    }
                                                }
                                                self.uploaded_count = Some(count);
                                                self.uploaded_collections = Some(uploaded_collections);
                                            }
                                            Err(ex) => self.mg_err = Some(("Error connecting to target cluster".into(), ex)),
                                        };

                                    }
                                }
                            });
                        }
                        // Set error, disconnect from cluster, clear promise
                        Err(ex) => {
                            self.mg_err = Some(("Error getting collections".into(), ex.clone()));
                            self.collections = None;
                            self.source_client = None;
                        }
                    }
                }
                // Show a loading dialog
                else {
                    ui.vertical_centered(|ui| {
                        ui.label("Loading, please wait");
                        ui.spinner();
                        if ui.button("Cancel").clicked() {
                            self.collections = None;
                            self.source_client = None;
                        }
                    });
                }
            }

            if let Some(count) = self.uploaded_count {
                let mut processed = vec![];

                if let Some(f) = &self.uploaded_collections {
                    for promise in f.iter() {
                        if let Some((name, rename, val)) = promise.ready() {
                            processed.push((name.clone(), rename.clone(), val.clone()));
                        }
                    }

                    debug!("{}, {}", processed.len(), f.len());
                }


                egui::Window::new("Uploading").show(ctx, |ui| {
                    ui.add(ProgressBar::new(processed.len() as f32 / count as f32));
                    ScrollArea::vertical().show(ui, |ui| {
                        for (name, rename, val) in processed {
                            if let Err(err) = val {
                                ui.label(RichText::new(format!("Error processing {name}: {err}")).color(Rgba::from_srgba_premultiplied(250, 0, 0, 255)));
                            } else {
                                ui.label(RichText::new(format!("Successfully copied {name} -> {rename}")).color(Rgba::from_srgba_premultiplied(0, 250, 0, 255)));
                            }
                        }
                    });
                });
            }
        });
    }
}

impl Default for MongoClonerApp {
    fn default() -> Self {
        Self {
            source: Url::parse("mongodb://username:password@host:27017").unwrap(),
            target: Url::parse("mongodb://username:password@host:27017/?ssl=false").unwrap(),
            rt: Runtime::new().unwrap(),
            source_client: None,
            collections: None,
            uploaded_collections: None,
            uploaded_count: None,
            mg_err: None,
        }
    }
}

impl MongoClonerApp {
    pub fn new(CreationContext { egui_ctx: ctx, .. }: &CreationContext) -> Self {
        // Fonts used by the application
        let mut fonts = FontDefinitions::default();

        // We do a little font loadge
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

        // Add the fonts
        ctx.set_fonts(fonts);

        // Dark mode, fuck light mode
        ctx.set_visuals(Visuals::dark());

        Self::default()
    }
}
