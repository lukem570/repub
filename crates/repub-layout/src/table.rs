//! Table layout.
//!
//! A table is a [`Shape`] whose [`ShapeKind::Table`] carries a [`TableData`].
//! The table's placement comes from `shape.common.bounds`; its grid (rows,
//! columns, cell sizes and contents) comes from the [`TableData`]. Methods that
//! need the on-page rectangle take the shape (or its bounds); methods that only
//! touch the grid take the data.

use repub_geometry::Rect;
use repub_model::shape::{CellVAlign, Shape, ShapeKind, TableCell, TableData};

use repub_fonts::FontProvider;

/// Vertical alignment for cell content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Align content to top of cell
    #[default]
    Top,
    /// Center content vertically
    Center,
    /// Align content to bottom of cell
    Bottom,
}

impl From<CellVAlign> for VerticalAlignment {
    fn from(align: CellVAlign) -> Self {
        match align {
            CellVAlign::Top => VerticalAlignment::Top,
            CellVAlign::Middle => VerticalAlignment::Center,
            CellVAlign::Bottom => VerticalAlignment::Bottom,
        }
    }
}

/// Size constraints for cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellConstraints {
    /// Minimum width
    pub min_width: f64,
    /// Maximum width (None = no max)
    pub max_width: Option<f64>,
    /// Minimum height
    pub min_height: f64,
    /// Maximum height (None = no max)
    pub max_height: Option<f64>,
}

impl Default for CellConstraints {
    fn default() -> Self {
        Self {
            min_width: 20.0,
            max_width: None,
            min_height: 14.0,
            max_height: None,
        }
    }
}

impl CellConstraints {
    /// Create constraints with explicit bounds.
    pub fn new(
        min_width: f64,
        max_width: Option<f64>,
        min_height: f64,
        max_height: Option<f64>,
    ) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// Apply width constraints to a value.
    pub fn constrain_width(&self, width: f64) -> f64 {
        let clamped = width.max(self.min_width);
        match self.max_width {
            Some(max) => clamped.min(max),
            None => clamped,
        }
    }

    /// Apply height constraints to a value.
    pub fn constrain_height(&self, height: f64) -> f64 {
        let clamped = height.max(self.min_height);
        match self.max_height {
            Some(max) => clamped.min(max),
            None => clamped,
        }
    }
}

/// Result of measuring cell content for text wrapping.
#[derive(Debug, Clone)]
pub struct CellContentMeasurement {
    /// Natural width (single line, no wrapping)
    pub natural_width: f64,
    /// Required height at a given width
    pub height_at_width: f64,
    /// Whether content needs wrapping
    pub needs_wrap: bool,
}

/// Layout for a single cell.
#[derive(Debug, Clone)]
pub struct CellLayout {
    /// Row index
    pub row: usize,
    /// Column index
    pub col: usize,
    /// Cell bounds
    pub bounds: Rect,
    /// Content bounds (inside padding)
    pub content_bounds: Rect,
    /// Vertical alignment for content
    pub vertical_align: VerticalAlignment,
    /// Content offset for vertical alignment
    pub content_y_offset: f64,
}

/// Layout for a complete table.
#[derive(Debug, Clone)]
pub struct TableLayout {
    /// Table bounds
    pub bounds: Rect,
    /// Row heights
    pub row_heights: Vec<f64>,
    /// Column widths
    pub col_widths: Vec<f64>,
    /// Cell layouts
    pub cells: Vec<CellLayout>,
}

impl TableLayout {
    /// Create a new table layout.
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            row_heights: Vec::new(),
            col_widths: Vec::new(),
            cells: Vec::new(),
        }
    }

    /// Total width of all columns.
    pub fn total_width(&self) -> f64 {
        self.col_widths.iter().sum()
    }

    /// Total height of all rows.
    pub fn total_height(&self) -> f64 {
        self.row_heights.iter().sum()
    }

    /// Get cell at a specific position.
    pub fn cell_at(&self, row: usize, col: usize) -> Option<&CellLayout> {
        self.cells.iter().find(|c| c.row == row && c.col == col)
    }
}

