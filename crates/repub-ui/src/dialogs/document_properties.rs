//! Document Properties dialog: title, author, subject and keywords, plus the
//! document's measurement unit, grid and snapping.

use egui::Ui;
use repub_model::DocumentMetadata;

use super::controls::{choice, number_row, ok_cancel, row};
use crate::state::{AppState, DisplayUnit};

/// The Document Properties dialog.
#[derive(Default)]
pub struct DocumentPropertiesDialog {
    pub open: bool,
    metadata: DocumentMetadata,
    title: String,
    author: String,
    subject: String,
    keywords: String,
    units: DisplayUnit,
    grid_spacing: f64,
    show_rulers: bool,
    show_guides: bool,
    show_grid: bool,
    snap_to_grid: bool,
    snap_to_guides: bool,
    snap_to_objects: bool,
}

impl DocumentPropertiesDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_for(&mut self, state: &AppState) {
        let Some(doc) = state.document.as_ref() else {
            return;
        };
        self.metadata = doc.metadata.clone();
        self.title = self.metadata.title.clone().unwrap_or_default();
        self.author = self.metadata.author.clone().unwrap_or_default();
        self.subject = self.metadata.subject.clone().unwrap_or_default();
        self.keywords = self.metadata.keywords.join(", ");
        self.units = state.units;
        self.grid_spacing = state.grid_spacing;
        self.show_rulers = state.show_rulers;
        self.show_guides = state.show_guides;
        self.show_grid = state.show_grid;
        self.snap_to_grid = state.snap_to_grid;
        self.snap_to_guides = state.snap_to_guides;
        self.snap_to_objects = state.snap_to_objects;
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.open {
            return;
        }
        let mut result = None;
        egui::Window::new("Document Properties")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                self.fields(ui);
                result = ok_cancel(ui);
            });
        match result {
            Some(true) => {
                self.apply(state);
                self.open = false;
            }
            Some(false) => self.open = false,
            None => {}
        }
    }

    fn fields(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Summary").strong());
        text_row(ui, "Title", &mut self.title);
        text_row(ui, "Author", &mut self.author);
        text_row(ui, "Subject", &mut self.subject);
        text_row(ui, "Keywords", &mut self.keywords);
        if let Some(created) = self.metadata.created {
            row(ui, "Created", |ui| ui.label(format_timestamp(created)));
        }
        if let Some(modified) = self.metadata.modified {
            row(ui, "Modified", |ui| ui.label(format_timestamp(modified)));
        }
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Measurement and grid").strong());
        choice(
            ui,
            "doc_units",
            "Units",
            &mut self.units,
            &[
                (DisplayUnit::Inches, "Inches"),
                (DisplayUnit::Millimeters, "Millimeters"),
                (DisplayUnit::Points, "Points"),
            ],
        );
        let units = self.units;
        let mut spacing = units.from_points(self.grid_spacing);
        if number_row(
            ui,
            "Grid spacing",
            &mut spacing,
            units.from_points(1.0)..=units.from_points(720.0),
            units.from_points(0.5),
            units.suffix(),
        ) {
            self.grid_spacing = units.to_points(spacing);
        }
        ui.add_space(6.0);
        row(ui, "Show", |ui| {
            ui.checkbox(&mut self.show_rulers, "Rulers");
            ui.checkbox(&mut self.show_guides, "Guides");
            ui.checkbox(&mut self.show_grid, "Grid");
        });
        row(ui, "Snap to", |ui| {
            ui.checkbox(&mut self.snap_to_grid, "Grid");
            ui.checkbox(&mut self.snap_to_guides, "Guides");
            ui.checkbox(&mut self.snap_to_objects, "Objects");
        });
    }

    fn apply(&self, state: &mut AppState) {
        let opt = |s: &str| {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_string())
        };
        state.units = self.units;
        state.grid_spacing = self.grid_spacing;
        state.show_rulers = self.show_rulers;
        state.show_guides = self.show_guides;
        state.show_grid = self.show_grid;
        state.snap_to_grid = self.snap_to_grid;
        state.snap_to_guides = self.snap_to_guides;
        state.snap_to_objects = self.snap_to_objects;
        state.record_undo("Document Properties");
        if let Some(doc) = state.document.as_mut() {
            doc.metadata.title = opt(&self.title);
            doc.metadata.author = opt(&self.author);
            doc.metadata.subject = opt(&self.subject);
            doc.metadata.keywords = self
                .keywords
                .split(',')
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .map(str::to_string)
                .collect();
        }
        state.store_document_settings();
        state.mark_dirty();
    }
}

fn text_row(ui: &mut Ui, label: &str, text: &mut String) {
    row(ui, label, |ui| {
        ui.add(egui::TextEdit::singleline(text).desired_width(280.0))
    });
}

/// A Unix timestamp as `YYYY-MM-DD HH:MM` (UTC).
pub fn format_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::format_timestamp;

    #[test]
    fn timestamps_format_as_utc_dates() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_timestamp(1_000_000_000), "2001-09-09 01:46 UTC");
        assert_eq!(format_timestamp(1_709_164_800), "2024-02-29 00:00 UTC");
    }
}
