//! Selection pane (Home › Arrange › Selection Pane, as in Publisher): the
//! current page's objects, front first, to select and to show or hide. When a
//! page has more than one layer, each layer gets a row to show/hide and lock it.

use egui::{Color32, Sense, Vec2};
use repub_model::ObjectId;
use repub_model::geometry::ShapeGeometry;
use repub_model::shape::{Shape, ShapeKind};

use crate::icons::Icon;
use crate::state::AppState;
use crate::theme::colors;

/// What an object is called in the pane: its name, else its kind and a number
/// counting objects of that kind on the page from the back.
pub fn object_labels(objects: &[&Shape]) -> Vec<String> {
    let mut counts = std::collections::HashMap::new();
    objects
        .iter()
        .map(|obj| {
            let kind = kind_name(obj);
            let n = counts.entry(kind).or_insert(0);
            *n += 1;
            obj.common.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| format!("{kind} {n}"))
        })
        .collect()
}

fn kind_name(obj: &Shape) -> &'static str {
    match &obj.kind {
        ShapeKind::TextFrame(_) => "Text Box",
        ShapeKind::Picture(_) => "Picture",
        ShapeKind::AutoShape(s) => match s.geometry {
            ShapeGeometry::Rectangle { .. } => "Rectangle",
            ShapeGeometry::Ellipse => "Oval",
            ShapeGeometry::Star { .. } => "Star",
            _ => "Shape",
        },
        ShapeKind::Line(_) => "Line",
        ShapeKind::Group(_) => "Group",
        ShapeKind::Table(_) => "Table",
        ShapeKind::TextEffect(_) => "WordArt",
    }
}

/// A pane edit, applied after drawing (as one undo step).
enum Edit {
    Select(ObjectId, bool),
    ObjectVisible(usize, usize, bool),
    ObjectLocked(usize, usize, bool),
    RenameObject(usize, usize, String),
    LayerVisible(usize, bool),
    LayerLocked(usize, bool),
    LayerPrintable(usize, bool),
    RenameLayer(usize, String),
    AddLayer,
    RemoveLayer(usize),
    MoveLayer(usize, bool),
    MoveSelectionToLayer(usize),
    AllVisible(bool),
}

/// What is being renamed in the pane.
#[derive(Clone, Copy, PartialEq)]
enum Renaming {
    Object(ObjectId),
    Layer(usize),
}

/// An inline name field for the thing being renamed. Returns the new name when committed
/// (Enter or clicking away); Escape cancels.
fn rename_field(ui: &mut egui::Ui, target: Renaming) -> Option<String> {
    let id = egui::Id::new("selection_rename");
    let (current, mut text): (Option<Renaming>, String) = ui.data(|d| d.get_temp(id)).unwrap_or((None, String::new()));
    debug_assert!(current == Some(target));
    let response = ui.add(egui::TextEdit::singleline(&mut text).desired_width(ui.available_width() - 60.0));
    if !response.has_focus() && !response.lost_focus() {
        response.request_focus();
    }
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let result = if escape {
        ui.data_mut(|d| d.remove_temp::<(Option<Renaming>, String)>(id));
        None
    } else if response.lost_focus() {
        ui.data_mut(|d| d.remove_temp::<(Option<Renaming>, String)>(id));
        Some(text)
    } else {
        ui.data_mut(|d| d.insert_temp(id, (current, text)));
        None
    };
    result
}

fn start_rename(ui: &egui::Ui, target: Renaming, current: &str) {
    ui.data_mut(|d| d.insert_temp(egui::Id::new("selection_rename"), (Some(target), current.to_string())));
}

fn renaming(ui: &egui::Ui) -> Option<Renaming> {
    ui.data(|d| d.get_temp::<(Option<Renaming>, String)>(egui::Id::new("selection_rename"))).and_then(|(t, _)| t)
}

