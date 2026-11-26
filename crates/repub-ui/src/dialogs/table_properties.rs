//! Table Properties dialog: borders, column widths and row heights, and the
//! fill, alignment, padding and merging of the cell being edited.

use egui::Ui;
use repub_color::Color;
use repub_geometry::Rect;
use repub_model::shape::{CellHAlign, CellVAlign, Shape, ShapeKind, TableBorders, TableCell, TableData};
use repub_model::styles::Stroke;
use repub_model::ObjectId;

use super::controls::{choice, color_edit, number_row, ok_cancel, row, tab_strip};
use super::format_shape::fill_editor;
use crate::state::AppState;

const TABS: [&str; 4] = ["Borders", "Size", "Cell", "Fill"];

/// The Table Properties dialog.
#[derive(Default)]
pub struct TablePropertiesDialog {
    pub open: bool,
    id: Option<ObjectId>,
    tab: usize,
    original: Option<TableData>,
    work: Option<TableData>,
    /// The cell being edited when the dialog opened (row, column).
    cell: Option<(usize, usize)>,
    original_cell: TableCell,
    work_cell: TableCell,
    /// Apply fill, alignment and padding to every cell.
    all_cells: bool,
}

impl TablePropertiesDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_for_selection(&mut self, state: &AppState) {
        let [shape] = state.selected_objects()[..] else { return };
        let ShapeKind::Table(table) = &shape.kind else { return };
        self.id = Some(shape.common.id);
        self.original = Some(table.clone());
        self.work = Some(table.clone());
        self.cell = state.text_edit.as_ref().filter(|e| e.frame == shape.common.id).and_then(|e| e.cell);
        let cell = self.cell.and_then(|(r, c)| table.cell(r as u32, c as u32)).cloned().unwrap_or_default();
        self.original_cell = cell.clone();
        self.work_cell = cell;
        self.all_cells = self.cell.is_none();
        self.tab = 0;
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.open {
            return;
        }
        let Some(work) = self.work.as_mut() else {
            self.open = false;
            return;
        };
        let mut result = None;
        let cell = self.cell;
        let (work_cell, all_cells, tab) = (&mut self.work_cell, &mut self.all_cells, &mut self.tab);
        egui::Window::new("Table Properties").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_min_width(420.0);
            *tab = tab_strip(ui, *tab, &TABS);
            egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| match *tab {
                0 => borders_tab(ui, &mut work.borders),
                1 => size_tab(ui, work),
                2 => cell_tab(ui, work, cell, work_cell, all_cells),
                _ => cell_fill_tab(ui, cell, work_cell, all_cells),
            });
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

    fn apply(&self, state: &mut AppState) {
        let (Some(id), Some(original), Some(work)) = (self.id, self.original.as_ref(), self.work.as_ref()) else { return };
        state.begin_edit();
        if let Some(shape) = state.current_page_mut().and_then(|p| p.get_shape_mut(id)) {
            apply_to_shape(shape, original, work, self.cell, &self.original_cell, &self.work_cell, self.all_cells);
        }
        state.end_edit("Table Properties");
    }
}

fn apply_to_shape(
    shape: &mut Shape,
    original: &TableData,
    work: &TableData,
    cell: Option<(usize, usize)>,
    original_cell: &TableCell,
    work_cell: &TableCell,
    all_cells: bool,
) {
    let bounds = shape.common.bounds;
    let ShapeKind::Table(table) = &mut shape.kind else { return };
    if work.borders != original.borders {
        table.borders = work.borders.clone();
    }
    let resized = work.col_widths != original.col_widths || work.row_heights != original.row_heights;
    if resized {
        table.col_widths = work.col_widths.clone();
        table.row_heights = work.row_heights.clone();
    }
    let (fill, padding, h, v) = (
        work_cell.fill != original_cell.fill,
        work_cell.padding != original_cell.padding,
        work_cell.h_align != original_cell.h_align,
        work_cell.v_align != original_cell.v_align,
    );
    let mut set = |c: &mut TableCell| {
        if fill {
            c.fill = work_cell.fill.clone();
        }
        if padding {
            c.padding = work_cell.padding;
        }
        if h {
            c.h_align = work_cell.h_align;
        }
        if v {
            c.v_align = work_cell.v_align;
        }
    };
    if all_cells {
        table.cells.iter_mut().for_each(&mut set);
    } else if let Some((r, c)) = cell {
        if let Some(target) = table.cell_mut(r as u32, c as u32) {
            set(target);
        }
    }
    if let Some((r, c)) = cell {
        if (work_cell.col_span, work_cell.row_span) != (original_cell.col_span, original_cell.row_span) {
            set_span(table, r, c, work_cell.col_span, work_cell.row_span);
        }
    }
    if resized {
        shape.common.bounds = Rect::new(bounds.origin.x, bounds.origin.y, work.col_widths.iter().sum(), work.row_heights.iter().sum());
    }
}

