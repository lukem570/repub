//! Table geometry for drawing and hit-testing.
//!
//! A table's grid is laid out to fill its bounds: row heights and column widths
//! are used as proportions and scaled to the bounds' size. Resizing a table
//! only changes its bounds, and imported tables can store sizes that don't add
//! up to their bounds, so scaling keeps what is drawn, what is clickable and
//! the selection outline in agreement.

use repub_geometry::Rect;
use repub_model::shape::{
    CellHAlign, CellVAlign, Margins, Shape, ShapeKind, TableCell, TableData, VerticalAlignment,
};
use repub_model::styles::TextAlignment;

/// Borrow the [`TableData`] payload of a table shape, if `shape` is a table.
fn table_data(shape: &Shape) -> Option<&TableData> {
    match &shape.kind {
        ShapeKind::Table(table) => Some(table),
        _ => None,
    }
}

/// Grid line positions in document points: `xs` has `cols + 1` entries from the
/// table's left edge to its right edge, `ys` has `rows + 1` from top to bottom.
#[derive(Debug, Clone, PartialEq)]
pub struct TableGrid {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
}

impl TableGrid {
    pub fn of(shape: &Shape) -> Self {
        let b = shape.common.bounds;
        let Some(table) = table_data(shape) else {
            return Self {
                xs: vec![b.origin.x],
                ys: vec![b.origin.y],
            };
        };
        let xs = edges(
            b.origin.x,
            b.size.width,
            &table.col_widths,
            table.cols as usize,
        );
        Self {
            ys: row_edges(shape, &xs),
            xs,
        }
    }

    pub fn rows(&self) -> usize {
        self.ys.len().saturating_sub(1)
    }

    pub fn cols(&self) -> usize {
        self.xs.len().saturating_sub(1)
    }

    /// Rect covering rows `row..row + row_span` and columns `col..col + col_span`
    /// (clamped to the table).
    pub fn span_rect(&self, row: usize, col: usize, row_span: usize, col_span: usize) -> Rect {
        let r1 = (row + row_span.max(1)).min(self.rows());
        let c1 = (col + col_span.max(1)).min(self.cols());
        Rect::new(
            self.xs[col],
            self.ys[row],
            self.xs[c1] - self.xs[col],
            self.ys[r1] - self.ys[row],
        )
    }
}

/// Cumulative edges for `count` tracks spread over `length` from `start`, in
/// proportion to `sizes`. Missing, mismatched or non-positive sizes fall back
/// to equal tracks.
#[allow(clippy::needless_range_loop)]
fn edges(start: f64, length: f64, sizes: &[f64], count: usize) -> Vec<f64> {
    let count = count.max(1);
    let usable = sizes.len() == count && sizes.iter().all(|s| s.is_finite() && *s >= 0.0);
    let total: f64 = if usable { sizes.iter().sum() } else { 0.0 };
    let mut out = Vec::with_capacity(count + 1);
    out.push(start);
    let mut acc = 0.0;
    for i in 0..count {
        acc += if total > 0.0 {
            sizes[i] / total
        } else {
            1.0 / count as f64
        };
        out.push(start + acc * length);
    }
    out
}