/// A small icon toggle; returns true when clicked.
fn icon_toggle(ui: &mut egui::Ui, icon: Icon, tip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 3.0, colors::hover());
    }
    icon.paint(ui, rect.shrink(3.0), colors::text_secondary());
    response.on_hover_text(tip).clicked()
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Selection").size(17.0).color(colors::accent()));
    });
    ui.add_space(4.0);

    let Some(page) = state.current_page() else { return };
    let several_layers = true;
    let all: Vec<&Shape> = page.layers().iter().flat_map(|l| l.shapes()).collect();
    let labels = object_labels(&all);
    let mut edit = None;

    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if ui.button("Show All").clicked() {
            edit = Some(Edit::AllVisible(true));
        }
        if ui.button("Hide All").clicked() {
            edit = Some(Edit::AllVisible(false));
        }
        if ui.button("New Layer").clicked() {
            edit = Some(Edit::AddLayer);
        }
    });
    ui.add_space(4.0);

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // Front first: layers from the top, objects from the front
        let mut first_label = 0;
        let starts: Vec<usize> = page
            .layers()
            .iter()
            .map(|l| {
                let start = first_label;
                first_label += l.shapes().len();
                start
            })
            .collect();
        for (li, layer) in page.layers().iter().enumerate().rev() {
            {
                let layer_count = page.layers().len();
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let name = if layer.name.is_empty() { format!("Layer {}", li + 1) } else { layer.name.clone() };
                    if renaming(ui) == Some(Renaming::Layer(li)) {
                        if let Some(new_name) = rename_field(ui, Renaming::Layer(li)) {
                            edit = Some(Edit::RenameLayer(li, new_name));
                        }
                    } else {
                        let label = ui.add(
                            egui::Label::new(egui::RichText::new(&name).strong().color(if layer.visible { colors::text_primary() } else { colors::text_disabled() }))
                                .sense(Sense::click()),
                        );
                        if label.double_clicked() {
                            start_rename(ui, Renaming::Layer(li), &name);
                        }
                        label.on_hover_text("Double-click to rename");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        let menu = ui.menu_button("…", |ui| {
                            if ui.add_enabled(li + 1 < layer_count, egui::Button::new("Move layer up")).clicked() {
                                edit = Some(Edit::MoveLayer(li, true));
                                ui.close_menu();
                            }
                            if ui.add_enabled(li > 0, egui::Button::new("Move layer down")).clicked() {
                                edit = Some(Edit::MoveLayer(li, false));
                                ui.close_menu();
                            }
                            if ui.add_enabled(!state.selection.is_empty(), egui::Button::new("Move selection here")).clicked() {
                                edit = Some(Edit::MoveSelectionToLayer(li));
                                ui.close_menu();
                            }
                            if ui.add(egui::Button::new(if layer.printable { "Don't print this layer" } else { "Print this layer" })).clicked() {
                                edit = Some(Edit::LayerPrintable(li, !layer.printable));
                                ui.close_menu();
                            }
                            if ui.add_enabled(layer_count > 1, egui::Button::new("Delete layer")).clicked() {
                                edit = Some(Edit::RemoveLayer(li));
                                ui.close_menu();
                            }
                        });
                        menu.response.on_hover_text("Layer options");
                        let (icon, tip) = if layer.visible { (Icon::Eye, "Hide this layer") } else { (Icon::EyeOff, "Show this layer") };
                        if icon_toggle(ui, icon, tip) {
                            edit = Some(Edit::LayerVisible(li, !layer.visible));
                        }
                        let (icon, tip) = if layer.locked { (Icon::LockClosed, "Unlock this layer") } else { (Icon::LockOpen, "Lock this layer") };
                        if icon_toggle(ui, icon, tip) {
                            edit = Some(Edit::LayerLocked(li, !layer.locked));
                        }
                    });
                });
            }
            for (oi, obj) in layer.shapes().iter().enumerate().rev() {
                let id = obj.id();
                let visible = obj.common.visible && layer.visible;
                let selected = state.selection.contains(&id);
                let row = ui.horizontal(|ui| {
                    ui.set_min_height(24.0);
                    ui.add_space(if several_layers { 20.0 } else { 8.0 });
                    if renaming(ui) == Some(Renaming::Object(id)) {
                        if let Some(new_name) = rename_field(ui, Renaming::Object(id)) {
                            edit = Some(Edit::RenameObject(li, oi, new_name));
                        }
                    } else {
                        let text = egui::RichText::new(&labels[starts[li] + oi]).color(if visible { colors::text_primary() } else { colors::text_disabled() });
                        let label = ui.add(egui::Label::new(text).sense(Sense::click()).truncate());
                        if label.clicked() {
                            let extend = ui.input(|i| i.modifiers.shift || i.modifiers.command);
                            edit = Some(Edit::Select(id, extend));
                        }
                        if label.double_clicked() {
                            start_rename(ui, Renaming::Object(id), &labels[starts[li] + oi]);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        let on = obj.common.visible;
                        let (icon, tip) = if on { (Icon::Eye, "Hide") } else { (Icon::EyeOff, "Show") };
                        if icon_toggle(ui, icon, tip) {
                            edit = Some(Edit::ObjectVisible(li, oi, !on));
                        }
                        let locked = obj.common.locked;
                        let (icon, tip) = if locked { (Icon::LockClosed, "Unlock") } else { (Icon::LockOpen, "Lock") };
                        if icon_toggle(ui, icon, tip) {
                            edit = Some(Edit::ObjectLocked(li, oi, !locked));
                        }
                    });
                });
                if selected {
                    let rect = row.response.rect.expand2(Vec2::new(0.0, 1.0));
                    ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0x07, 0x75, 0x68, 40));
                }
            }
        }
        if all.is_empty() {
            ui.add_space(12.0);
            ui.vertical_centered(|ui| ui.label(egui::RichText::new("No objects on this page").color(colors::text_disabled())));
        }
    });

    apply(state, edit);
}