/// Table layouter.
pub struct TableLayouter {
    /// Minimum cell width
    pub min_cell_width: f64,
    /// Maximum cell width (None = no max)
    pub max_cell_width: Option<f64>,
    /// Minimum cell height
    pub min_cell_height: f64,
    /// Maximum cell height (None = no max)
    pub max_cell_height: Option<f64>,
    /// Default cell padding
    pub default_padding: f64,
    /// Default font size for content measurement
    pub default_font_size: f64,
    /// Default line height factor
    pub default_line_height: f64,
}

impl Default for TableLayouter {
    fn default() -> Self {
        Self {
            min_cell_width: 20.0,
            max_cell_width: None,
            min_cell_height: 14.0,
            max_cell_height: None,
            default_padding: 4.0,
            default_font_size: 12.0,
            default_line_height: 1.2,
        }
    }
}

impl TableLayouter {
    /// Create a new table layouter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a table layouter with constraints.
    pub fn with_constraints(constraints: CellConstraints) -> Self {
        Self {
            min_cell_width: constraints.min_width,
            max_cell_width: constraints.max_width,
            min_cell_height: constraints.min_height,
            max_cell_height: constraints.max_height,
            ..Default::default()
        }
    }

    /// Set minimum cell width.
    pub fn set_min_width(&mut self, min: f64) {
        self.min_cell_width = min;
    }

    /// Set maximum cell width.
    pub fn set_max_width(&mut self, max: Option<f64>) {
        self.max_cell_width = max;
    }

    /// Set minimum cell height.
    pub fn set_min_height(&mut self, min: f64) {
        self.min_cell_height = min;
    }

    /// Set maximum cell height.
    pub fn set_max_height(&mut self, max: Option<f64>) {
        self.max_cell_height = max;
    }

    /// Get the current constraints.
    pub fn constraints(&self) -> CellConstraints {
        CellConstraints {
            min_width: self.min_cell_width,
            max_width: self.max_cell_width,
            min_height: self.min_cell_height,
            max_height: self.max_cell_height,
        }
    }

    /// Layout a table shape, if its kind is a table.
    pub fn layout_shape(&self, shape: &Shape) -> Option<TableLayout> {
        match &shape.kind {
            ShapeKind::Table(data) => Some(self.layout(shape.common.bounds, data)),
            _ => None,
        }
    }

    /// Layout a table from its on-page `bounds` and grid `data`.
    pub fn layout(&self, bounds: Rect, data: &TableData) -> TableLayout {
        let mut layout = TableLayout::new(bounds);

        // Use the table's pre-defined row heights and column widths
        layout.col_widths = data.col_widths.clone();
        layout.row_heights = data.row_heights.clone();

        // Create cell layouts
        self.create_cell_layouts(data, &mut layout);

        layout
    }

    /// Estimate the natural width of cell content (single line).
    fn estimate_cell_content_width(
        &self,
        cell: &TableCell,
        font_provider: &dyn FontProvider,
    ) -> f64 {
        let text = cell.content.plain_text();
        let font = font_provider.default_font();
        let width = font_provider.measure_text(&text, &font, self.default_font_size);
        width + (cell.padding * 2.0)
    }

    /// Estimate the height needed for a cell, optionally with text wrapping.
    #[allow(dead_code)]
    fn estimate_cell_height(&self, cell: &TableCell) -> f64 {
        let text = cell.content.plain_text();
        let line_count = text.lines().count().max(1);
        let line_height = self.default_font_size * self.default_line_height;

        (line_count as f64 * line_height) + (cell.padding * 2.0)
    }

    /// Measure cell content with text wrapping at a given width.
    pub fn measure_cell_content(
        &self,
        cell: &TableCell,
        available_width: f64,
        font_provider: &dyn FontProvider,
    ) -> CellContentMeasurement {
        let text = cell.content.plain_text();
        let font = font_provider.default_font();
        let content_width = available_width - (cell.padding * 2.0);
        let line_height = self.default_font_size * self.default_line_height;

        // Measure natural width (single line)
        let natural_width =
            font_provider.measure_text(&text, &font, self.default_font_size) + (cell.padding * 2.0);

        // Check if wrapping is needed
        let needs_wrap = natural_width > available_width;

        // Calculate height with wrapping
        let height_at_width = if needs_wrap && content_width > 0.0 {
            // Simple word-based wrapping estimation
            let words: Vec<&str> = text.split_whitespace().collect();
            let mut lines = 1;
            let mut current_line_width = 0.0;
            let space_width = font_provider.measure_text(" ", &font, self.default_font_size);

            for word in words {
                let word_width = font_provider.measure_text(word, &font, self.default_font_size);
                if current_line_width + word_width > content_width && current_line_width > 0.0 {
                    lines += 1;
                    current_line_width = word_width + space_width;
                } else {
                    current_line_width += word_width + space_width;
                }
            }

            (lines as f64 * line_height) + (cell.padding * 2.0)
        } else {
            // Single line height
            line_height + (cell.padding * 2.0)
        };

        CellContentMeasurement {
            natural_width,
            height_at_width,
            needs_wrap,
        }
    }

