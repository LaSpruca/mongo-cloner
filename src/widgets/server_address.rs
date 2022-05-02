use eframe::egui::{Response, TextEdit, Ui, Widget};

use std::str::FromStr;
use tracing::{debug, error};

use url::Url;

pub struct ServerAddress<'a> {
    source_field: &'a mut Url,
}

impl<'a> ServerAddress<'a> {
    pub fn new(url: &'a mut Url) -> Self {
        Self { source_field: url }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let mut source_field = format!("{}", self.source_field);
        let mut username = self.source_field.username().to_string();
        let mut password = self.source_field.password().unwrap_or_default().to_string();

        let mut host = self
            .source_field
            .host()
            .map(|k| format!("{k}"))
            .unwrap_or_default();

        let mut port = self
            .source_field
            .port()
            .map(|x| x.to_string())
            .unwrap_or_default();

        let mut ssl = self
            .source_field
            .query_pairs()
            .find(|(key, _)| key == "ssl")
            .map(|(_, value)| value.parse().unwrap_or(false))
            .unwrap_or_default();

        let mut srv = self.source_field.scheme().ends_with("+srv");

        if srv {
            self.source_field.set_port(None).unwrap();
        }

        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                ui.label("Connection Uri: ");

                if ui
                    .add(
                        TextEdit::singleline(&mut source_field).desired_width(ui.available_width()),
                    )
                    .changed()
                {
                    match Url::from_str(&source_field) {
                        Ok(url) => {
                            *self.source_field = url;
                        }
                        Err(ex) => {
                            error!("Could not do parse uri: {ex}")
                        }
                    }
                };
            });

            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Host");
                    if ui.text_edit_singleline(&mut host).changed() {
                        self.source_field.set_host(Some(host.as_str())).unwrap();
                    };
                });
                ui.horizontal(|ui| {
                    ui.label("Port");
                    if ui
                        .add(TextEdit::singleline(&mut port).interactive(!srv))
                        .changed()
                    {
                        if port.trim() == "" {
                            self.source_field.set_port(None).unwrap();
                        }
                        match port.parse() {
                            Ok(val) => self.source_field.set_port(Some(val)).unwrap(),
                            Err(e) => {
                                error!("Inlaid Port: {e}")
                            }
                        }
                    }
                });
            });

            ui.columns(2, |columns| {
                columns[0].horizontal(|ui| {
                    ui.label("Username");
                    if ui.text_edit_singleline(&mut username).changed() {
                        self.source_field.set_username(username.as_str()).unwrap();
                    };
                });
                columns[1].horizontal(|ui| {
                    ui.label("Password");
                    if ui.text_edit_singleline(&mut password).changed() {
                        if password.is_empty() {
                            self.source_field.set_password(None).unwrap();
                        } else {
                            self.source_field
                                .set_password(Some(password.as_str()))
                                .unwrap()
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                if ui.checkbox(&mut ssl, "Use SSL").changed() {
                    let query_params: Vec<(_, _)> = self
                        .source_field
                        .query_pairs()
                        .filter(|(key, _)| key != "ssl")
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    self.source_field.query_pairs_mut().clear().finish();
                    for (key, val) in query_params {
                        self.source_field
                            .query_pairs_mut()
                            .append_pair(key.as_ref(), val.as_ref())
                            .finish();
                    }

                    self.source_field
                        .query_pairs_mut()
                        .append_pair("ssl", &ssl.to_string())
                        .finish();
                }

                if ui.checkbox(&mut srv, "Use +srv").changed() {
                    let scheme = self.source_field.scheme().to_string();
                    if !srv {
                        debug!("Stripping +srv from scheme");
                        self.source_field
                            .set_scheme(scheme.strip_suffix("+srv").unwrap_or(&scheme))
                            .unwrap();
                    } else {
                        debug!("Adding +srv from scheme");
                        self.source_field
                            .set_scheme(&format!("{scheme}+srv"))
                            .unwrap();
                    }
                }
            });
        })
        .response
    }
}

impl<'a> Widget for &mut ServerAddress<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}