fn apply(state: &mut AppState, edit: Option<Edit>) {
    let Some(edit) = edit else { return };
    let label = match &edit {
        Edit::Select(id, extend) => {
            if *extend {
                if let Some(i) = state.selection.iter().position(|s| s == id) {
                    state.selection.remove(i);
                } else {
                    state.selection.push(*id);
                }
            } else {
                state.selection = vec![*id];
            }
            return;
        }
        Edit::ObjectVisible(_, _, on) | Edit::AllVisible(on) => if *on { "Show Object" } else { "Hide Object" },
        Edit::ObjectLocked(_, _, on) => if *on { "Lock Object" } else { "Unlock Object" },
        Edit::RenameObject(..) => "Rename Object",
        Edit::LayerVisible(_, on) => if *on { "Show Layer" } else { "Hide Layer" },
        Edit::LayerLocked(_, on) => if *on { "Lock Layer" } else { "Unlock Layer" },
        Edit::LayerPrintable(..) => "Layer Printing",
        Edit::RenameLayer(..) => "Rename Layer",
        Edit::AddLayer => "New Layer",
        Edit::RemoveLayer(_) => "Delete Layer",
        Edit::MoveLayer(..) => "Move Layer",
        Edit::MoveSelectionToLayer(_) => "Move to Layer",
    };
    state.begin_edit();
    let selection = state.selection.clone();
    if let Some(page) = state.current_page_mut() {
        apply_layer_edit(page, &edit, &selection);
        let layers = page.layers_mut();
        match edit {
            Edit::ObjectVisible(li, oi, on) => {
                if let Some(obj) = layers.get_mut(li).and_then(|l| l.shapes_mut().get_mut(oi)) {
                    obj.common.visible = on;
                }
            }
            Edit::AllVisible(on) => {
                for obj in layers.iter_mut().flat_map(|l| l.shapes_mut().iter_mut()) {
                    obj.common.visible = on;
                }
            }
            Edit::LayerVisible(li, on) => {
                if let Some(layer) = layers.get_mut(li) {
                    layer.visible = on;
                }
            }
            Edit::LayerLocked(li, on) => {
                if let Some(layer) = layers.get_mut(li) {
                    layer.locked = on;
                }
            }
            Edit::ObjectLocked(li, oi, on) => {
                if let Some(obj) = layers.get_mut(li).and_then(|l| l.shapes_mut().get_mut(oi)) {
                    obj.common.locked = on;
                }
            }
            Edit::RenameObject(li, oi, name) => {
                if let Some(obj) = layers.get_mut(li).and_then(|l| l.shapes_mut().get_mut(oi)) {
                    obj.common.name = (!name.is_empty()).then_some(name);
                }
            }
            Edit::LayerPrintable(li, on) => {
                if let Some(layer) = layers.get_mut(li) {
                    layer.printable = on;
                }
            }
            Edit::RenameLayer(li, name) => {
                if let Some(layer) = layers.get_mut(li) {
                    layer.name = name;
                }
            }
            Edit::Select(..) | Edit::AddLayer | Edit::RemoveLayer(_) | Edit::MoveLayer(..) | Edit::MoveSelectionToLayer(_) => {}
        }
    }
    state.end_edit(label);
}