    /// Create cell layouts.
    fn create_cell_layouts(&self, data: &TableData, layout: &mut TableLayout) {
        let mut y = layout.bounds.origin.y;

        for row in 0..data.rows {
            let row_height = layout
                .row_heights
                .get(row as usize)
                .copied()
                .unwrap_or(self.min_cell_height);
            let mut x = layout.bounds.origin.x;

            for col in 0..data.cols {
                let col_width = layout
                    .col_widths
                    .get(col as usize)
                    .copied()
                    .unwrap_or(self.min_cell_width);

                let cell_bounds = Rect::new(x, y, col_width, row_height);
                let content_bounds = Rect::new(
                    x + self.default_padding,
                    y + self.default_padding,
                    (col_width - self.default_padding * 2.0).max(0.0),
                    (row_height - self.default_padding * 2.0).max(0.0),
                );

                let v_align = data
                    .cell(row, col)
                    .map(|c| VerticalAlignment::from(c.v_align))
                    .unwrap_or_default();

                layout.cells.push(CellLayout {
                    row: row as usize,
                    col: col as usize,
                    bounds: cell_bounds,
                    content_bounds,
                    vertical_align: v_align,
                    content_y_offset: 0.0,
                });

                x += col_width;
            }

            y += row_height;
        }
    }

    /// Create cell layouts with vertical alignment offsets calculated.
    fn create_cell_layouts_with_alignment(
        &self,
        data: &TableData,
        layout: &mut TableLayout,
        font_provider: &dyn FontProvider,
    ) {
        let mut y = layout.bounds.origin.y;

        for row in 0..data.rows {
            let row_height = layout
                .row_heights
                .get(row as usize)
                .copied()
                .unwrap_or(self.min_cell_height);
            let mut x = layout.bounds.origin.x;

            for col in 0..data.cols {
                let col_width = layout
                    .col_widths
                    .get(col as usize)
                    .copied()
                    .unwrap_or(self.min_cell_width);

                let cell_bounds = Rect::new(x, y, col_width, row_height);
                let content_bounds = Rect::new(
                    x + self.default_padding,
                    y + self.default_padding,
                    (col_width - self.default_padding * 2.0).max(0.0),
                    (row_height - self.default_padding * 2.0).max(0.0),
                );

                let cell = data.cell(row, col);
                let v_align = cell
                    .map(|c| VerticalAlignment::from(c.v_align))
                    .unwrap_or_default();

                let content_y_offset = if let Some(cell) = cell {
                    let measurement = self.measure_cell_content(cell, col_width, font_provider);
                    let content_height = measurement.height_at_width - (cell.padding * 2.0);
                    let available_height = content_bounds.height();

                    match v_align {
                        VerticalAlignment::Top => 0.0,
                        VerticalAlignment::Center => {
                            ((available_height - content_height) / 2.0).max(0.0)
                        }
                        VerticalAlignment::Bottom => (available_height - content_height).max(0.0),
                    }
                } else {
                    0.0
                };

                layout.cells.push(CellLayout {
                    row: row as usize,
                    col: col as usize,
                    bounds: cell_bounds,
                    content_bounds,
                    vertical_align: v_align,
                    content_y_offset,
                });

                x += col_width;
            }

            y += row_height;
        }
    }

    /// Auto-fit column widths to content.
    pub fn auto_fit_columns(&self, data: &mut TableData, font_provider: &dyn FontProvider) {
        let constraints = self.constraints();

        // Calculate max content width for each column
        let mut col_widths: Vec<f64> = vec![self.min_cell_width; data.cols as usize];

        for row in 0..data.rows {
            for col in 0..data.cols {
                if let Some(cell) = data.cell(row, col) {
                    let content_width = self.estimate_cell_content_width(cell, font_provider);
                    let constrained = constraints.constrain_width(content_width);
                    col_widths[col as usize] = col_widths[col as usize].max(constrained);
                }
            }
        }

        data.col_widths = col_widths;
    }

