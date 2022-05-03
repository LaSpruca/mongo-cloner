use crate::db::Db;
use eframe::{
    egui::{Response, RichText, Ui, Widget},
    epaint::FontFamily,
};

pub struct DbDisplay<'a> {
    db: &'a mut Db,
}

impl<'a> DbDisplay<'a> {
    pub fn new(db: &'a mut Db) -> Self {
        Self { db }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.collapsing(self.db.db_name.name.as_str(), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Rename to: ");
                        ui.text_edit_singleline(&mut self.db.db_name.rename);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("All").clicked() {
                            self.db
                                .collections
                                .iter_mut()
                                .for_each(|f| f.selected = true);
                        }

                        if ui.button("None").clicked() {
                            self.db
                                .collections
                                .iter_mut()
                                .for_each(|f| f.selected = false);
                        }
                    });
                    for collection in self.db.collections.iter_mut() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut collection.selected, collection.name.as_str());
                            ui.label(RichText::new("->").family(FontFamily::Monospace));
                            ui.text_edit_singleline(&mut collection.rename);
                        });
                    }
                });
            })
        })
        .response
    }
}

impl<'a> Widget for &mut DbDisplay<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}