/// The edits that restructure a page's layers.
fn apply_layer_edit(page: &mut repub_model::Page, edit: &Edit, selection: &[ObjectId]) {
    match edit {
        Edit::AddLayer => {
            let n = page.layers().len() + 1;
            page.add_layer(format!("Layer {n}"));
        }
        Edit::RemoveLayer(li) => {
            let count = page.layers().len();
            if count <= 1 || *li >= count {
                return;
            }
            // Its objects go to the layer below (or above, for the bottom one)
            let target = if *li > 0 { li - 1 } else { 1 };
            let moved: Vec<_> = page.layers()[*li].shapes().to_vec();
            let id = page.layers()[*li].id;
            for shape in moved {
                page.layers_mut()[target].add_shape(shape);
            }
            page.remove_layer(id);
        }
        Edit::MoveLayer(li, up) => {
            let other = if *up { li + 1 } else { li.wrapping_sub(1) };
            if *li < page.layers().len() && other < page.layers().len() {
                page.layers_mut().swap(*li, other);
            }
        }
        Edit::MoveSelectionToLayer(li) => {
            if *li >= page.layers().len() {
                return;
            }
            let mut moving = Vec::new();
            for (index, layer) in page.layers_mut().iter_mut().enumerate() {
                if index == *li {
                    continue;
                }
                for id in selection {
                    if let Some(shape) = layer.remove_shape(*id) {
                        moving.push(shape);
                    }
                }
            }
            for shape in moving {
                page.layers_mut()[*li].add_shape(shape);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;
    use repub_model::{Document, PageOrientation, PageSize};

    fn page() -> repub_model::Page {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let page = &mut doc.pages_mut()[0];
        page.add_shape(Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0)));
        page.add_shape(Shape::ellipse(Rect::new(0.0, 0.0, 10.0, 10.0)));
        page.clone()
    }

    #[test]
    fn layers_can_be_added_moved_and_removed_keeping_objects() {
        let mut page = page();
        apply_layer_edit(&mut page, &Edit::AddLayer, &[]);
        assert_eq!(page.layers().len(), 2);
        let first = page.layers()[0].id;
        apply_layer_edit(&mut page, &Edit::MoveLayer(0, true), &[]);
        assert_eq!(page.layers()[1].id, first);
        // The two shapes live on the original layer, now on top; deleting it keeps them
        apply_layer_edit(&mut page, &Edit::RemoveLayer(1), &[]);
        assert_eq!(page.layers().len(), 1);
        assert_eq!(page.layers()[0].shapes().len(), 2);
    }

    #[test]
    fn selection_moves_to_another_layer() {
        let mut page = page();
        apply_layer_edit(&mut page, &Edit::AddLayer, &[]);
        let id = page.layers()[0].shapes()[0].id();
        apply_layer_edit(&mut page, &Edit::MoveSelectionToLayer(1), &[id]);
        assert_eq!(page.layers()[0].shapes().len(), 1);
        assert_eq!(page.layers()[1].shapes()[0].id(), id);
    }
}
