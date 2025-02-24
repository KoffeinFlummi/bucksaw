type Cbs<'a> = Vec<Box<dyn FnOnce(&mut egui::Ui) -> egui::Response + 'a>>;

pub struct FlexLayout<'a> {
    heading: String,
    min_width: f32,
    cbs: Cbs<'a>,
}

impl<'a> FlexLayout<'a> {
    pub fn new(min_width: f32, heading: impl ToString) -> Self {
        Self {
            heading: heading.to_string(),
            min_width,
            cbs: vec![],
        }
    }

    pub fn add<F>(mut self, cb: F) -> Self
    where
        F: FnOnce(&mut egui::Ui) -> egui::Response + 'a,
    {
        self.cbs.push(Box::new(cb));
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}

pub struct FlexColumns<'a> {
    min_width: f32,
    cbs: Cbs<'a>,
}

impl egui::Widget for FlexLayout<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        if self.cbs.is_empty() {
            return ui.label("");
        }

        if ui.available_width() < self.min_width {
            ui.vertical(|ui| {
                ui.collapsing(self.heading, |ui| {
                    self.cbs.into_iter().map(|cb| cb(ui)).last().unwrap()
                })
                .header_response
            })
            .inner
        } else {
            ui.horizontal(|ui| {
                self.cbs
                    .into_iter()
                    .enumerate()
                    .map(|(i, cb)| {
                        if i != 0 {
                            ui.separator();
                        }

                        cb(ui)
                    })
                    .last()
                    .unwrap()
            })
            .inner
        }
    }
}

impl<'a> FlexColumns<'a> {
    pub fn new(min_width: f32) -> Self {
        Self {
            min_width,
            cbs: vec![],
        }
    }

    pub fn column<F>(mut self, cb: F) -> Self
    where
        F: FnOnce(&mut egui::Ui) -> egui::Response + 'a,
    {
        self.cbs.push(Box::new(cb));
        self
    }

    pub fn column_enabled<F>(mut self, enabled: bool, cb: F) -> Self
    where
        F: FnOnce(&mut egui::Ui) -> egui::Response + 'a,
    {
        if enabled {
            self.cbs.push(Box::new(cb));
        }
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}

impl egui::Widget for FlexColumns<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        if self.cbs.is_empty() {
            return ui.label("");
        }

        if ui.available_width() < self.min_width {
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    self.cbs
                        .into_iter()
                        .enumerate()
                        .map(|(i, cb)| {
                            if i != 0 {
                                ui.separator();
                            }

                            cb(ui)
                        })
                        .last()
                        .unwrap()
                })
                .inner
        } else {
            ui.columns(self.cbs.len(), |columns| {
                self.cbs
                    .into_iter()
                    .enumerate()
                    .map(|(i, cb)| cb(&mut columns[i]))
                    .last()
                    .unwrap()
            })
        }
    }
}
