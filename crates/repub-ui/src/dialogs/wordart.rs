//! WordArt dialog: the text, its font, warp shape and 3-D look. Fill and
//! outline are set in Format Shape.

use egui::Ui;
use repub_color::Color;
use repub_model::shape::ShapeKind;
use repub_model::shape::{
    BevelType, Extrusion3D, Preset3DStyle, TextEffectData, TextTransform, WordArtPreset,
};
use repub_model::styles::{FontStyle, FontWeight};
use repub_model::ObjectId;

use super::controls::{choice, color_edit, number_row, ok_cancel, row};
use super::font::family_box;
use crate::state::AppState;

/// Warp shapes the canvas draws, with their names.
pub const PRESETS: [(WordArtPreset, &str); 23] = [
    (WordArtPreset::Plain, "Plain text"),
    (WordArtPreset::ArchUp, "Arch up"),
    (WordArtPreset::ArchDown, "Arch down"),
    (WordArtPreset::Circle, "Circle"),
    (WordArtPreset::Wave, "Wave"),
    (WordArtPreset::Inflate, "Inflate"),
    (WordArtPreset::Deflate, "Deflate"),
    (WordArtPreset::SlantUp, "Slant up"),
    (WordArtPreset::SlantDown, "Slant down"),
    (WordArtPreset::FadeRight, "Fade right"),
    (WordArtPreset::FadeLeft, "Fade left"),
    (WordArtPreset::ChevronUp, "Chevron up"),
    (WordArtPreset::ChevronDown, "Chevron down"),
    (WordArtPreset::TriangleUp, "Triangle up"),
    (WordArtPreset::TriangleDown, "Triangle down"),
    (WordArtPreset::CanUp, "Can up"),
    (WordArtPreset::CanDown, "Can down"),
    (WordArtPreset::CascadeUp, "Cascade up"),
    (WordArtPreset::CascadeDown, "Cascade down"),
    (WordArtPreset::Stop, "Stop"),
    (WordArtPreset::RingInside, "Ring inside"),
    (WordArtPreset::RingOutside, "Ring outside"),
    (WordArtPreset::Button, "Button"),
];

const BEVELS: [(Option<BevelType>, &str); 13] = [
    (None, "None"),
    (Some(BevelType::Flat), "Flat"),
    (Some(BevelType::Circle), "Circle"),
    (Some(BevelType::RelaxedInset), "Relaxed inset"),
    (Some(BevelType::Cross), "Cross"),
    (Some(BevelType::Convex), "Convex"),
    (Some(BevelType::CoolSlant), "Cool slant"),
    (Some(BevelType::Angle), "Angle"),
    (Some(BevelType::SoftRound), "Soft round"),
    (Some(BevelType::ArtDeco), "Art deco"),
    (Some(BevelType::Slope), "Slope"),
    (Some(BevelType::Divot), "Divot"),
    (Some(BevelType::HardEdge), "Hard edge"),
];

const STYLES_3D: [(Preset3DStyle, &str); 10] = [
    (Preset3DStyle::None, "None"),
    (Preset3DStyle::ParallelRight, "Parallel, right"),
    (Preset3DStyle::ParallelLeft, "Parallel, left"),
    (Preset3DStyle::ParallelTop, "Parallel, top"),
    (Preset3DStyle::ParallelBottom, "Parallel, bottom"),
    (Preset3DStyle::PerspectiveRight, "Perspective, right"),
    (Preset3DStyle::PerspectiveLeft, "Perspective, left"),
    (Preset3DStyle::PerspectiveHeroicRight, "Heroic, right"),
    (Preset3DStyle::IsometricRightDown, "Isometric, right down"),
    (Preset3DStyle::OffAxis1Right, "Off axis, right"),
];

/// The WordArt dialog.
#[derive(Default)]
pub struct WordArtDialog {
    pub open: bool,
    id: Option<ObjectId>,
    work: Option<TextEffectData>,
}

