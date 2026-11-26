//! Page Properties dialog: size, orientation, margins and background of a page.

use egui::Ui;
use repub_model::styles::Fill;
use repub_model::{Page, PageOrientation, PageSize};

use super::controls::{choice, number_row, ok_cancel, row};
use super::format_shape::fill_editor;
use crate::state::{AppState, DisplayUnit};

/// Every page size the model knows, with its name.
const SIZES: [(PageSize, &str); 12] = [
    (PageSize::Letter, "Letter (8.5 × 11 in)"),
    (PageSize::Legal, "Legal (8.5 × 14 in)"),
    (PageSize::Tabloid, "Tabloid (11 × 17 in)"),
    (PageSize::Executive, "Executive (7.25 × 10.5 in)"),
    (PageSize::A3, "A3 (297 × 420 mm)"),
    (PageSize::A4, "A4 (210 × 297 mm)"),
    (PageSize::A5, "A5 (148 × 210 mm)"),
    (PageSize::A6, "A6 (105 × 148 mm)"),
    (PageSize::B5, "B5 (176 × 250 mm)"),
    (PageSize::BusinessCard, "Business card (3.5 × 2 in)"),
    (PageSize::IndexCard4x6, "Index card (4 × 6 in)"),
    (PageSize::IndexCard5x7, "Index card (5 × 7 in)"),
];

/// A size choice: one of the presets, or custom dimensions.
#[derive(Clone, Copy, PartialEq)]
enum SizeChoice {
    Preset(usize),
    Custom,
}

/// Page Properties dialog state.
pub struct PagePropertiesDialog {
    pub open: bool,
    page_index: Option<usize>,
    size: SizeChoice,
    /// Portrait width and height, points
    width: f64,
    height: f64,
    orientation: PageOrientation,
    margins: [f64; 4],
    background: Fill,
    all_pages: bool,
    units: DisplayUnit,
}

impl Default for PagePropertiesDialog {
    fn default() -> Self {
        Self {
            open: false,
            page_index: None,
            size: SizeChoice::Preset(0),
            width: 612.0,
            height: 792.0,
            orientation: PageOrientation::Portrait,
            margins: [72.0; 4],
            background: Fill::None,
            all_pages: false,
            units: DisplayUnit::Inches,
        }
    }
}