/// Row edges: rows keep the proportions stored in the table but grow when their
/// text needs more room, as Publisher's tables do. The space comes from rows
/// with room to spare; if every row is full the table extends past its bounds
/// rather than clipping or scaling the text.
fn row_edges(shape: &Shape, xs: &[f64]) -> Vec<f64> {
    let b = shape.common.bounds;
    let Some(table) = table_data(shape) else {
        return vec![b.origin.y];
    };
    let rows = table.rows.max(1) as usize;
    let base = edges(b.origin.y, b.size.height, &table.row_heights, rows);
    let mut heights: Vec<f64> = (0..rows).map(|i| base[i + 1] - base[i]).collect();

    let needed = row_min_heights(shape, xs);
    // Deficit is paid for by rows with slack; anything left over grows the table
    for _ in 0..rows + 1 {
        let deficits: Vec<f64> = (0..rows)
            .map(|i| (needed[i] - heights[i]).max(0.0))
            .collect();
        let total_deficit: f64 = deficits.iter().sum();
        if total_deficit <= 0.001 {
            break;
        }
        let slack: Vec<f64> = (0..rows)
            .map(|i| (heights[i] - needed[i]).max(0.0))
            .collect();
        let total_slack: f64 = slack.iter().sum();
        if total_slack <= 0.001 {
            for i in 0..rows {
                heights[i] += deficits[i];
            }
            break;
        }
        let take = total_deficit.min(total_slack);
        for i in 0..rows {
            if slack[i] > 0.0 {
                heights[i] -= take * slack[i] / total_slack;
            }
            if deficits[i] > 0.0 {
                heights[i] += take * deficits[i] / total_deficit;
            }
        }
    }

    let mut out = Vec::with_capacity(rows + 1);
    let mut y = b.origin.y;
    out.push(y);
    for h in heights {
        y += h.max(0.0);
        out.push(y);
    }
    out
}

/// The least height each row needs for its text, at the column widths in `xs`.
/// Empty cells don't force a row to grow.
#[allow(clippy::needless_range_loop)]
fn row_min_heights(shape: &Shape, xs: &[f64]) -> Vec<f64> {
    let b = shape.common.bounds;
    let Some(table) = table_data(shape) else {
        return Vec::new();
    };
    let rows = table.rows.max(1) as usize;
    let cols = table.cols.max(1) as usize;
    let mut needed = vec![0.0f64; rows];
    let origin_y = b.origin.y;
    for (i, cell) in table.cells.iter().enumerate() {
        let (row, col) = (i / cols, i % cols);
        if row >= rows || col >= cols || cell.plain_text().trim().is_empty() {
            continue;
        }
        let row_span = (cell.row_span.max(1) as usize).min(rows - row);
        let col_span = (cell.col_span.max(1) as usize).min(cols - col);
        let x0 = xs[col];
        let x1 = xs[(col + col_span).min(xs.len() - 1)];
        let frame = cell_text_frame(cell, Rect::new(x0, origin_y, x1 - x0, 0.0));
        let need = crate::sdf_text::content_height(&frame);
        // A merged cell's need is shared by the rows it covers
        let per_row = need / row_span.max(1) as f64;
        for r in row..row + row_span {
            needed[r] = needed[r].max(per_row);
        }
    }
    needed
}

/// A cell to draw: grid position, the cell, and its rect (spans included).
pub struct VisibleCell<'a> {
    pub row: usize,
    pub col: usize,
    pub cell: &'a TableCell,
    pub rect: Rect,
}

/// Every cell that is drawn, in row-major order. Cells covered by a merged
/// neighbour's span are left out.
pub fn visible_cells<'a>(shape: &'a Shape, grid: &TableGrid) -> Vec<VisibleCell<'a>> {
    let Some(table) = table_data(shape) else {
        return Vec::new();
    };
    let (rows, cols) = (grid.rows(), grid.cols());
    let mut covered = vec![false; rows * cols];
    let mut out = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            if covered[row * cols + col] {
                continue;
            }
            let Some(cell) = table.cells.get(row * cols + col) else {
                continue;
            };
            let row_span = (cell.row_span.max(1) as usize).min(rows - row);
            let col_span = (cell.col_span.max(1) as usize).min(cols - col);
            for r in row..row + row_span {
                for c in col..col + col_span {
                    covered[r * cols + c] = true;
                }
            }
            out.push(VisibleCell {
                row,
                col,
                cell,
                rect: grid.span_rect(row, col, row_span, col_span),
            });
        }
    }
    out
}

