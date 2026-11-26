//! New from Template dialog: pick a template and fill in its placeholders.

use repub_model::Document;
use repub_templates::{
    PlaceholderType, PlaceholderValues, Template, TemplateCategory, TemplateLibrary,
};

use super::controls::ok_cancel;

/// The New from Template dialog.
pub struct TemplatesDialog {
    pub open: bool,
    library: TemplateLibrary,
    search: String,
    category: Option<TemplateCategory>,
    selected: Option<String>,
    /// Text typed for each placeholder, by placeholder id
    values: std::collections::HashMap<String, String>,
    error: Option<String>,
}

impl Default for TemplatesDialog {
    fn default() -> Self {
        Self {
            open: false,
            library: TemplateLibrary::with_builtins(),
            search: String::new(),
            category: None,
            selected: None,
            values: Default::default(),
            error: None,
        }
    }
}

impl TemplatesDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
        self.error = None;
    }

    /// Show the dialog. Returns the new document when the user creates one.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Document> {
        if !self.open {
            return None;
        }
        let mut created = None;
        let mut result = None;
        egui::Window::new("New from Template")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(640.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search templates")
                            .desired_width(220.0),
                    );
                    if ui
                        .selectable_label(self.category.is_none(), "All")
                        .clicked()
                    {
                        self.category = None;
                    }
                    for category in TemplateCategory::all() {
                        if self.library.by_category(*category).is_empty() {
                            continue;
                        }
                        if ui
                            .selectable_label(self.category == Some(*category), category.name())
                            .clicked()
                        {
                            self.category = Some(*category);
                        }
                    }
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(220.0);
                        self.list(ui);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_min_width(380.0);
                        self.details(ui);
                    });
                });
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(0xC0, 0x30, 0x30), error);
                }
                result = ok_cancel(ui);
            });
        match result {
            Some(true) => match self.create() {
                Ok(Some(doc)) => {
                    created = Some(doc);
                    self.open = false;
                }
                Ok(None) => self.error = Some("Choose a template first.".into()),
                Err(e) => self.error = Some(e),
            },
            Some(false) => self.open = false,
            None => {}
        }
        created
    }

    fn matching(&self) -> Vec<&Template> {
        let mut found: Vec<&Template> = if self.search.trim().is_empty() {
            self.library.all().collect()
        } else {
            self.library.search(self.search.trim())
        };
        if let Some(category) = self.category {
            found.retain(|t| t.metadata.category == category);
        }
        found.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        found
    }

    fn list(&mut self, ui: &mut egui::Ui) {
        let templates: Vec<(String, String)> = self
            .matching()
            .iter()
            .map(|t| (t.metadata.id.clone(), t.metadata.name.clone()))
            .collect();
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .id_salt("template_list")
            .show(ui, |ui| {
                if templates.is_empty() {
                    ui.label("No templates match.");
                }
                for (id, name) in templates {
                    if ui
                        .selectable_label(self.selected.as_deref() == Some(id.as_str()), name)
                        .clicked()
                    {
                        self.selected = Some(id);
                        self.values.clear();
                        self.error = None;
                    }
                }
            });
    }

    fn details(&mut self, ui: &mut egui::Ui) {
        let Some(template) = self.selected.as_deref().and_then(|id| self.library.get(id)) else {
            ui.label("Choose a template to see what you can fill in.");
            return;
        };
        ui.label(
            egui::RichText::new(&template.metadata.name)
                .strong()
                .size(16.0),
        );
        ui.label(&template.metadata.description);
        let size = template.document.pages().first().map(|p| p.size);
        if let Some(size) = size {
            ui.label(
                egui::RichText::new(format!(
                    "{:.2} × {:.2} in, {} page(s)",
                    size.width / 72.0,
                    size.height / 72.0,
                    template.document.pages().len()
                ))
                .weak(),
            );
        }
        ui.add_space(8.0);
        let placeholders: Vec<_> = template.placeholders.clone();
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .id_salt("template_fields")
            .show(ui, |ui| {
                for p in placeholders {
                    ui.label(format!("{}{}", p.name, if p.required { " *" } else { "" }));
                    let value = self.values.entry(p.id.clone()).or_default();
                    let hint = p
                        .example
                        .clone()
                        .or(p.default_value.clone())
                        .unwrap_or_default();
                    match p.placeholder_type {
                        PlaceholderType::MultiLineText => {
                            ui.add(
                                egui::TextEdit::multiline(value)
                                    .hint_text(hint)
                                    .desired_rows(3)
                                    .desired_width(360.0),
                            );
                        }
                        PlaceholderType::Image => {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(value)
                                        .hint_text("Picture file")
                                        .desired_width(280.0),
                                );
                                if ui.button("Choose…").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter(
                                            "Pictures",
                                            &[
                                                "png", "jpg", "jpeg", "gif", "bmp", "webp", "tif",
                                                "tiff",
                                            ],
                                        )
                                        .pick_file()
                                    {
                                        *value = path.to_string_lossy().into_owned();
                                    }
                                }
                            });
                        }
                        _ => {
                            ui.add(
                                egui::TextEdit::singleline(value)
                                    .hint_text(hint)
                                    .desired_width(360.0),
                            );
                        }
                    }
                    ui.add_space(4.0);
                }
            });
    }

    /// Build the document from the selected template and the typed values. `Ok(None)` when no
    /// template is chosen.
    fn create(&self) -> Result<Option<Document>, String> {
        let Some(template) = self.selected.as_deref().and_then(|id| self.library.get(id)) else {
            return Ok(None);
        };
        let mut values = PlaceholderValues::new();
        for p in &template.placeholders {
            let text = self.values.get(&p.id).map(|s| s.trim()).unwrap_or("");
            if text.is_empty() {
                continue;
            }
            match p.placeholder_type {
                PlaceholderType::Image => values.set_image(&p.id, text, None),
                PlaceholderType::Number | PlaceholderType::Currency => match text.parse::<f64>() {
                    Ok(n) => values.set_number(&p.id, n),
                    Err(_) => return Err(format!("\"{}\" needs a number.", p.name)),
                },
                _ => values.set(&p.id, text),
            }
        }
        template.apply(&values).map(Some).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_template_can_be_created_with_no_input_or_gives_a_clear_error() {
        let mut dialog = TemplatesDialog::new();
        let ids: Vec<String> = dialog
            .library
            .all()
            .map(|t| t.metadata.id.clone())
            .collect();
        assert!(!ids.is_empty());
        for id in ids {
            dialog.selected = Some(id.clone());
            dialog.values.clear();
            match dialog.create() {
                Ok(Some(doc)) => assert!(!doc.pages().is_empty(), "{id}"),
                Ok(None) => panic!("{id} not found"),
                Err(e) => assert!(!e.is_empty(), "{id}"),
            }
        }
    }

    #[test]
    fn search_and_category_narrow_the_list() {
        let mut dialog = TemplatesDialog::new();
        let all = dialog.matching().len();
        dialog.category = Some(TemplateCategory::Business);
        assert!(dialog.matching().len() < all);
        dialog.category = None;
        dialog.search = "zzzz-no-such-template".into();
        assert!(dialog.matching().is_empty());
    }
}