/// Merge the cell at (`row`, `col`) over `col_span` x `row_span` cells, if that fits in the table
/// and doesn't cut into another merged cell. The cells it covers return to single cells.
pub fn set_span(table: &mut TableData, row: usize, col: usize, col_span: u32, row_span: u32) -> bool {
    let (rows, cols) = (table.rows as usize, table.cols as usize);
    let (cs, rs) = (col_span.max(1) as usize, row_span.max(1) as usize);
    if row + rs > rows || col + cs > cols {
        return false;
    }
    // Another merged cell may not overlap the new area
    for r in 0..rows {
        for c in 0..cols {
            if (r, c) == (row, col) {
                continue;
            }
            let Some(other) = table.cells.get(r * cols + c) else { continue };
            let (ors, ocs) = (other.row_span.max(1) as usize, other.col_span.max(1) as usize);
            let overlaps = r < row + rs && row < r + ors && c < col + cs && col < c + ocs;
            let inside = r >= row && r < row + rs && c >= col && c < col + cs;
            if overlaps && !(inside && ors == 1 && ocs == 1) {
                return false;
            }
        }
    }
    if let Some(cell) = table.cells.get_mut(row * cols + col) {
        cell.row_span = rs as u32;
        cell.col_span = cs as u32;
    }
    true
}

fn borders_tab(ui: &mut Ui, borders: &mut TableBorders) {
    ui.horizontal(|ui| {
        if ui.button("All").clicked() {
            *borders = TableBorders::uniform(first_stroke(borders));
        }
        if ui.button("Outside").clicked() {
            *borders = TableBorders::outer_only(first_stroke(borders));
        }
        if ui.button("None").clicked() {
            *borders = TableBorders::none();
        }
    });
    ui.add_space(6.0);
    border_row(ui, "Top", &mut borders.top);
    border_row(ui, "Bottom", &mut borders.bottom);
    border_row(ui, "Left", &mut borders.left);
    border_row(ui, "Right", &mut borders.right);
    border_row(ui, "Inside horizontal", &mut borders.inner_horizontal);
    border_row(ui, "Inside vertical", &mut borders.inner_vertical);
}

/// A stroke to spread over several borders: the first one the table has, else a thin black line.
fn first_stroke(borders: &TableBorders) -> Stroke {
    [&borders.top, &borders.bottom, &borders.left, &borders.right, &borders.inner_horizontal, &borders.inner_vertical]
        .into_iter()
        .find_map(|s| s.clone())
        .unwrap_or_else(|| Stroke::solid(Color::BLACK, 0.5))
}

fn border_row(ui: &mut Ui, label: &str, border: &mut Option<Stroke>) {
    row(ui, label, |ui| {
        let mut on = border.is_some();
        if ui.checkbox(&mut on, "").changed() {
            *border = on.then(|| Stroke::solid(Color::BLACK, 0.5));
        }
        if let Some(stroke) = border {
            color_edit(ui, &mut stroke.color);
            ui.add(egui::DragValue::new(&mut stroke.width).range(0.0..=20.0).speed(0.1).suffix(" pt"));
        }
    });
}