/// A text frame holding a cell's text, sized to the cell with its padding and
/// alignment, so cell text is laid out and drawn exactly like a text box.
pub fn cell_text_frame(cell: &TableCell, rect: Rect) -> Shape {
    let mut shape = Shape::text_frame(rect);
    let ShapeKind::TextFrame(tf) = &mut shape.kind else {
        unreachable!()
    };
    tf.content = cell.content.clone();
    // The cell's horizontal alignment applies unless the paragraph sets its own
    if cell.h_align != CellHAlign::Left {
        let alignment = match cell.h_align {
            CellHAlign::Left => TextAlignment::Left,
            CellHAlign::Center => TextAlignment::Center,
            CellHAlign::Right => TextAlignment::Right,
        };
        for p in &mut tf.content.paragraphs {
            if p.style.alignment == TextAlignment::Left {
                p.style.alignment = alignment;
            }
        }
    }
    // Rows grow to fit their text (see `row_edges`), so the text keeps the size
    // written in the document instead of being scaled down to the row height.
    tf.margins = Margins::uniform(cell.padding.max(0.0));
    tf.vertical_align = match cell.v_align {
        CellVAlign::Top => VerticalAlignment::Top,
        CellVAlign::Middle => VerticalAlignment::Center,
        CellVAlign::Bottom => VerticalAlignment::Bottom,
    };
    shape
}

/// A cell's text frame where it sits on the page: [`cell_text_frame`] for the
/// cell's `rect`, turned with a rotated table about the table's centre.
pub fn placed_cell_frame(shape: &Shape, cell: &TableCell, rect: Rect) -> Shape {
    let mut tf = cell_text_frame(cell, rect);
    let rotation = shape.common.rotation;
    if rotation != 0.0 {
        let tc = shape.common.bounds.center();
        let cc = rect.center();
        let (dx, dy) = (cc.x - tc.x, cc.y - tc.y);
        let (sin, cos) = rotation.to_radians().sin_cos();
        let (mx, my) = (tc.x + dx * cos - dy * sin, tc.y + dx * sin + dy * cos);
        tf.common.bounds.origin =
            repub_geometry::Point::new(mx - rect.size.width / 2.0, my - rect.size.height / 2.0);
        tf.common.rotation = rotation;
    }
    tf
}

/// The text frame of the visible cell at (`row`, `col`), placed on the page.
pub fn cell_frame(shape: &Shape, row: usize, col: usize) -> Option<Shape> {
    let grid = TableGrid::of(shape);
    visible_cells(shape, &grid)
        .into_iter()
        .find(|c| c.row == row && c.col == col)
        .map(|c| placed_cell_frame(shape, c.cell, c.rect))
}

/// The visible cell (row, col) under page point `p`, if any.
pub fn cell_at(shape: &Shape, p: repub_geometry::Point) -> Option<(usize, usize)> {
    // Into the table's unturned frame
    let c = shape.common.bounds.center();
    let (sin, cos) = (-shape.common.rotation).to_radians().sin_cos();
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let p = repub_geometry::Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos);
    let grid = TableGrid::of(shape);
    visible_cells(shape, &grid)
        .into_iter()
        .find(|v| v.rect.contains(p))
        .map(|v| (v.row, v.col))
}

