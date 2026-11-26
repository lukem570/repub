//! The Shapes gallery (Insert/Home › Shapes): Office's preset shapes in
//! Publisher's categories, each drawn as a small preview by the preset engine.

use egui::{Rect, Sense, Ui, Vec2};
use repub_escher::{preset, PresetExt};
use repub_geometry::Rect as DocRect;
use repub_model::geometry::{AutoShape, PresetGeometry, ShapeGeometry};
use repub_model::shape::{Shape, ShapeKind};
use repub_model::styles::{Fill, Stroke};

use crate::canvas::{shape_object_shapes, DocToScreen};
use crate::state::AppState;
use crate::theme::colors;
use crate::tools::Tool;

/// Gallery sections: title and Escher shape types, in Publisher's order.
pub const CATEGORIES: &[(&str, &[u16])] = &[
    (
        "Basic Shapes",
        &[
            1, 2, 3, 5, 6, 7, 8, 4, 10, 9, 56, 11, 23, 57, 95, 65, 96, 74, 73, 183, 184, 21, 22,
            16, 84, 19, 85, 86, 87, 88, 185, 186,
        ],
    ),
    (
        "Block Arrows",
        &[
            13, 66, 68, 67, 69, 70, 76, 182, 89, 90, 91, 101, 102, 103, 104, 105, 99, 93, 94, 15,
            55, 78, 77, 79, 80, 81, 82, 83,
        ],
    ),
    (
        "Flowchart",
        &[
            109, 176, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 177, 121, 122, 123,
            124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
        ],
    ),
    (
        "Stars and Banners",
        &[
            71, 72, 187, 12, 58, 59, 92, 60, 53, 54, 107, 108, 97, 98, 64, 188,
        ],
    ),
    (
        "Callouts",
        &[
            61, 62, 63, 106, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52,
        ],
    ),
];

/// The name shown for shape type `spt` (Publisher's wording where it differs
/// from the template name).
pub fn display_name(spt: u16) -> &'static str {
    match spt {
        2 => "Rounded Rectangle",
        3 => "Oval",
        5 => "Isosceles Triangle",
        6 => "Right Triangle",
        12 => "5-Point Star",
        13 => "Right Arrow",
        15 => "Pentagon Arrow",
        19 => "Arc",
        53 => "Down Ribbon",
        54 => "Up Ribbon",
        57 => "No Symbol",
        58 => "8-Point Star",
        59 => "16-Point Star",
        60 => "32-Point Star",
        61 => "Rectangular Callout",
        62 => "Rounded Rectangular Callout",
        63 => "Oval Callout",
        65 => "Folded Corner",
        71 => "Explosion 1",
        72 => "Explosion 2",
        92 => "24-Point Star",
        95 => "Block Arc",
        96 => "Smiley Face",
        106 => "Cloud Callout",
        107 => "Curved Down Ribbon",
        108 => "Curved Up Ribbon",
        187 => "4-Point Star",
        _ => preset(spt).map_or("Shape", |d| d.name),
    }
}

/// Build a preset auto shape (Escher shape type `spt`) at `bounds`, with its
/// outline computed by the preset engine.
pub(crate) fn preset_shape(bounds: DocRect, spt: u16) -> Option<Shape> {
    let geometry = PresetGeometry::new(spt);
    let path = geometry.path(bounds)?;
    let auto = AutoShape {
        geometry: ShapeGeometry::Path(path),
        preset: Some(geometry),
    };
    Some(Shape::new(bounds, ShapeKind::AutoShape(auto)))
}

/// A preset as inserted: its default look for previews and new shapes.
fn sample(spt: u16, bounds: DocRect) -> Option<Shape> {
    let mut shape = preset_shape(bounds, spt)?;
    if !matches!(shape.fill, Fill::None) {
        shape.fill = Fill::Solid(repub_color::Color::from_rgb8(0xCD, 0xE6, 0xDE));
    }
    shape.line = Some(Stroke::new(
        repub_color::Color::from_rgb8(0x44, 0x44, 0x44),
        1.0,
    ));
    Some(shape)
}

/// Show the gallery; picking a shape selects its drawing tool. Returns true
/// when a shape was picked (the menu can close).
pub fn show(ui: &mut Ui, state: &mut AppState) -> bool {
    let mut picked = false;
    let cell = Vec2::splat(26.0);
    let per_row = 10;
    ui.set_max_width(cell.x * per_row as f32 + 16.0);
    for (title, shapes) in CATEGORIES {
        ui.label(
            egui::RichText::new(*title)
                .strong()
                .size(11.0)
                .color(colors::text_secondary()),
        );
        for row in shapes.chunks(per_row) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for &spt in row {
                    let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
                    let selected = state.current_tool == Tool::Preset(spt);
                    if selected || response.hovered() {
                        let bg = if selected {
                            colors::selected()
                        } else {
                            colors::hover()
                        };
                        ui.painter().rect_filled(rect, 3.0, bg);
                    }
                    paint_preview(ui, spt, rect.shrink(4.0));
                    if response.on_hover_text(display_name(spt)).clicked() {
                        state.current_tool = Tool::Preset(spt);
                        picked = true;
                    }
                }
            });
        }
        ui.add_space(4.0);
    }
    picked
}

/// Draw shape `spt` filling `rect` (screen).
pub fn paint_preview(ui: &Ui, spt: u16, rect: Rect) {
    let bounds = DocRect::new(0.0, 0.0, rect.width() as f64, rect.height() as f64);
    let Some(mut shape) = sample(spt, bounds) else {
        return;
    };
    shape.line = Some(Stroke::new(
        repub_color::Color::from_rgb8(0x44, 0x44, 0x44),
        1.0,
    ));
    let map = DocToScreen {
        origin: rect.min,
        scale: 1.0,
    };
    ui.painter()
        .with_clip_rect(rect.expand(1.0))
        .extend(shape_object_shapes(&shape, map, 1.0, 0.0));
}