    /// Auto-fit row heights to content.
    pub fn auto_fit_rows(&self, data: &mut TableData, font_provider: &dyn FontProvider) {
        let constraints = self.constraints();

        // Calculate max content height for each row
        let mut row_heights: Vec<f64> = vec![self.min_cell_height; data.rows as usize];

        for row in 0..data.rows {
            for col in 0..data.cols {
                if let Some(cell) = data.cell(row, col) {
                    let col_width = data.col_widths.get(col as usize).copied().unwrap_or(100.0);
                    let measurement = self.measure_cell_content(cell, col_width, font_provider);
                    let content_height = measurement.height_at_width;
                    let constrained = constraints.constrain_height(content_height);
                    row_heights[row as usize] = row_heights[row as usize].max(constrained);
                }
            }
        }

        data.row_heights = row_heights;
    }

    /// Auto-fit both columns and rows to content.
    pub fn auto_fit(&self, data: &mut TableData, font_provider: &dyn FontProvider) {
        // Fit columns first (determines wrapping behavior)
        self.auto_fit_columns(data, font_provider);
        // Then fit rows based on wrapped content
        self.auto_fit_rows(data, font_provider);
    }

    /// Layout a table with vertical alignment calculated.
    pub fn layout_with_alignment(
        &self,
        bounds: Rect,
        data: &TableData,
        font_provider: &dyn FontProvider,
    ) -> TableLayout {
        let mut layout = TableLayout::new(bounds);

        layout.col_widths = data.col_widths.clone();
        layout.row_heights = data.row_heights.clone();

        // Create cell layouts with alignment offsets
        self.create_cell_layouts_with_alignment(data, &mut layout, font_provider);

        layout
    }

    /// Distribute rows evenly.
    pub fn distribute_rows(&self, layout: &mut TableLayout) {
        if layout.row_heights.is_empty() {
            return;
        }

        let total_height: f64 = layout.row_heights.iter().sum();
        let even_height = total_height / layout.row_heights.len() as f64;

        layout.row_heights.fill(even_height);
    }

    /// Distribute columns evenly.
    pub fn distribute_columns(&self, layout: &mut TableLayout) {
        if layout.col_widths.is_empty() {
            return;
        }

        let total_width: f64 = layout.col_widths.iter().sum();
        let even_width = total_width / layout.col_widths.len() as f64;

        layout.col_widths.fill(even_width);
    }

    /// Distribute column widths proportionally according to weights.
    pub fn distribute_proportional_columns(&self, layout: &mut TableLayout, weights: &[f64]) {
        if layout.col_widths.is_empty() || weights.is_empty() {
            return;
        }

        let total_width: f64 = layout.col_widths.iter().sum();
        let total_weight: f64 = weights.iter().sum();

        if total_weight <= 0.0 {
            return;
        }

        let constraints = self.constraints();

        for (i, col_width) in layout.col_widths.iter_mut().enumerate() {
            let weight = weights.get(i).copied().unwrap_or(1.0);
            let proportional = (weight / total_weight) * total_width;
            *col_width = constraints.constrain_width(proportional);
        }
    }

    /// Distribute row heights proportionally according to weights.
    pub fn distribute_proportional_rows(&self, layout: &mut TableLayout, weights: &[f64]) {
        if layout.row_heights.is_empty() || weights.is_empty() {
            return;
        }

        let total_height: f64 = layout.row_heights.iter().sum();
        let total_weight: f64 = weights.iter().sum();

        if total_weight <= 0.0 {
            return;
        }

        let constraints = self.constraints();

        for (i, row_height) in layout.row_heights.iter_mut().enumerate() {
            let weight = weights.get(i).copied().unwrap_or(1.0);
            let proportional = (weight / total_weight) * total_height;
            *row_height = constraints.constrain_height(proportional);
        }
    }

    /// Calculate vertical alignment offset for a cell.
    pub fn calculate_vertical_offset(
        &self,
        cell: &TableCell,
        cell_height: f64,
        content_height: f64,
    ) -> f64 {
        let available = cell_height - (cell.padding * 2.0);
        let align = VerticalAlignment::from(cell.v_align);

        match align {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Center => ((available - content_height) / 2.0).max(0.0),
            VerticalAlignment::Bottom => (available - content_height).max(0.0),
        }
    }