impl PagePropertiesDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open for the current page.
    pub fn open_for_page(&mut self, state: &AppState) {
        self.open_for_index(state, state.current_page);
    }

    /// Open for the page at `index`.
    pub fn open_for_index(&mut self, state: &AppState, index: usize) {
        let page = if index == state.current_page {
            state.current_page()
        } else {
            state.document.as_ref().and_then(|d| d.pages().get(index))
        };
        let Some(page) = page else { return };
        self.page_index = Some(index);
        self.orientation = if page.size.width > page.size.height { PageOrientation::Landscape } else { PageOrientation::Portrait };
        let (w, h) = (page.size.width.min(page.size.height), page.size.width.max(page.size.height));
        self.width = w;
        self.height = h;
        self.size = matching_size(w, h);
        self.margins = [page.margins.top, page.margins.bottom, page.margins.left, page.margins.right];
        self.background = page.background.clone().unwrap_or(Fill::None);
        self.all_pages = false;
        self.units = state.units;
        self.open = true;
    }

    /// Show the dialog. `Some(true)` when OK was clicked, `Some(false)` for Cancel.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }
        let mut result = None;
        egui::Window::new("Page Properties").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_min_width(400.0);
            egui::ScrollArea::vertical().max_height(450.0).auto_shrink([false, true]).show(ui, |ui| self.fields(ui));
            result = ok_cancel(ui);
        });
        match result {
            Some(true) => {
                if let Some(index) = self.page_index {
                    self.apply(state, index);
                }
                self.open = false;
            }
            Some(false) => self.open = false,
            None => {}
        }
        result
    }

    fn fields(&mut self, ui: &mut Ui) {
        let units = self.units;
        ui.label(egui::RichText::new("Page size").strong());
        let mut options: Vec<(SizeChoice, &str)> = SIZES.iter().enumerate().map(|(i, (_, name))| (SizeChoice::Preset(i), *name)).collect();
        options.push((SizeChoice::Custom, "Custom"));
        let before = self.size;
        choice(ui, "page_size_preset", "Size", &mut self.size, &options);
        if self.size != before {
            if let SizeChoice::Preset(i) = self.size {
                let points = SIZES[i].0.to_points();
                self.width = points.width.min(points.height);
                self.height = points.width.max(points.height);
            }
        }
        if self.size == SizeChoice::Custom {
            let range = units.from_points(18.0)..=units.from_points(5000.0);
            let mut w = units.from_points(self.width);
            let mut h = units.from_points(self.height);
            if number_row(ui, "Width", &mut w, range.clone(), units.from_points(1.0), units.suffix()) {
                self.width = units.to_points(w);
            }
            if number_row(ui, "Height", &mut h, range, units.from_points(1.0), units.suffix()) {
                self.height = units.to_points(h);
            }
        }
        choice(ui, "page_orientation", "Orientation", &mut self.orientation, &[(PageOrientation::Portrait, "Portrait"), (PageOrientation::Landscape, "Landscape")]);
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Margins").strong());
        for (label, value) in ["Top", "Bottom", "Left", "Right"].into_iter().zip(self.margins.iter_mut()) {
            let mut shown = units.from_points(*value);
            if number_row(ui, label, &mut shown, 0.0..=units.from_points(500.0), units.from_points(0.5), units.suffix()) {
                *value = units.to_points(shown);
            }
        }
        row(ui, "", |ui| ui.checkbox(&mut self.all_pages, "Apply size and margins to every page"));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Background").strong());
        fill_editor(ui, &mut self.background);
    }

    fn apply(&self, state: &mut AppState, index: usize) {
        state.record_undo("Page Setup");
        let (w, h) = match self.orientation {
            PageOrientation::Portrait => (self.width, self.height),
            PageOrientation::Landscape => (self.height, self.width),
        };
        let setup = |page: &mut Page| {
            page.size.width = w;
            page.size.height = h;
            page.margins.top = self.margins[0];
            page.margins.bottom = self.margins[1];
            page.margins.left = self.margins[2];
            page.margins.right = self.margins[3];
        };
        let background = (!self.background.is_none()).then(|| self.background.clone());
        if self.all_pages {
            if let Some(doc) = state.document.as_mut() {
                doc.pages_mut().iter_mut().for_each(setup);
            }
        }
        if index == state.current_page {
            if let Some(page) = state.current_page_mut() {
                setup(page);
                page.background = background;
            }
        } else if let Some(page) = state.document.as_mut().and_then(|d| d.pages_mut().get_mut(index)) {
            setup(page);
            page.background = background;
        }
        state.mark_dirty();
    }
}

/// The preset with these portrait dimensions, or custom.
fn matching_size(width: f64, height: f64) -> SizeChoice {
    SIZES
        .iter()
        .position(|(size, _)| {
            let p = size.to_points();
            let (pw, ph) = (p.width.min(p.height), p.width.max(p.height));
            (pw - width).abs() < 1.0 && (ph - height).abs() < 1.0
        })
        .map_or(SizeChoice::Custom, SizeChoice::Preset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_matched_whatever_the_orientation_they_are_stored_in() {
        assert!(matches!(matching_size(612.0, 792.0), SizeChoice::Preset(0)));
        assert!(matches!(matching_size(144.0, 252.0), SizeChoice::Preset(9)));
        assert!(matches!(matching_size(500.0, 500.0), SizeChoice::Custom));
    }

    #[test]
    fn a_background_gradient_survives_editing_other_properties() {
        use repub_model::{Document, PageOrientation as O};
        let mut state = AppState::new();
        let mut doc = Document::new(PageSize::Letter, O::Portrait);
        let gradient = Fill::LinearGradient(repub_color::LinearGradient::horizontal(repub_color::Color::WHITE, repub_color::Color::BLACK));
        doc.pages_mut()[0].background = Some(gradient.clone());
        state.document = Some(doc);
        let mut dialog = PagePropertiesDialog::new();
        dialog.open_for_page(&state);
        dialog.margins = [10.0; 4];
        dialog.apply(&mut state, 0);
        let page = state.current_page().unwrap();
        assert_eq!(page.background, Some(gradient));
        assert_eq!(page.margins.top, 10.0);
    }
}