fn size_tab(ui: &mut Ui, table: &mut TableData) {
    ui.label(egui::RichText::new("Column widths").strong());
    for (i, width) in table.col_widths.iter_mut().enumerate() {
        number_row(ui, &format!("Column {}", i + 1), width, 4.0..=2000.0, 0.5, " pt");
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Row heights").strong());
    for (i, height) in table.row_heights.iter_mut().enumerate() {
        number_row(ui, &format!("Row {}", i + 1), height, 4.0..=2000.0, 0.5, " pt");
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Rows grow to fit their text.").weak().small());
}

fn cell_scope(ui: &mut Ui, cell: Option<(usize, usize)>, all_cells: &mut bool) {
    match cell {
        Some((r, c)) => {
            ui.label(format!("Cell: row {}, column {}", r + 1, c + 1));
            ui.checkbox(all_cells, "Apply fill, alignment and padding to every cell");
        }
        None => {
            ui.label("These settings apply to every cell. Click into a cell before opening this dialog to change just that cell.");
        }
    }
    ui.add_space(6.0);
}

fn cell_tab(ui: &mut Ui, table: &TableData, cell: Option<(usize, usize)>, work: &mut TableCell, all_cells: &mut bool) {
    cell_scope(ui, cell, all_cells);
    choice(ui, "cell_h", "Horizontal", &mut work.h_align, &[(CellHAlign::Left, "Left"), (CellHAlign::Center, "Center"), (CellHAlign::Right, "Right")]);
    choice(ui, "cell_v", "Vertical", &mut work.v_align, &[(CellVAlign::Top, "Top"), (CellVAlign::Middle, "Middle"), (CellVAlign::Bottom, "Bottom")]);
    number_row(ui, "Padding", &mut work.padding, 0.0..=100.0, 0.25, " pt");
    if let Some((r, c)) = cell {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Merge").strong());
        let (max_cols, max_rows) = (table.cols as usize - c, table.rows as usize - r);
        let (mut cs, mut rs) = (f64::from(work.col_span), f64::from(work.row_span));
        if number_row(ui, "Columns spanned", &mut cs, 1.0..=max_cols as f64, 0.1, "") {
            work.col_span = cs.round() as u32;
        }
        if number_row(ui, "Rows spanned", &mut rs, 1.0..=max_rows as f64, 0.1, "") {
            work.row_span = rs.round() as u32;
        }
    }
}

fn cell_fill_tab(ui: &mut Ui, cell: Option<(usize, usize)>, work: &mut TableCell, all_cells: &mut bool) {
    cell_scope(ui, cell, all_cells);
    fill_editor(ui, &mut work.fill);
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::styles::Fill;

    fn table() -> TableData {
        TableData::new(Rect::new(0.0, 0.0, 300.0, 200.0), 3, 3)
    }

    #[test]
    fn merging_stays_inside_the_table_and_clear_of_other_merges() {
        let mut t = table();
        assert!(set_span(&mut t, 0, 0, 2, 2));
        assert_eq!((t.cells[0].col_span, t.cells[0].row_span), (2, 2));
        // Overlapping the merged block is refused
        assert!(!set_span(&mut t, 1, 1, 2, 1));
        // Falling off the table is refused
        assert!(!set_span(&mut t, 2, 2, 2, 1));
        // Un-merging
        assert!(set_span(&mut t, 0, 0, 1, 1));
        assert!(set_span(&mut t, 1, 1, 2, 1));
    }

    #[test]
    fn cell_and_border_changes_apply_without_touching_the_rest() {
        let original = table();
        let mut work = original.clone();
        work.borders = TableBorders::none();
        work.col_widths = vec![50.0, 100.0, 150.0];
        let mut work_cell = TableCell::default();
        work_cell.fill = Fill::Solid(Color::from_rgb8(255, 0, 0));
        let mut shape = Shape::new(Rect::new(10.0, 10.0, 300.0, 200.0), ShapeKind::Table(original.clone()));
        if let ShapeKind::Table(t) = &mut shape.kind {
            t.cells[4].padding = 9.0;
        }
        apply_to_shape(&mut shape, &original, &work, Some((1, 1)), &TableCell::default(), &work_cell, false);
        let ShapeKind::Table(t) = &shape.kind else { unreachable!() };
        assert_eq!(t.borders, TableBorders::none());
        assert_eq!(t.col_widths, vec![50.0, 100.0, 150.0]);
        assert_eq!(shape.common.bounds.width(), 300.0);
        assert!(matches!(t.cells[4].fill, Fill::Solid(_)));
        assert_eq!(t.cells[4].padding, 9.0, "padding wasn't changed in the dialog");
        assert!(matches!(t.cells[0].fill, Fill::None));
    }
}