impl WordArtDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open on the selected WordArt.
    pub fn open_for_selection(&mut self, state: &AppState) {
        let [shape] = state.selected_objects()[..] else {
            return;
        };
        let ShapeKind::TextEffect(data) = &shape.kind else {
            return;
        };
        self.id = Some(shape.common.id);
        self.work = Some(data.clone());
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
        egui::Window::new("Edit WordArt")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                fields(ui, work);
                result = ok_cancel(ui);
            });
        match result {
            Some(true) => {
                if let (Some(id), Some(work)) = (self.id, self.work.take()) {
                    state.begin_edit();
                    if let Some(shape) = state.current_page_mut().and_then(|p| p.get_shape_mut(id))
                    {
                        if let ShapeKind::TextEffect(data) = &mut shape.kind {
                            *data = work;
                        }
                    }
                    state.end_edit("Edit WordArt");
                }
                self.open = false;
            }
            Some(false) => self.open = false,
            None => {}
        }
    }
}

fn fields(ui: &mut Ui, data: &mut TextEffectData) {
    row(ui, "Text", |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut data.text)
                .desired_rows(2)
                .desired_width(280.0),
        )
    });
    row(ui, "Font", |ui| family_box(ui, &mut data.style.font.family));
    row(ui, "", |ui| {
        let mut bold = data.style.font.weight.weight() >= 600;
        let mut italic = data.style.font.style != FontStyle::Normal;
        if ui.checkbox(&mut bold, "Bold").changed() {
            data.style.font.weight = if bold {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            };
        }
        if ui.checkbox(&mut italic, "Italic").changed() {
            data.style.font.style = if italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };
        }
    });
    let mut preset = match data.transform {
        TextTransform::Preset(p) => p,
        _ => WordArtPreset::Plain,
    };
    let known = matches!(
        data.transform,
        TextTransform::None | TextTransform::Preset(_)
    );
    if known && choice(ui, "wordart_shape", "Shape", &mut preset, &PRESETS) {
        data.transform = if preset == WordArtPreset::Plain {
            TextTransform::None
        } else {
            TextTransform::Preset(preset)
        };
    } else if !known {
        row(ui, "Shape", |ui| ui.label("Custom path"));
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new("3-D").strong());
    let mut style = Preset3DStyle::None;
    if choice(ui, "wordart_3d_style", "Style", &mut style, &STYLES_3D) {
        data.rotation_3d = style.to_rotation();
        data.extrusion = style.to_extrusion();
    }
    let mut on = data.extrusion.is_some();
    row(ui, "Extrusion", |ui| {
        if ui.checkbox(&mut on, "").changed() {
            data.extrusion = on.then(|| Extrusion3D {
                depth: 12.0,
                ..Default::default()
            });
        }
    });
    if let Some(ext) = &mut data.extrusion {
        number_row(ui, "Depth", &mut ext.depth, 0.0..=200.0, 0.5, " pt");
        row(ui, "Color", |ui| {
            let mut own = ext.color.is_some();
            if ui.checkbox(&mut own, "Own color").changed() {
                ext.color = own.then_some(Color::from_rgb8(0x80, 0x80, 0x80));
            }
            if let Some(color) = &mut ext.color {
                color_edit(ui, color);
            }
        });
        choice(ui, "wordart_bevel", "Bevel", &mut ext.bevel, &BEVELS);
        if ext.bevel.is_some() {
            number_row(
                ui,
                "Bevel size",
                &mut ext.bevel_size,
                0.0..=50.0,
                0.5,
                " pt",
            );
        }
        number_row(
            ui,
            "Contour width",
            &mut ext.contour_width,
            0.0..=20.0,
            0.1,
            " pt",
        );
    }
    number_row(
        ui,
        "Rotate X",
        &mut data.rotation_3d.x,
        -90.0..=90.0,
        1.0,
        "°",
    );
    number_row(
        ui,
        "Rotate Y",
        &mut data.rotation_3d.y,
        -90.0..=90.0,
        1.0,
        "°",
    );
    number_row(
        ui,
        "Perspective",
        &mut data.rotation_3d.perspective,
        0.0..=1.0,
        0.01,
        "",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_the_canvas_draws_is_listed_once() {
        for (i, (preset, _)) in PRESETS.iter().enumerate() {
            assert!(PRESETS[i + 1..].iter().all(|(other, _)| other != preset));
        }
        assert_eq!(PRESETS[0].0, WordArtPreset::Plain);
    }
}