    /// Update cell layouts with vertical alignment offsets.
    pub fn apply_vertical_alignment(
        &self,
        data: &TableData,
        layout: &mut TableLayout,
        font_provider: &dyn FontProvider,
    ) {
        for cell_layout in &mut layout.cells {
            if let Some(cell) = data.cell(cell_layout.row as u32, cell_layout.col as u32) {
                let col_width = layout.col_widths.get(cell_layout.col).copied().unwrap_or(100.0);
                let measurement = self.measure_cell_content(cell, col_width, font_provider);
                let content_height = measurement.height_at_width - (cell.padding * 2.0);

                cell_layout.content_y_offset = self.calculate_vertical_offset(
                    cell,
                    cell_layout.bounds.height(),
                    content_height,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontdue::Font;
    use repub_fonts::FontProvider;
    use repub_model::shape::{Shape, ShapeKind, TableData};
    use std::sync::Arc;

    struct MockFontProvider {
        font: Arc<Font>,
    }

    impl MockFontProvider {
        fn new() -> Self {
            Self {
                font: Arc::new(
                    Font::from_bytes(
                        include_bytes!("../../../assets/fonts/fallback.ttf").to_vec(),
                        fontdue::FontSettings::default(),
                    )
                    .unwrap(),
                ),
            }
        }
    }

    impl FontProvider for MockFontProvider {
        fn get_font(&self, _: &str, _: u16, _: bool) -> Option<Arc<Font>> {
            Some(self.font.clone())
        }

        fn default_font(&self) -> Arc<Font> {
            self.font.clone()
        }
    }

    /// Build a table's data payload sized to `bounds` with the given grid.
    fn table_data(bounds: Rect, rows: u32, cols: u32) -> TableData {
        TableData::new(bounds, rows, cols)
    }

    #[test]
    fn test_table_layout_creation() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let data = table_data(bounds, 3, 4);

        let layouter = TableLayouter::new();
        let layout = layouter.layout(bounds, &data);

        assert_eq!(layout.row_heights.len(), 3);
        assert_eq!(layout.col_widths.len(), 4);
        assert_eq!(layout.cells.len(), 12);
    }

    #[test]
    fn test_layout_shape() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let shape = Shape::table(bounds, 2, 2);
        let layouter = TableLayouter::new();
        let layout = layouter.layout_shape(&shape).unwrap();
        assert_eq!(layout.cells.len(), 4);
        assert_eq!(layout.bounds, bounds);
    }

    #[test]
    fn test_distribute_columns() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let data = table_data(bounds, 2, 3);

        let layouter = TableLayouter::new();
        let mut layout = layouter.layout(bounds, &data);

        layouter.distribute_columns(&mut layout);

        let first_width = layout.col_widths[0];
        for width in &layout.col_widths {
            assert!((width - first_width).abs() < 0.01);
        }
    }

    #[test]
    fn test_cell_constraints() {
        let constraints = CellConstraints::new(50.0, Some(200.0), 20.0, Some(100.0));

        assert!((constraints.constrain_width(30.0) - 50.0).abs() < 0.01);
        assert!((constraints.constrain_width(100.0) - 100.0).abs() < 0.01);
        assert!((constraints.constrain_width(300.0) - 200.0).abs() < 0.01);

        assert!((constraints.constrain_height(10.0) - 20.0).abs() < 0.01);
        assert!((constraints.constrain_height(50.0) - 50.0).abs() < 0.01);
        assert!((constraints.constrain_height(150.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_constraints_no_max() {
        let constraints = CellConstraints::new(20.0, None, 14.0, None);

        assert!((constraints.constrain_width(1000.0) - 1000.0).abs() < 0.01);
        assert!((constraints.constrain_height(1000.0) - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_auto_fit_columns() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let mut data = table_data(bounds, 2, 3);

        data.set_cell_text(0, 0, "Short");
        data.set_cell_text(0, 1, "Medium text");
        data.set_cell_text(0, 2, "This is much longer text");
        data.set_cell_text(1, 0, "A");
        data.set_cell_text(1, 1, "B");
        data.set_cell_text(1, 2, "C");

        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();

        let initial_widths = data.col_widths.clone();
        assert!((initial_widths[0] - initial_widths[1]).abs() < 0.01);

        layouter.auto_fit_columns(&mut data, &font_provider);

        assert!(data.col_widths[2] > data.col_widths[0]);
        assert!(data.col_widths[1] > data.col_widths[0]);
    }

    #[test]
    fn test_auto_fit_rows() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 200.0);
        let mut data = table_data(bounds, 3, 1);

        data.set_cell_text(0, 0, "Short");
        data.set_cell_text(1, 0, "This is a much longer piece of text that will definitely need to wrap to multiple lines because it exceeds the available column width significantly");
        data.set_cell_text(2, 0, "Brief");

        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();

        layouter.auto_fit_rows(&mut data, &font_provider);

        assert!(
            data.row_heights[1] > data.row_heights[0],
            "Expected row 1 ({}) to be taller than row 0 ({})",
            data.row_heights[1],
            data.row_heights[0]
        );
    }

    #[test]
    fn test_proportional_columns() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let data = table_data(bounds, 2, 3);

        let layouter = TableLayouter::new();
        let mut layout = layouter.layout(bounds, &data);

        let weights = vec![1.0, 2.0, 1.0];
        layouter.distribute_proportional_columns(&mut layout, &weights);

        let total: f64 = layout.col_widths.iter().sum();
        assert!((total - 300.0).abs() < 0.01);

        let ratio = layout.col_widths[1] / layout.col_widths[0];
        assert!((ratio - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_proportional_rows() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let data = table_data(bounds, 4, 2);

        let layouter = TableLayouter::new();
        let mut layout = layouter.layout(bounds, &data);

        let weights = vec![1.0, 2.0, 2.0, 1.0];
        layouter.distribute_proportional_rows(&mut layout, &weights);

        let total: f64 = layout.row_heights.iter().sum();
        assert!((total - 100.0).abs() < 0.01);

        let ratio = layout.row_heights[1] / layout.row_heights[0];
        assert!((ratio - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_proportional_with_constraints() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let data = table_data(bounds, 2, 3);

        let mut layouter = TableLayouter::new();
        layouter.set_min_width(80.0);
        layouter.set_max_width(Some(150.0));

        let mut layout = layouter.layout(bounds, &data);

        let weights = vec![1.0, 10.0, 1.0];
        layouter.distribute_proportional_columns(&mut layout, &weights);

        for width in &layout.col_widths {
            assert!(*width >= 80.0);
            assert!(*width <= 150.0);
        }
    }

    #[test]
    fn test_vertical_alignment_top() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut data = table_data(bounds, 1, 1);
        data.set_cell_text(0, 0, "Test");

        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();
        let layout = layouter.layout_with_alignment(bounds, &data, &font_provider);

        let cell = layout.cell_at(0, 0).unwrap();
        assert_eq!(cell.vertical_align, VerticalAlignment::Top);
        assert!((cell.content_y_offset - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_vertical_alignment_center() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut data = table_data(bounds, 1, 1);
        data.set_cell_text(0, 0, "Test");

        if let Some(cell) = data.cell_mut(0, 0) {
            cell.v_align = CellVAlign::Middle;
        }

        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();
        let layout = layouter.layout_with_alignment(bounds, &data, &font_provider);

        let cell = layout.cell_at(0, 0).unwrap();
        assert_eq!(cell.vertical_align, VerticalAlignment::Center);
        assert!(cell.content_y_offset > 0.0);
    }

    #[test]
    fn test_vertical_alignment_bottom() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut data = table_data(bounds, 1, 1);
        data.set_cell_text(0, 0, "Test");

        if let Some(cell) = data.cell_mut(0, 0) {
            cell.v_align = CellVAlign::Bottom;
        }

        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();
        let layout = layouter.layout_with_alignment(bounds, &data, &font_provider);

        let cell = layout.cell_at(0, 0).unwrap();
        assert_eq!(cell.vertical_align, VerticalAlignment::Bottom);
        assert!(cell.content_y_offset > 0.0);
    }

    #[test]
    fn test_cell_content_measurement() {
        let cell = TableCell::with_text("Hello World");
        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();

        let wide_measurement = layouter.measure_cell_content(&cell, 500.0, &font_provider);
        assert!(!wide_measurement.needs_wrap);

        let narrow_measurement = layouter.measure_cell_content(&cell, 30.0, &font_provider);
        assert!(narrow_measurement.needs_wrap);
        assert!(narrow_measurement.height_at_width > wide_measurement.height_at_width);
    }

    #[test]
    fn test_text_wrapping_height() {
        let cell = TableCell::with_text("This is a longer text that will need to wrap to multiple lines when constrained to a narrow width");
        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();

        let wide = layouter.measure_cell_content(&cell, 500.0, &font_provider);
        let medium = layouter.measure_cell_content(&cell, 100.0, &font_provider);
        let narrow = layouter.measure_cell_content(&cell, 50.0, &font_provider);

        assert!(narrow.height_at_width >= medium.height_at_width);
        assert!(medium.height_at_width >= wide.height_at_width);
    }

    #[test]
    fn test_apply_vertical_alignment() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut data = table_data(bounds, 2, 2);

        data.set_cell_text(0, 0, "Top");
        data.set_cell_text(0, 1, "Middle");
        data.set_cell_text(1, 0, "Bottom");
        data.set_cell_text(1, 1, "Top again");

        if let Some(cell) = data.cell_mut(0, 1) {
            cell.v_align = CellVAlign::Middle;
        }
        if let Some(cell) = data.cell_mut(1, 0) {
            cell.v_align = CellVAlign::Bottom;
        }

        let font_provider = MockFontProvider::new();
        let layouter = TableLayouter::new();
        let mut layout = layouter.layout(bounds, &data);

        layouter.apply_vertical_alignment(&data, &mut layout, &font_provider);

        let top_cell = layout.cell_at(0, 0).unwrap();
        let middle_cell = layout.cell_at(0, 1).unwrap();
        let bottom_cell = layout.cell_at(1, 0).unwrap();

        assert!((top_cell.content_y_offset - 0.0).abs() < 0.01);
        assert!(middle_cell.content_y_offset > 0.0);
        assert!(bottom_cell.content_y_offset > middle_cell.content_y_offset);
    }

    #[test]
    fn test_layouter_with_constraints() {
        let constraints = CellConstraints::new(50.0, Some(150.0), 30.0, Some(80.0));
        let layouter = TableLayouter::with_constraints(constraints);

        assert!((layouter.min_cell_width - 50.0).abs() < 0.01);
        assert_eq!(layouter.max_cell_width, Some(150.0));
        assert!((layouter.min_cell_height - 30.0).abs() < 0.01);
        assert_eq!(layouter.max_cell_height, Some(80.0));
    }

    #[test]
    fn test_auto_fit_respects_constraints() {
        let bounds = Rect::new(0.0, 0.0, 300.0, 100.0);
        let mut data = table_data(bounds, 1, 2);

        data.set_cell_text(0, 0, "X");
        data.set_cell_text(0, 1, "This is an extremely long piece of text that would normally make a very wide column");

        let font_provider = MockFontProvider::new();
        let mut layouter = TableLayouter::new();
        layouter.set_min_width(40.0);
        layouter.set_max_width(Some(120.0));

        layouter.auto_fit_columns(&mut data, &font_provider);

        assert!(data.col_widths[0] >= 40.0);
        assert!(data.col_widths[1] <= 120.0);
    }

    #[test]
    fn test_cell_layout_has_vertical_align() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let data = table_data(bounds, 1, 1);

        let layouter = TableLayouter::new();
        let layout = layouter.layout(bounds, &data);

        let cell = layout.cell_at(0, 0).unwrap();
        assert_eq!(cell.vertical_align, VerticalAlignment::Top);
        assert_eq!(cell.content_y_offset, 0.0);
    }

    #[test]
    fn test_distribute_rows_evenly() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 120.0);
        let data = table_data(bounds, 4, 2);

        let layouter = TableLayouter::new();
        let mut layout = layouter.layout(bounds, &data);

        layout.row_heights[0] = 10.0;
        layout.row_heights[1] = 50.0;
        layout.row_heights[2] = 40.0;
        layout.row_heights[3] = 20.0;

        layouter.distribute_rows(&mut layout);

        let first = layout.row_heights[0];
        for height in &layout.row_heights {
            assert!((height - first).abs() < 0.01);
        }
    }

    // Keep the convenience constructor referenced so ShapeKind imports are used.
    #[test]
    fn test_layout_shape_non_table_is_none() {
        let shape = Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0));
        let layouter = TableLayouter::new();
        assert!(layouter.layout_shape(&shape).is_none());
        assert!(matches!(shape.kind, ShapeKind::AutoShape(_)));
    }
}