/// The visible cell after (`forward`) or before (`row`, `col`) in reading order.
pub fn neighbour_cell(
    shape: &Shape,
    row: usize,
    col: usize,
    forward: bool,
) -> Option<(usize, usize)> {
    let grid = TableGrid::of(shape);
    let cells: Vec<(usize, usize)> = visible_cells(shape, &grid)
        .iter()
        .map(|c| (c.row, c.col))
        .collect();
    let i = cells.iter().position(|&rc| rc == (row, col))?;
    if forward {
        cells.get(i + 1).copied()
    } else {
        i.checked_sub(1).map(|j| cells[j])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Borrow the table payload of a table shape in tests.
    fn td(shape: &mut Shape) -> &mut TableData {
        match &mut shape.kind {
            ShapeKind::Table(table) => table,
            _ => unreachable!(),
        }
    }

    #[test]
    fn grid_fills_the_bounds_in_proportion() {
        let mut t = Shape::table(Rect::new(10.0, 20.0, 300.0, 100.0), 2, 3);
        td(&mut t).col_widths = vec![1.0, 2.0, 1.0];
        td(&mut t).row_heights = vec![30.0, 10.0];
        let g = TableGrid::of(&t);
        assert_eq!(g.xs, vec![10.0, 85.0, 235.0, 310.0]);
        assert_eq!(g.ys, vec![20.0, 95.0, 120.0]);
    }

    #[test]
    fn grid_follows_a_resize() {
        let mut t = Shape::table(Rect::new(0.0, 0.0, 100.0, 100.0), 2, 2);
        t.common.bounds.size.width = 50.0;
        let g = TableGrid::of(&t);
        assert_eq!(g.xs, vec![0.0, 25.0, 50.0]);
    }

    #[test]
    fn bad_sizes_fall_back_to_equal_tracks() {
        let mut t = Shape::table(Rect::new(0.0, 0.0, 90.0, 60.0), 2, 3);
        td(&mut t).col_widths = vec![10.0]; // too few
        td(&mut t).row_heights = vec![0.0, 0.0]; // no total
        let g = TableGrid::of(&t);
        assert_eq!(g.xs, vec![0.0, 30.0, 60.0, 90.0]);
        assert_eq!(g.ys, vec![0.0, 30.0, 60.0]);
    }

    #[test]
    fn rows_grow_to_fit_their_text() {
        use repub_model::text::InlineElement;

        let mut t = Shape::table(Rect::new(0.0, 0.0, 300.0, 100.0), 2, 1);
        td(&mut t).row_heights = vec![82.0, 18.0];
        let cell = td(&mut t).cell_mut(1, 0).unwrap();
        cell.set_text("2");
        // 28pt, like a page number in an imported Publisher table
        let p = &mut cell.content.paragraphs[0];
        p.style.default_char_style.font_size = 28.0;
        for e in &mut p.elements {
            if let InlineElement::Text(run) = e {
                run.style.font_size = 28.0;
            }
        }

        let g = TableGrid::of(&t);
        // The footer row grows past its stored 18pt to hold the text...
        assert!(g.ys[2] - g.ys[1] > 18.0);
        // ...and takes the space from the empty content row, keeping the bounds
        assert!((g.ys[2] - g.ys[0] - 100.0).abs() < 1e-6);
    }

    #[test]
    fn empty_cells_do_not_grow_rows() {
        let mut t = Shape::table(Rect::new(0.0, 0.0, 100.0, 100.0), 2, 1);
        td(&mut t).row_heights = vec![90.0, 10.0];
        let g = TableGrid::of(&t);
        assert_eq!(g.ys, vec![0.0, 90.0, 100.0]);
    }

    #[test]
    fn merged_cells_cover_their_neighbours() {
        let mut t = Shape::table(Rect::new(0.0, 0.0, 90.0, 60.0), 2, 3);
        let c = td(&mut t).cell_mut(0, 0).unwrap();
        c.col_span = 2;
        c.row_span = 2;
        let g = TableGrid::of(&t);
        let cells = visible_cells(&t, &g);
        let positions: Vec<_> = cells.iter().map(|c| (c.row, c.col)).collect();
        assert_eq!(positions, vec![(0, 0), (0, 2), (1, 2)]);
        assert_eq!(cells[0].rect, Rect::new(0.0, 0.0, 60.0, 60.0));
    }

    #[test]
    fn oversized_spans_are_clamped() {
        let mut t = Shape::table(Rect::new(0.0, 0.0, 60.0, 60.0), 2, 2);
        td(&mut t).cell_mut(1, 1).unwrap().col_span = 5;
        let g = TableGrid::of(&t);
        let last = visible_cells(&t, &g).pop().unwrap();
        assert_eq!(last.rect, Rect::new(30.0, 30.0, 30.0, 30.0));
    }
}
