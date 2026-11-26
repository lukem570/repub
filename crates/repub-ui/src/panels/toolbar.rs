//! Office-style ribbon.
//!
//! Layout follows Publisher:
//! a tab row (File, Home, Insert, Page Design, Review, View, Help, plus a contextual
//! Format tab when an object is selected) above a command body made of groups.
//! Each group has a fixed content height with its name on a shared baseline at
//! the bottom. All icons are SVGs from `crate::icons`.

use egui::{Align, Align2, Color32, FontId, Id, Layout, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::app::AppAction;
use crate::icons::Icon;
use crate::state::AppState;
use crate::text_format::{self, Toggle};
use crate::theme::{colors, fonts, metrics, shortcut, super_tooltip};
use crate::tools::Tool;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::styles::TextAlignment;

const TAB_ROW_HEIGHT: f32 = 28.0;
/// Height of the button area inside a group.
const CONTENT_HEIGHT: f32 = 68.0;
/// Height of the group name row under the buttons.
const LABEL_HEIGHT: f32 = 17.0;
const BODY_PADDING: f32 = 4.0;
const SMALL_HEIGHT: f32 = 22.0;

/// Contextual tab colors (Publisher 2013+ Drawing / Text Box / Picture Tools).
const SHAPE_TAB_COLOR: Color32 = Color32::from_rgb(0xC5, 0x5A, 0x11);
const TEXT_TAB_COLOR: Color32 = Color32::from_rgb(0x7B, 0x4F, 0xA6);
const PICTURE_TAB_COLOR: Color32 = Color32::from_rgb(0xA0, 0x3A, 0x5E);
const TABLE_TAB_COLOR: Color32 = Color32::from_rgb(0x2E, 0x75, 0xB6);

fn label_font() -> FontId {
    FontId::proportional(12.0)
}

/// Ribbon tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonTab {
    #[default]
    Home,
    Insert,
    PageDesign,
    Review,
    View,
    Help,
    /// Contextual tab, shown only while an object of the matching kind is selected.
    Format,
}

/// What kind of object the selection holds; picks the contextual Format tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionKind {
    None,
    Shape,
    Text,
    Picture,
    Table,
    /// Several kinds, or kinds without their own tools (groups, WordArt)
    Mixed,
}

impl SelectionKind {
    fn of(state: &AppState) -> Self {
        let objects = state.selected_objects();
        let mut kind = SelectionKind::None;
        for obj in objects {
            let k = match &obj.kind {
                ShapeKind::AutoShape(_) | ShapeKind::Line(_) => SelectionKind::Shape,
                ShapeKind::TextFrame(_) => SelectionKind::Text,
                ShapeKind::Picture(_) => SelectionKind::Picture,
                ShapeKind::Table(_) => SelectionKind::Table,
                _ => SelectionKind::Mixed,
            };
            kind = match kind {
                SelectionKind::None => k,
                same if same == k => same,
                _ => SelectionKind::Mixed,
            };
        }
        kind
    }

    /// Contextual tab title and color.
    fn format_tab(self) -> Option<(&'static str, Color32)> {
        match self {
            SelectionKind::Shape => Some(("Shape Format", SHAPE_TAB_COLOR)),
            SelectionKind::Text => Some(("Text Box Format", TEXT_TAB_COLOR)),
            SelectionKind::Picture => Some(("Picture Format", PICTURE_TAB_COLOR)),
            SelectionKind::Table => Some(("Table Layout", TABLE_TAB_COLOR)),
            SelectionKind::Mixed => Some(("Format", SHAPE_TAB_COLOR)),
            SelectionKind::None => None,
        }
    }
}

/// App state the ribbon reads but does not own.
pub struct RibbonContext {
    pub clipboard_empty: bool,
    pub pages_pane_visible: bool,
    pub selection_pane_visible: bool,
}

/// Main ribbon.
pub struct Toolbar {
    current_tab: RibbonTab,
    /// Only the tab row is shown when collapsed.
    collapsed: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of clicking a button that may have a dropdown part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    None,
    /// The main part (or the whole button, for plain buttons).
    Main,
    /// The ▾ part of a split button, or anywhere on a dropdown-only button.
    Menu,
}

/// Button description shared by large and small ribbon buttons.
#[derive(Clone, Copy)]
struct Btn<'a> {
    icon: Icon,
    label: &'a str,
    enabled: bool,
    checked: bool,
    /// Has a ▾ menu.
    dropdown: bool,
    /// The ▾ is a separate part (split button); otherwise the whole button opens the menu.
    split: bool,
    tip: &'a str,
    keys: Option<&'a str>,
}

impl<'a> Btn<'a> {
    fn new(icon: Icon, label: &'a str, tip: &'a str) -> Self {
        Self {
            icon,
            label,
            enabled: true,
            checked: false,
            dropdown: false,
            split: false,
            tip,
            keys: None,
        }
    }
    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    fn menu(mut self) -> Self {
        self.dropdown = true;
        self
    }
    fn split(mut self) -> Self {
        self.dropdown = true;
        self.split = true;
        self
    }
    fn keys(mut self, keys: &'a str) -> Self {
        self.keys = Some(keys);
        self
    }
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            current_tab: RibbonTab::Home,
            collapsed: false,
        }
    }

    /// Show the ribbon and return the actions triggered this frame.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        ctx: &RibbonContext,
    ) -> Vec<AppAction> {
        let mut actions = Vec::new();
        let selection = SelectionKind::of(state);
        let format_tab = selection.format_tab();
        if self.current_tab == RibbonTab::Format && format_tab.is_none() {
            self.current_tab = RibbonTab::Home;
        }

        self.tab_row(ui, format_tab, &mut actions);

        if !self.collapsed {
            let body_top = ui.cursor().top();
            ui.add_space(BODY_PADDING);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 1.0);
                match self.current_tab {
                    RibbonTab::Home => self.home_tab(ui, state, ctx, selection, &mut actions),
                    RibbonTab::Insert => self.insert_tab(ui, state, &mut actions),
                    RibbonTab::PageDesign => self.page_design_tab(ui, state, &mut actions),
                    RibbonTab::Review => self.review_tab(ui, state, &mut actions),
                    RibbonTab::View => self.view_tab(ui, state, ctx, &mut actions),
                    RibbonTab::Help => self.help_tab(ui, &mut actions),
                    RibbonTab::Format => self.format_tab(ui, state, selection, &mut actions),
                }
            });
            ui.add_space(BODY_PADDING);

            // Collapse chevron in the body's bottom-right corner
            let body = Rect::from_min_max(
                egui::pos2(ui.max_rect().left(), body_top),
                egui::pos2(ui.max_rect().right(), ui.cursor().top()),
            );
            let chevron = Rect::from_min_size(
                body.right_bottom() - Vec2::new(22.0, 20.0),
                Vec2::splat(16.0),
            );
            let response = ui.interact(chevron, ui.id().with("collapse_ribbon"), Sense::click());
            if response.hovered() {
                ui.painter()
                    .rect_filled(chevron.expand(2.0), metrics::ROUNDING, colors::hover());
            }
            Icon::ChevronUp.paint(ui, chevron.shrink(3.0), colors::text_secondary());
            if super_tooltip(
                response,
                "Collapse the Ribbon",
                None,
                "Show only the tab names. Double-click a tab to bring it back.",
            )
            .clicked()
            {
                self.collapsed = true;
            }
            ui.painter().hline(
                body.x_range(),
                body.bottom() - 0.5,
                Stroke::new(1.0, colors::border()),
            );
        }

        actions
    }

    // ------------------------------------------------------------------ tab row

    fn tab_row(
        &mut self,
        ui: &mut Ui,
        format_tab: Option<(&'static str, Color32)>,
        actions: &mut Vec<AppAction>,
    ) {
        let row = Rect::from_min_size(
            ui.max_rect().min - Vec2::new(0.0, 2.0),
            Vec2::new(ui.max_rect().width(), TAB_ROW_HEIGHT + 2.0),
        );
        ui.painter().rect_filled(row, 0.0, colors::bg_window());
        if !self.collapsed {
            ui.painter().hline(
                row.x_range(),
                row.bottom() - 0.5,
                Stroke::new(1.0, colors::border()),
            );
        }

        ui.horizontal(|ui| {
            ui.set_height(TAB_ROW_HEIGHT);
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(6.0);

            // File: accent-filled, opens Backstage
            let file = self.tab_button(ui, "File", false, Some(colors::accent()), true);
            if file.clicked() {
                actions.push(AppAction::ShowBackstage);
            }

            let tabs = [
                (RibbonTab::Home, "Home"),
                (RibbonTab::Insert, "Insert"),
                (RibbonTab::PageDesign, "Page Design"),
                (RibbonTab::Review, "Review"),
                (RibbonTab::View, "View"),
                (RibbonTab::Help, "Help"),
            ];
            for (tab, title) in tabs {
                let response = self.tab_button(ui, title, self.current_tab == tab, None, false);
                self.handle_tab_click(&response, tab);
            }
            if let Some((title, color)) = format_tab {
                let response = self.tab_button(
                    ui,
                    title,
                    self.current_tab == RibbonTab::Format,
                    Some(color),
                    false,
                );
                self.handle_tab_click(&response, RibbonTab::Format);
            }

            if self.collapsed {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                    if response.hovered() {
                        ui.painter()
                            .rect_filled(rect, metrics::ROUNDING, colors::hover());
                    }
                    Icon::ChevronDown.paint(ui, rect.shrink(3.0), colors::text_secondary());
                    if super_tooltip(
                        response,
                        "Pin the Ribbon",
                        None,
                        "Keep the ribbon expanded.",
                    )
                    .clicked()
                    {
                        self.collapsed = false;
                    }
                });
            }
        });
    }

    fn handle_tab_click(&mut self, response: &Response, tab: RibbonTab) {
        if response.double_clicked() {
            self.collapsed = !self.collapsed;
            self.current_tab = tab;
        } else if response.clicked() {
            self.current_tab = tab;
            self.collapsed = false;
        }
    }

    /// One tab. `accent` colors the File tab fill (`filled`) or a contextual tab's text.
    fn tab_button(
        &self,
        ui: &mut Ui,
        title: &str,
        active: bool,
        accent: Option<Color32>,
        filled: bool,
    ) -> Response {
        let galley = ui.fonts(|f| {
            f.layout_no_wrap(title.to_owned(), fonts::ribbon_tab(), Color32::PLACEHOLDER)
        });
        let size = Vec2::new(galley.size().x + 22.0, TAB_ROW_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter();
        let tab_rect = Rect::from_min_max(
            rect.min + Vec2::new(0.0, 3.0),
            rect.max + Vec2::new(0.0, 1.0),
        );

        let text_color = if filled {
            painter.rect_filled(
                tab_rect,
                metrics::ROUNDING,
                if response.hovered() {
                    colors::accent_hover()
                } else {
                    colors::accent()
                },
            );
            Color32::WHITE
        } else if active && !self.collapsed {
            // Active tab merges into the ribbon body: fill down over the body's top border
            painter.rect_filled(
                tab_rect,
                egui::Rounding {
                    nw: 2.0,
                    ne: 2.0,
                    sw: 0.0,
                    se: 0.0,
                },
                colors::bg_ribbon(),
            );
            let s = Stroke::new(1.0, colors::border());
            painter.vline(tab_rect.left(), tab_rect.top()..=tab_rect.bottom() - 1.0, s);
            painter.vline(
                tab_rect.right(),
                tab_rect.top()..=tab_rect.bottom() - 1.0,
                s,
            );
            painter.hline(tab_rect.x_range(), tab_rect.top(), s);
            accent.unwrap_or(colors::accent())
        } else {
            if response.hovered() {
                painter.rect_filled(
                    tab_rect.shrink2(Vec2::new(1.0, 2.0)),
                    metrics::ROUNDING,
                    colors::hover(),
                );
            }
            accent.unwrap_or(colors::text_primary())
        };

        // Contextual tabs carry a colored strip along the top
        if let (Some(color), false) = (accent, filled) {
            painter.rect_filled(
                Rect::from_min_size(tab_rect.min, Vec2::new(tab_rect.width(), 3.0)),
                0.0,
                color,
            );
        }

        painter.galley(
            rect.center() + Vec2::new(-galley.size().x / 2.0, -galley.size().y / 2.0 + 2.0),
            galley,
            text_color,
        );
        response
    }

    // ------------------------------------------------------------------ tabs

    fn home_tab(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        ctx: &RibbonContext,
        selection: SelectionKind,
        actions: &mut Vec<AppAction>,
    ) {
        clipboard_group(ui, state, ctx, actions);

        let _ = selection;
        let text = text_format::has_target(state);
        let font_options = group(ui, "Font", true, |ui| {
            ui.add_enabled_ui(text, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        font_family_box(ui, state, text, actions);
                        let size = text.then(|| text_format::current_size(state)).flatten();
                        egui::ComboBox::from_id_salt("ribbon_size")
                            .width(52.0)
                            .selected_text(size.map(format_size).unwrap_or_default())
                            .show_ui(ui, |ui| {
                                for s in text_format::FONT_SIZES {
                                    if ui
                                        .selectable_label(size == Some(s), format_size(s))
                                        .clicked()
                                    {
                                        actions.push(AppAction::SetFontSize(s));
                                    }
                                }
                            });
                        if icon_button(
                            ui,
                            Btn::new(Icon::FontIncrease, "", "Increase Font Size")
                                .enabled(text)
                                .keys("Shift+."),
                        )
                        .clicked()
                        {
                            actions.push(AppAction::GrowFont);
                        }
                        if icon_button(
                            ui,
                            Btn::new(Icon::FontDecrease, "", "Decrease Font Size")
                                .enabled(text)
                                .keys("Shift+,"),
                        )
                        .clicked()
                        {
                            actions.push(AppAction::ShrinkFont);
                        }
                    });
                    ui.horizontal(|ui| {
                        let toggles = [
                            (
                                Icon::TextBold,
                                "Bold",
                                "B",
                                Toggle::Bold,
                                AppAction::TextBold,
                            ),
                            (
                                Icon::TextItalic,
                                "Italic",
                                "I",
                                Toggle::Italic,
                                AppAction::TextItalic,
                            ),
                            (
                                Icon::TextUnderline,
                                "Underline",
                                "U",
                                Toggle::Underline,
                                AppAction::TextUnderline,
                            ),
                        ];
                        for (icon, tip, key, toggle, action) in toggles {
                            let on = text && text_format::is_on(state, toggle);
                            if icon_button(
                                ui,
                                Btn::new(icon, "", tip).enabled(text).checked(on).keys(key),
                            )
                            .clicked()
                            {
                                actions.push(action);
                            }
                        }
                        let scripts = [
                            (
                                Icon::TextSubscript,
                                "Subscript",
                                Toggle::Subscript,
                                AppAction::TextSubscript,
                            ),
                            (
                                Icon::TextSuperscript,
                                "Superscript",
                                Toggle::Superscript,
                                AppAction::TextSuperscript,
                            ),
                        ];
                        for (icon, tip, toggle, action) in scripts {
                            let on = text && text_format::is_on(state, toggle);
                            if icon_button(ui, Btn::new(icon, "", tip).enabled(text).checked(on))
                                .clicked()
                            {
                                actions.push(action);
                            }
                        }
                        let response = icon_button(
                            ui,
                            Btn::new(Icon::TextColor, "", "Font Color")
                                .enabled(text)
                                .menu(),
                        );
                        let hit = if response.clicked() {
                            Hit::Menu
                        } else {
                            Hit::None
                        };
                        dropdown(ui, &response, hit, "ribbon_font_color", |ui| {
                            if let Some(color) = swatches(ui) {
                                actions.push(AppAction::SetFontColor(color));
                            }
                        });
                    });
                });
            });
        });

        let paragraph_options = group(ui, "Paragraph", true, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let list = text.then(|| text_format::current_list(state)).flatten();
                    if icon_button(
                        ui,
                        Btn::new(Icon::TextBulletListLtr, "", "Bullets")
                            .enabled(text)
                            .checked(list == Some(text_format::ListKind::Bullets))
                            .keys("Shift+L"),
                    )
                    .clicked()
                    {
                        actions.push(AppAction::ToggleBullets);
                    }
                    if icon_button(
                        ui,
                        Btn::new(Icon::TextNumberListLtr, "", "Numbering")
                            .enabled(text)
                            .checked(list == Some(text_format::ListKind::Numbering)),
                    )
                    .clicked()
                    {
                        actions.push(AppAction::ToggleNumbering);
                    }
                    if icon_button(
                        ui,
                        Btn::new(Icon::TextIndentDecreaseLtr, "", "Decrease Indent").enabled(text),
                    )
                    .clicked()
                    {
                        actions.push(AppAction::ChangeIndent(false));
                    }
                    if icon_button(
                        ui,
                        Btn::new(Icon::TextIndentIncreaseLtr, "", "Increase Indent").enabled(text),
                    )
                    .clicked()
                    {
                        actions.push(AppAction::ChangeIndent(true));
                    }
                    let response = icon_button(
                        ui,
                        Btn::new(Icon::TextLineSpacing, "", "Line Spacing")
                            .enabled(text)
                            .menu(),
                    );
                    let hit = if response.clicked() {
                        Hit::Menu
                    } else {
                        Hit::None
                    };
                    let current = text_format::current_line_spacing(state);
                    dropdown(ui, &response, hit, "ribbon_line_spacing", |ui| {
                        for m in [1.0, 1.15, 1.5, 2.0, 2.5, 3.0] {
                            let checked = current.is_some_and(|c| (c - m).abs() < 1e-6);
                            if menu_item(
                                ui,
                                checked.then_some(Icon::Checkmark),
                                &format!("{m}"),
                                None,
                                true,
                            ) {
                                actions.push(AppAction::SetLineSpacing(m));
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    let current = text
                        .then(|| text_format::current_alignment(state))
                        .flatten();
                    let aligns = [
                        (
                            Icon::TextAlignLeft,
                            "Align Left",
                            "L",
                            TextAlignment::Left,
                            AppAction::TextAlignLeft,
                        ),
                        (
                            Icon::TextAlignCenter,
                            "Center",
                            "E",
                            TextAlignment::Center,
                            AppAction::TextAlignCenter,
                        ),
                        (
                            Icon::TextAlignRight,
                            "Align Right",
                            "R",
                            TextAlignment::Right,
                            AppAction::TextAlignRight,
                        ),
                        (
                            Icon::TextAlignJustify,
                            "Justify",
                            "J",
                            TextAlignment::Justify,
                            AppAction::TextAlignJustify,
                        ),
                    ];
                    for (icon, tip, key, alignment, action) in aligns {
                        if icon_button(
                            ui,
                            Btn::new(icon, "", tip)
                                .enabled(text)
                                .checked(current == Some(alignment))
                                .keys(key),
                        )
                        .clicked()
                        {
                            actions.push(action);
                        }
                    }
                });
            });
        });

        if font_options && text {
            actions.push(AppAction::ShowFontDialog);
        }
        if paragraph_options && text {
            actions.push(AppAction::ShowParagraphDialog);
        }

        objects_group(ui, state, actions);
        arrange_group(ui, state, actions);

        group(ui, "Editing", false, |ui| {
            ui.vertical(|ui| {
                if small_button(
                    ui,
                    Btn::new(Icon::Search, "Find", "Find text in the publication.")
                        .enabled(true)
                        .keys("F"),
                )
                .1 == Hit::Main
                {
                    actions.push(AppAction::Find);
                }
                let replace = Btn::new(Icon::ArrowSwap, "Replace", "Replace text.");
                let replace = if cfg!(target_os = "macos") {
                    replace
                } else {
                    replace.keys("H")
                };
                if small_button(ui, replace).1 == Hit::Main {
                    actions.push(AppAction::Replace);
                }
                let (response, hit) = small_button(
                    ui,
                    Btn::new(Icon::Cursor, "Select", "Select objects or text.").menu(),
                );
                dropdown(ui, &response, hit, "ribbon_select_menu", |ui| {
                    if menu_item(ui, Some(Icon::SelectObject), "Select Objects", None, true) {
                        state.current_tool = Tool::Select;
                    }
                    if menu_item(ui, None, "Select All", Some(shortcut("A")), true) {
                        actions.push(AppAction::SelectAll);
                    }
                });
            });
        });
    }

    fn insert_tab(&mut self, ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
        let has_doc = state.has_document();
        group(ui, "Pages", false, |ui| {
            let after = state.current_page + 1;
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::DocumentAddColor,
                    "Page",
                    "Insert a blank page after this one.",
                )
                .split()
                .enabled(has_doc),
            );
            if hit == Hit::Main {
                actions.push(AppAction::InsertPageAt(after));
            }
            dropdown(ui, &response, hit, "ribbon_insert_page", |ui| {
                if menu_item(ui, Some(Icon::DocumentAdd), "Insert Blank Page", None, true) {
                    actions.push(AppAction::InsertPageAt(after));
                }
                if menu_item(
                    ui,
                    Some(Icon::DocumentCopy),
                    "Insert Duplicate Page",
                    None,
                    true,
                ) {
                    actions.push(AppAction::DuplicatePage(state.current_page));
                }
            });
        });
        group(ui, "Tables", false, |ui| {
            if large_button(
                ui,
                Btn::new(Icon::TableColor, "Table", "Insert a table.").enabled(has_doc),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::InsertTable);
            }
        });
        group(ui, "Illustrations", false, |ui| {
            if large_button(
                ui,
                Btn::new(
                    Icon::ImageColor,
                    "Pictures",
                    "Insert a picture from a file.",
                )
                .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::InsertPicture);
            }
            shapes_button(ui, state, has_doc);
        });
        group(ui, "Text", false, |ui| {
            if large_button(
                ui,
                Btn::new(Icon::Textbox, "Draw\nText Box", "Draw a text box.")
                    .checked(state.current_tool == Tool::Text)
                    .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                state.current_tool = Tool::Text;
            }
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::TextCaseTitle,
                    "WordArt",
                    "Insert decorative text with a warp shape and 3-D look.",
                )
                .menu()
                .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_wordart", |ui| {
                for (preset, name) in crate::dialogs::wordart::PRESETS {
                    if menu_item(ui, None, name, None, true) {
                        actions.push(AppAction::InsertWordArt(preset));
                    }
                }
            });
        });
        group(ui, "Header & Footer", false, |ui| {
            // Headers and footers live on the master page, as in Publisher
            if large_button(
                ui,
                Btn::new(
                    Icon::DocumentHeader,
                    "Header",
                    "Edit the header on this page's master page.",
                )
                .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::EditMasterPage(None));
            }
            if large_button(
                ui,
                Btn::new(
                    Icon::DocumentFooter,
                    "Footer",
                    "Edit the footer on this page's master page.",
                )
                .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::EditMasterPage(None));
            }
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::NumberSymbol,
                    "Page\nNumber",
                    "Insert a page number or another field.",
                )
                .split()
                .enabled(has_doc),
            );
            if hit == Hit::Main {
                actions.push(AppAction::InsertPageNumber);
            }
            dropdown(ui, &response, hit, "ribbon_insert_field", |ui| {
                use repub_model::text::Field;
                let items = [
                    ("Page Number", Field::page_number()),
                    ("Page Count", Field::page_count()),
                    ("Date", Field::date("%B %e, %Y")),
                    ("Time", Field::time("%l:%M %p")),
                    ("File Name", Field::FileName),
                ];
                for (label, field) in items {
                    if menu_item(ui, None, label, None, true) {
                        actions.push(AppAction::InsertField(field));
                    }
                }
            });
        });
        group(ui, "Links", false, |ui| {
            if large_button(
                ui,
                Btn::new(Icon::LinkColor, "Link", "Insert a hyperlink.")
                    .enabled(has_doc)
                    .keys("K"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::InsertHyperlink);
            }
        });
    }

    fn page_design_tab(&mut self, ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
        let has_doc = state.has_document();
        if group(ui, "Page Setup", true, |ui| {
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::DocumentMargins,
                    "Margins",
                    "Set the margins of every page.",
                )
                .menu()
                .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_margins_menu", |ui| {
                for (label, inches) in [
                    ("Wide (1\")", 1.0),
                    ("Moderate (0.5\")", 0.5),
                    ("Narrow (0.25\")", 0.25),
                    ("None", 0.0),
                ] {
                    if menu_item(ui, None, label, None, true) {
                        actions.push(AppAction::SetMargins(inches * 72.0));
                    }
                }
                ui.separator();
                if menu_item(ui, None, "Custom Margins…", None, true) {
                    actions.push(AppAction::ShowPageProperties);
                }
            });
            let landscape = state
                .current_page()
                .is_some_and(|p| p.size.width > p.size.height);
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::Orientation,
                    "Orientation",
                    "Switch every page between portrait and landscape.",
                )
                .menu()
                .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_orientation_menu", |ui| {
                if menu_item(
                    ui,
                    (!landscape).then_some(Icon::Checkmark),
                    "Portrait",
                    None,
                    true,
                ) {
                    actions.push(AppAction::SetOrientation(false));
                }
                if menu_item(
                    ui,
                    landscape.then_some(Icon::Checkmark),
                    "Landscape",
                    None,
                    true,
                ) {
                    actions.push(AppAction::SetOrientation(true));
                }
            });
            let (response, hit) = large_button(
                ui,
                Btn::new(Icon::Resize, "Size", "Choose a page size for every page.")
                    .menu()
                    .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_size_menu", |ui| {
                use repub_model::PageSize;
                let sizes = [
                    ("Letter (8.5 x 11\")", PageSize::Letter),
                    ("Legal (8.5 x 14\")", PageSize::Legal),
                    ("Tabloid (11 x 17\")", PageSize::Tabloid),
                    ("A4 (210 x 297 mm)", PageSize::A4),
                    ("A5 (148 x 210 mm)", PageSize::A5),
                    ("A3 (297 x 420 mm)", PageSize::A3),
                    ("Business Card (3.5 x 2\")", PageSize::BusinessCard),
                ];
                for (label, size) in sizes {
                    let pts = size.to_points();
                    // Portrait dimensions; the menu keeps each page's orientation
                    let (w, h) = (pts.width.min(pts.height), pts.width.max(pts.height));
                    if menu_item(ui, None, label, None, true) {
                        actions.push(AppAction::SetPageSize {
                            width: w,
                            height: h,
                        });
                    }
                }
                ui.separator();
                if menu_item(ui, None, "Custom Page Size…", None, true) {
                    actions.push(AppAction::ShowPageProperties);
                }
            });
        }) {
            actions.push(AppAction::ShowPageProperties);
        }

        group(ui, "Layout", false, |ui| {
            let (response, hit) = large_button(
                ui,
                Btn::new(Icon::Grid, "Guides", "Show, hide, or clear layout guides.")
                    .menu()
                    .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_guides_menu", |ui| {
                if menu_item(
                    ui,
                    state.show_guides.then_some(Icon::Checkmark),
                    "Show Guides",
                    None,
                    true,
                ) {
                    actions.push(AppAction::ToggleGuides);
                }
                if menu_item(
                    ui,
                    state.show_grid.then_some(Icon::Checkmark),
                    "Show Grid",
                    None,
                    true,
                ) {
                    actions.push(AppAction::ToggleGrid);
                }
                ui.separator();
                if menu_item(
                    ui,
                    None,
                    "Clear All Ruler Guides",
                    None,
                    !state.guides().is_empty(),
                ) {
                    actions.push(AppAction::ClearGuides);
                }
            });
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Align To")
                        .font(label_font())
                        .color(colors::text_primary()),
                );
                ui.checkbox(&mut state.snap_to_guides, "Guides");
                ui.checkbox(&mut state.snap_to_grid, "Grid");
            });
        });

        group(ui, "Pages", false, |ui| {
            ui.vertical(|ui| {
                let can_delete = state.page_count() > 1;
                if small_button(
                    ui,
                    Btn::new(Icon::Dismiss, "Delete", "Delete the current page.")
                        .enabled(can_delete),
                )
                .1 == Hit::Main
                {
                    actions.push(AppAction::DeletePage(state.current_page));
                }
                if small_button(
                    ui,
                    Btn::new(Icon::ArrowSwap, "Move", "Move the current page.").enabled(has_doc),
                )
                .1 == Hit::Main
                {
                    actions.push(AppAction::MovePage);
                }
                if small_button(
                    ui,
                    Btn::new(Icon::Edit, "Rename", "Rename the current page.").enabled(has_doc),
                )
                .1 == Hit::Main
                {
                    actions.push(AppAction::RenamePage);
                }
            });
        });

        group(ui, "Page Background", false, |ui| {
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::PaintBucket,
                    "Background",
                    "Fill this page's background.",
                )
                .menu()
                .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_background_menu", |ui| {
                if menu_item(ui, None, "No Background", None, true) {
                    actions.push(AppAction::SetPageBackground(None));
                }
                ui.separator();
                if let Some(color) = swatches(ui) {
                    actions.push(AppAction::SetPageBackground(Some(color)));
                }
            });
            let editing = state.master_edit.is_some();
            let (response, hit) = large_button(
                ui,
                Btn::new(
                    Icon::DocumentCopy,
                    "Master\nPages",
                    "Edit, create or apply master pages.",
                )
                .menu()
                .checked(editing)
                .enabled(has_doc),
            );
            let masters: Vec<_> = state
                .document
                .as_ref()
                .map(|d| {
                    d.master_pages()
                        .iter()
                        .map(|m| (m.id, m.name.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let applied = state
                .document
                .as_ref()
                .and_then(|d| d.page(state.current_page))
                .and_then(|p| p.master_page);
            dropdown(ui, &response, hit, "ribbon_master_menu", |ui| {
                if editing {
                    if menu_item(ui, None, "Close Master Page", None, true) {
                        actions.push(AppAction::ExitMasterPageEdit);
                    }
                } else if menu_item(ui, Some(Icon::Edit), "Edit Master Page", None, true) {
                    actions.push(AppAction::EditMasterPage(None));
                }
                if menu_item(ui, Some(Icon::DocumentAdd), "New Master Page", None, true) {
                    actions.push(AppAction::NewMasterPage);
                }
                if !masters.is_empty() && !editing {
                    ui.separator();
                    ui.label(
                        egui::RichText::new("  Apply to this page")
                            .font(fonts::small())
                            .color(colors::text_secondary()),
                    );
                    if menu_item(
                        ui,
                        applied.is_none().then_some(Icon::Checkmark),
                        "None",
                        None,
                        true,
                    ) {
                        actions.push(AppAction::ApplyMasterPage(None));
                    }
                    for (id, name) in &masters {
                        if menu_item(
                            ui,
                            (applied == Some(*id)).then_some(Icon::Checkmark),
                            name,
                            None,
                            true,
                        ) {
                            actions.push(AppAction::ApplyMasterPage(Some(*id)));
                        }
                    }
                }
            });
        });
    }

    fn review_tab(&mut self, ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
        let has_doc = state.has_document();
        group(ui, "Proofing", false, |ui| {
            if large_button(
                ui,
                Btn::new(Icon::TextCaseTitle, "Spelling", "Check spelling (F7).").enabled(has_doc),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::CheckSpelling);
            }
        });
    }

    fn view_tab(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        ctx: &RibbonContext,
        actions: &mut Vec<AppAction>,
    ) {
        let has_doc = state.has_document();
        group(ui, "Views", false, |ui| {
            let editing = state.master_edit.is_some();
            if large_button(
                ui,
                Btn::new(
                    Icon::DocumentCopy,
                    "Master\nPage",
                    "Switch between the pages and their master page.",
                )
                .checked(editing)
                .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                actions.push(if editing {
                    AppAction::ExitMasterPageEdit
                } else {
                    AppAction::EditMasterPage(None)
                });
            }
        });
        group(ui, "Layout", false, |ui| {
            if large_button(
                ui,
                Btn::new(
                    Icon::DocumentOnePage,
                    "Single\nPage",
                    "Show one page at a time.",
                )
                .checked(!state.two_page_spread)
                .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                if state.two_page_spread {
                    actions.push(AppAction::ToggleTwoPageSpread);
                }
            }
            if large_button(
                ui,
                Btn::new(
                    Icon::BookOpenColor,
                    "Two-Page\nSpread",
                    "Show facing pages together.",
                )
                .checked(state.two_page_spread)
                .enabled(has_doc),
            )
            .1 == Hit::Main
            {
                if !state.two_page_spread {
                    actions.push(AppAction::ToggleTwoPageSpread);
                }
            }
        });
        group(ui, "Show", false, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                checkbox_action(
                    ui,
                    state.show_rulers,
                    "Rulers",
                    AppAction::ToggleRulers,
                    actions,
                );
                checkbox_action(
                    ui,
                    state.show_guides,
                    "Guides",
                    AppAction::ToggleGuides,
                    actions,
                );
                checkbox_action(ui, state.show_grid, "Grid", AppAction::ToggleGrid, actions);
            });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                checkbox_action(
                    ui,
                    ctx.pages_pane_visible,
                    "Page Navigation",
                    AppAction::TogglePagesPane,
                    actions,
                );
                checkbox_action(
                    ui,
                    ctx.selection_pane_visible,
                    "Selection Pane",
                    AppAction::ToggleSelectionPane,
                    actions,
                );
            });
        });
        group(ui, "Zoom", false, |ui| {
            let (response, hit) = large_button(
                ui,
                Btn::new(Icon::ZoomIn, "Zoom", "Choose a zoom level.")
                    .menu()
                    .enabled(has_doc),
            );
            dropdown(ui, &response, hit, "ribbon_zoom_menu", |ui| {
                for pct in [400, 200, 150, 100, 75, 50, 33, 25] {
                    if menu_item(ui, None, &format!("{pct}%"), None, true) {
                        state.set_zoom_centered(pct as f32 / 100.0);
                    }
                }
                ui.separator();
                if menu_item(ui, Some(Icon::PageFit), "Whole Page", None, true) {
                    actions.push(AppAction::ZoomFit);
                }
            });
            if large_button(
                ui,
                Btn::new(Icon::DocumentOnePage, "100%", "Zoom to 100%.")
                    .enabled(has_doc)
                    .keys("1"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::ZoomActual);
            }
            if large_button(
                ui,
                Btn::new(Icon::PageFit, "Whole\nPage", "Zoom to show the whole page.")
                    .enabled(has_doc)
                    .keys("0"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::ZoomFit);
            }
            ui.vertical(|ui| {
                if small_button(
                    ui,
                    Btn::new(Icon::ZoomIn, "Zoom In", "Zoom in.")
                        .enabled(has_doc)
                        .keys("="),
                )
                .1 == Hit::Main
                {
                    actions.push(AppAction::ZoomIn);
                }
                if small_button(
                    ui,
                    Btn::new(Icon::ZoomOut, "Zoom Out", "Zoom out.")
                        .enabled(has_doc)
                        .keys("-"),
                )
                .1 == Hit::Main
                {
                    actions.push(AppAction::ZoomOut);
                }
            });
        });
        group(ui, "Tools", false, |ui| {
            if large_button(
                ui,
                Btn::new(Icon::Cursor, "Select", "Select and move objects (V).")
                    .checked(state.current_tool == Tool::Select),
            )
            .1 == Hit::Main
            {
                state.current_tool = Tool::Select;
            }
            if large_button(
                ui,
                Btn::new(Icon::HandLeft, "Pan", "Drag to move around the page (H).")
                    .checked(state.current_tool == Tool::Pan),
            )
            .1 == Hit::Main
            {
                state.current_tool = Tool::Pan;
            }
        });
    }

    fn help_tab(&mut self, ui: &mut Ui, actions: &mut Vec<AppAction>) {
        group(ui, "Help", false, |ui| {
            if large_button(
                ui,
                Btn::new(
                    Icon::Info,
                    "About\nRepub",
                    "Version and license information.",
                ),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::ShowAbout);
            }
        });
    }

    fn format_tab(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        selection: SelectionKind,
        actions: &mut Vec<AppAction>,
    ) {
        match selection {
            SelectionKind::Text => {
                group(ui, "Text", false, |ui| {
                    if large_button(
                        ui,
                        Btn::new(
                            Icon::TextBoxSettings,
                            "Text Fit",
                            "Shrink or grow the text to fit the text box.",
                        )
                        .enabled(true),
                    )
                    .1 == Hit::Main
                    {
                        actions.push(AppAction::ShowTextFit);
                    }
                    if large_button(
                        ui,
                        Btn::new(Icon::TextColumnTwo, "Columns", "Flow the text in columns.")
                            .enabled(true),
                    )
                    .1 == Hit::Main
                    {
                        actions.push(AppAction::ShowTextColumns);
                    }
                    if large_button(
                        ui,
                        Btn::new(Icon::DocumentMargins, "Margins", "Text box margins.")
                            .enabled(true),
                    )
                    .1 == Hit::Main
                    {
                        actions.push(AppAction::ShowTextBoxMargins);
                    }
                });
                let linked = match state.selected_objects()[..] {
                    [obj] => match &obj.kind {
                        ShapeKind::TextFrame(tf) => Some(tf.linked_frame.is_some()),
                        _ => None,
                    },
                    _ => None,
                };
                shape_styles_group(ui, actions);
                group(ui, "Linking", false, |ui| {
                    let create = Btn::new(Icon::Link, "Create\nLink", "Continue this box's text in another text box: click an empty text box, or empty space for a new one (Esc cancels).")
                        .enabled(linked == Some(false))
                        .checked(state.link_source.is_some());
                    if large_button(ui, create).1 == Hit::Main {
                        actions.push(AppAction::CreateTextLink);
                    }
                    if large_button(
                        ui,
                        Btn::new(
                            Icon::LinkDismiss,
                            "Break",
                            "Stop this box's text continuing in the next box.",
                        )
                        .enabled(linked == Some(true)),
                    )
                    .1 == Hit::Main
                    {
                        actions.push(AppAction::BreakTextLink);
                    }
                });
            }
            SelectionKind::Picture => {
                group(ui, "Adjust", false, |ui| {
                    if large_button(
                        ui,
                        Btn::new(Icon::ImageColor, "Change\nPicture", "Replace this picture.")
                            .enabled(true),
                    )
                    .1 == Hit::Main
                    {
                        actions.push(AppAction::ChangePicture);
                    }
                });
                shape_styles_group(ui, actions);
            }
            SelectionKind::Shape => {
                group(ui, "Insert Shapes", false, |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            shape_tool_button(
                                ui,
                                state,
                                Icon::RectangleLandscape,
                                Tool::Rectangle,
                                "Rectangle",
                            );
                            shape_tool_button(ui, state, Icon::Oval, Tool::Ellipse, "Oval");
                            shape_tool_button(ui, state, Icon::Line, Tool::Line, "Line");
                        });
                        ui.horizontal(|ui| {
                            shape_tool_button(ui, state, Icon::Star, Tool::Star, "Star");
                            shape_tool_button(ui, state, Icon::Hexagon, Tool::Polygon, "Polygon");
                        });
                    });
                    let editing = state.edit_points.is_some();
                    let tip = "Edit the shape's points: drag a point or its curve handles, double-click an edge to add a point.";
                    let (response, hit) = small_button(
                        ui,
                        Btn::new(Icon::DrawShape, "Edit Shape", tip)
                            .split()
                            .checked(editing)
                            .enabled(state.selection.len() == 1),
                    );
                    if hit == Hit::Main {
                        actions.push(AppAction::EditShapePoints);
                    }
                    dropdown(ui, &response, hit, "ribbon_edit_shape", |ui| {
                        use crate::edit_points::{can_apply, PointOp};
                        if menu_item(
                            ui,
                            None,
                            if editing {
                                "Done Editing Points"
                            } else {
                                "Edit Points"
                            },
                            None,
                            true,
                        ) {
                            actions.push(AppAction::EditShapePoints);
                        }
                        ui.separator();
                        for (op, label) in [
                            (PointOp::Delete, "Delete Point"),
                            (PointOp::Straight, "Straighten Segment"),
                            (PointOp::Curved, "Curve Segment"),
                        ] {
                            if menu_item(ui, None, label, None, can_apply(state, op)) {
                                actions.push(AppAction::EditPoint(op));
                            }
                        }
                    });
                });
                shape_styles_group(ui, actions);
            }
            SelectionKind::Table => {
                use crate::app::TableEdit;
                let in_cell = state.text_edit.as_ref().is_some_and(|e| e.cell.is_some());
                let at = if in_cell { "this cell" } else { "the table" };
                group(ui, "Rows & Columns", false, |ui| {
                    let (response, hit) = large_button(
                        ui,
                        Btn::new(Icon::Delete, "Delete", "Delete rows, columns or the table.")
                            .menu(),
                    );
                    dropdown(ui, &response, hit, "ribbon_table_delete", |ui| {
                        for (icon, op) in [
                            (Icon::TableDeleteRow, TableEdit::DeleteRow),
                            (Icon::TableDeleteColumn, TableEdit::DeleteColumn),
                            (Icon::TableDismiss, TableEdit::DeleteTable),
                        ] {
                            if menu_item(ui, Some(icon), op.label(), None, op.enabled(state)) {
                                actions.push(AppAction::TableEdit(op));
                            }
                        }
                        if !in_cell {
                            ui.label(
                                egui::RichText::new(
                                    "Click into a cell to delete its row or column.",
                                )
                                .small()
                                .color(crate::theme::colors::text_muted()),
                            );
                        }
                    });
                    ui.vertical(|ui| {
                        if small_button(
                            ui,
                            Btn::new(
                                Icon::TableInsertRow,
                                "Insert Above",
                                &format!("Insert a row above {at}."),
                            ),
                        )
                        .1 == Hit::Main
                        {
                            actions.push(AppAction::TableEdit(TableEdit::RowAbove));
                        }
                        if small_button(
                            ui,
                            Btn::new(
                                Icon::TableInsertRow,
                                "Insert Below",
                                &format!("Insert a row below {at}."),
                            ),
                        )
                        .1 == Hit::Main
                        {
                            actions.push(AppAction::TableEdit(TableEdit::RowBelow));
                        }
                    });
                    if large_button(ui, Btn::new(Icon::TableColor, "Table\nProperties", "Borders, column widths, row heights, and the fill, alignment and merging of cells.")).1 == Hit::Main {
                        actions.push(AppAction::ShowTableProperties);
                    }
                    ui.vertical(|ui| {
                        if small_button(
                            ui,
                            Btn::new(
                                Icon::TableInsertColumn,
                                "Insert Left",
                                &format!("Insert a column left of {at}."),
                            ),
                        )
                        .1 == Hit::Main
                        {
                            actions.push(AppAction::TableEdit(TableEdit::ColumnLeft));
                        }
                        if small_button(
                            ui,
                            Btn::new(
                                Icon::TableInsertColumn,
                                "Insert Right",
                                &format!("Insert a column right of {at}."),
                            ),
                        )
                        .1 == Hit::Main
                        {
                            actions.push(AppAction::TableEdit(TableEdit::ColumnRight));
                        }
                    });
                });
            }
            SelectionKind::Mixed | SelectionKind::None => {
                let wordart = matches!(state.selected_objects()[..], [o] if matches!(o.kind, ShapeKind::TextEffect(_)));
                if wordart {
                    group(ui, "WordArt", false, |ui| {
                        if large_button(
                            ui,
                            Btn::new(
                                Icon::TextCaseTitle,
                                "Edit\nWordArt",
                                "Change the text, font, shape and 3-D look.",
                            ),
                        )
                        .1 == Hit::Main
                        {
                            actions.push(AppAction::ShowWordArt);
                        }
                    });
                }
            }
        }
        format_shape_group(ui, state, actions);
        arrange_group(ui, state, actions);
        size_group(ui, state, actions);
    }
}

// ---------------------------------------------------------------------- shared groups

fn clipboard_group(
    ui: &mut Ui,
    state: &mut AppState,
    ctx: &RibbonContext,
    actions: &mut Vec<AppAction>,
) {
    let has_selection = !state.selection.is_empty();
    group(ui, "Clipboard", false, |ui| {
        let (response, hit) = large_button(
            ui,
            Btn::new(
                Icon::ClipboardPaste,
                "Paste",
                "Add content from the clipboard.",
            )
            .split()
            .enabled(!ctx.clipboard_empty)
            .keys("V"),
        );
        if hit == Hit::Main {
            actions.push(AppAction::Paste);
        }
        dropdown(ui, &response, hit, "ribbon_paste_menu", |ui| {
            if menu_item(
                ui,
                Some(Icon::ClipboardPaste),
                "Paste",
                Some(shortcut("V")),
                true,
            ) {
                actions.push(AppAction::Paste);
            }
            if menu_item(ui, None, "Paste in Place", Some(shortcut("Shift+V")), true) {
                actions.push(AppAction::PasteInPlace);
            }
        });
        ui.vertical(|ui| {
            if small_button(
                ui,
                Btn::new(
                    Icon::Cut,
                    "Cut",
                    "Remove the selection and put it on the clipboard.",
                )
                .enabled(has_selection)
                .keys("X"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::Cut);
            }
            if small_button(
                ui,
                Btn::new(
                    Icon::Copy,
                    "Copy",
                    "Put a copy of the selection on the clipboard.",
                )
                .enabled(has_selection)
                .keys("C"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::Copy);
            }
            if small_button(
                ui,
                Btn::new(
                    Icon::PaintBrush,
                    "Format Painter",
                    "Copy formatting to another object.",
                )
                .enabled(!state.selection.is_empty()),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::FormatPainter);
            }
        });
    });
}

fn objects_group(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
    let has_doc = state.has_document();
    group(ui, "Objects", false, |ui| {
        if large_button(
            ui,
            Btn::new(Icon::Textbox, "Draw\nText Box", "Draw a text box (T).")
                .checked(state.current_tool == Tool::Text)
                .enabled(has_doc),
        )
        .1 == Hit::Main
        {
            state.current_tool = Tool::Text;
        }
        if large_button(
            ui,
            Btn::new(
                Icon::ImageColor,
                "Pictures",
                "Insert a picture from a file.",
            )
            .enabled(has_doc),
        )
        .1 == Hit::Main
        {
            actions.push(AppAction::InsertPicture);
        }
        if large_button(
            ui,
            Btn::new(Icon::TableColor, "Table", "Insert a table.").enabled(has_doc),
        )
        .1 == Hit::Main
        {
            actions.push(AppAction::InsertTable);
        }
        shapes_button(ui, state, has_doc);
    });
}

/// "Shapes ▾" large button: picks a drawing tool from a menu.
fn shapes_button(ui: &mut Ui, state: &mut AppState, enabled: bool) {
    let shape_tool = matches!(
        state.current_tool,
        Tool::Rectangle
            | Tool::Ellipse
            | Tool::Line
            | Tool::Polygon
            | Tool::Star
            | Tool::Preset(_)
            | Tool::Freeform
            | Tool::Pencil
    );
    let (response, hit) = large_button(
        ui,
        Btn::new(Icon::Shapes, "Shapes", "Draw a shape.")
            .menu()
            .checked(shape_tool)
            .enabled(enabled),
    );
    dropdown(ui, &response, hit, "ribbon_shapes_menu", |ui| {
        // Lines are their own objects (with arrowheads); the rest are Office's presets
        if menu_item(ui, Some(Icon::Line), "Line", Some("L".to_owned()), true) {
            state.current_tool = Tool::Line;
        }
        if menu_item(ui, Some(Icon::DrawShape), "Freeform", None, true) {
            state.current_tool = Tool::Freeform;
        }
        if menu_item(ui, Some(Icon::Edit), "Pencil", None, true) {
            state.current_tool = Tool::Pencil;
        }
        ui.separator();
        if crate::shape_gallery::show(ui, state) {
            ui.close_menu();
        }
    });
}

fn shape_tool_button(ui: &mut Ui, state: &mut AppState, icon: Icon, tool: Tool, tip: &str) {
    if icon_button(
        ui,
        Btn::new(icon, "", tip).checked(state.current_tool == tool),
    )
    .clicked()
    {
        state.current_tool = tool;
    }
}

fn shape_styles_group(ui: &mut Ui, actions: &mut Vec<AppAction>) {
    group(ui, "Shape Styles", false, |ui| {
        ui.vertical(|ui| {
            let (response, hit) = small_button(
                ui,
                Btn::new(
                    Icon::ColorFill,
                    "Shape Fill",
                    "Fill the shape with a color.",
                )
                .menu(),
            );
            dropdown(ui, &response, hit, "ribbon_shape_fill", |ui| {
                if menu_item(ui, None, "No Fill", None, true) {
                    actions.push(AppAction::SetShapeFill(None));
                }
                ui.separator();
                if let Some(color) = swatches(ui) {
                    actions.push(AppAction::SetShapeFill(Some(color)));
                }
            });
            let (response, hit) = small_button(
                ui,
                Btn::new(
                    Icon::ColorLine,
                    "Shape Outline",
                    "Outline color and weight.",
                )
                .menu(),
            );
            dropdown(ui, &response, hit, "ribbon_shape_outline", |ui| {
                if menu_item(ui, None, "No Outline", None, true) {
                    actions.push(AppAction::SetShapeOutline(None));
                }
                ui.separator();
                if let Some(color) = swatches(ui) {
                    actions.push(AppAction::SetShapeOutline(Some(color)));
                }
                ui.separator();
                for weight in [0.5, 1.0, 1.5, 2.25, 3.0, 4.5, 6.0] {
                    if menu_item(ui, None, &format!("{weight} pt"), None, true) {
                        actions.push(AppAction::SetOutlineWeight(weight));
                    }
                }
            });
        });
    });
}

fn format_shape_group(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
    group(ui, "Format", false, |ui| {
        let tip = "Fill, line, effects, size, position and other properties of the selection.";
        if large_button(
            ui,
            Btn::new(Icon::PaintBucket, "Format\nShape", tip).enabled(!state.selection.is_empty()),
        )
        .1 == Hit::Main
        {
            actions.push(AppAction::ShowFormatShape);
        }
    });
}

fn arrange_group(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
    let count = state.selection.len();
    let any = count > 0;
    group(ui, "Arrange", false, |ui| {
        if large_button(
            ui,
            Btn::new(
                Icon::TextWrap,
                "Wrap\nText",
                "How text flows around this object.",
            )
            .enabled(count == 1),
        )
        .1 == Hit::Main
        {
            actions.push(AppAction::ShowTextWrap);
        }
        if large_button(
            ui,
            Btn::new(
                Icon::Layer,
                "Selection\nPane",
                "List the objects on this page to select, show or hide them.",
            ),
        )
        .1 == Hit::Main
        {
            actions.push(AppAction::ToggleSelectionPane);
        }
        ui.vertical(|ui| {
            let (response, hit) = small_button(
                ui,
                Btn::new(
                    Icon::PositionForward,
                    "Bring Forward",
                    "Bring the object forward one level.",
                )
                .split()
                .enabled(any)
                .keys("]"),
            );
            if hit == Hit::Main {
                actions.push(AppAction::BringForward);
            }
            dropdown(ui, &response, hit, "ribbon_forward_menu", |ui| {
                if menu_item(
                    ui,
                    Some(Icon::PositionForward),
                    "Bring Forward",
                    Some(shortcut("]")),
                    true,
                ) {
                    actions.push(AppAction::BringForward);
                }
                if menu_item(
                    ui,
                    Some(Icon::PositionToFront),
                    "Bring to Front",
                    Some(shortcut("Shift+]")),
                    true,
                ) {
                    actions.push(AppAction::BringToFront);
                }
            });
            let (response, hit) = small_button(
                ui,
                Btn::new(
                    Icon::PositionBackward,
                    "Send Backward",
                    "Send the object back one level.",
                )
                .split()
                .enabled(any)
                .keys("["),
            );
            if hit == Hit::Main {
                actions.push(AppAction::SendBackward);
            }
            dropdown(ui, &response, hit, "ribbon_backward_menu", |ui| {
                if menu_item(
                    ui,
                    Some(Icon::PositionBackward),
                    "Send Backward",
                    Some(shortcut("[")),
                    true,
                ) {
                    actions.push(AppAction::SendBackward);
                }
                if menu_item(
                    ui,
                    Some(Icon::PositionToBack),
                    "Send to Back",
                    Some(shortcut("Shift+[")),
                    true,
                ) {
                    actions.push(AppAction::SendToBack);
                }
            });
            // One object aligns to the page, several to each other
            let (response, hit) = small_button(
                ui,
                Btn::new(
                    Icon::AlignLeft,
                    "Align",
                    "Line up the selected objects, or one object on the page.",
                )
                .menu()
                .enabled(any),
            );
            dropdown(ui, &response, hit, "ribbon_align_menu", |ui| {
                let items = [
                    (Icon::AlignLeft, "Align Left", AppAction::AlignLeft),
                    (
                        Icon::AlignCenterHorizontal,
                        "Align Center",
                        AppAction::AlignCenter,
                    ),
                    (Icon::AlignRight, "Align Right", AppAction::AlignRight),
                    (Icon::AlignTop, "Align Top", AppAction::AlignTop),
                    (
                        Icon::AlignCenterVertical,
                        "Align Middle",
                        AppAction::AlignMiddle,
                    ),
                    (Icon::AlignBottom, "Align Bottom", AppAction::AlignBottom),
                ];
                for (icon, label, action) in items {
                    if menu_item(ui, Some(icon), label, None, true) {
                        actions.push(action);
                    }
                }
                ui.separator();
                let several = count > 2;
                if menu_item(ui, None, "Distribute Horizontally", None, several) {
                    actions.push(AppAction::Distribute(true));
                }
                if menu_item(ui, None, "Distribute Vertically", None, several) {
                    actions.push(AppAction::Distribute(false));
                }
                ui.separator();
                use crate::state::AlignTarget;
                for (target, label) in [
                    (AlignTarget::Auto, "Align to Selection (or Page)"),
                    (AlignTarget::Page, "Align to Page"),
                    (AlignTarget::Margins, "Align to Margins"),
                ] {
                    let chosen = state.align_target == target;
                    if menu_item(ui, chosen.then_some(Icon::Checkmark), label, None, true) {
                        actions.push(AppAction::SetAlignTarget(target));
                    }
                }
            });
        });
        ui.vertical(|ui| {
            let single_group = count == 1
                && state
                    .selected_objects()
                    .first()
                    .is_some_and(|o| matches!(o.kind, ShapeKind::Group(_)));
            if small_button(
                ui,
                Btn::new(Icon::Group, "Group", "Group the selected objects.")
                    .enabled(count > 1)
                    .keys("G"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::Group);
            }
            if small_button(
                ui,
                Btn::new(Icon::GroupDismiss, "Ungroup", "Break the group apart.")
                    .enabled(single_group)
                    .keys("Shift+G"),
            )
            .1 == Hit::Main
            {
                actions.push(AppAction::Ungroup);
            }
            let (response, hit) = small_button(
                ui,
                Btn::new(
                    Icon::ArrowRotateClockwise,
                    "Rotate",
                    "Rotate or flip the selection.",
                )
                .menu()
                .enabled(any),
            );
            dropdown(ui, &response, hit, "ribbon_rotate_menu", |ui| {
                if menu_item(
                    ui,
                    Some(Icon::ArrowRotateClockwise),
                    "Rotate Right 90°",
                    None,
                    true,
                ) {
                    rotate_selection(state, 90.0);
                }
                if menu_item(
                    ui,
                    Some(Icon::ArrowRotateCounterclockwise),
                    "Rotate Left 90°",
                    None,
                    true,
                ) {
                    rotate_selection(state, -90.0);
                }
                let can_flip = state.selected_objects().iter().any(|o| o.can_flip());
                if menu_item(
                    ui,
                    Some(Icon::FlipVertical),
                    "Flip Vertical",
                    None,
                    can_flip,
                ) {
                    actions.push(AppAction::Flip(false));
                }
                if menu_item(
                    ui,
                    Some(Icon::FlipHorizontal),
                    "Flip Horizontal",
                    None,
                    can_flip,
                ) {
                    actions.push(AppAction::Flip(true));
                }
            });
        });
    });
}

/// Rotate the selected objects by `degrees` (clockwise), as one undo step.
pub fn rotate_selection(state: &mut AppState, degrees: f64) {
    if state.selection.is_empty() {
        return;
    }
    state.record_undo("Rotate");
    let ids = state.selection.clone();
    if let Some(page) = state.current_page_mut() {
        for id in ids {
            if let Some(obj) = page.get_shape_mut(id) {
                let pivot = obj.bounds().center();
                crate::canvas::rotate_about(obj, pivot, degrees);
            }
        }
    }
    state.mark_dirty();
}

/// Height / Width of a single selected object, in the measurement unit.
#[allow(clippy::ptr_arg)] // kept `&mut Vec` for signature parity with sibling *_group fns
fn size_group(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<AppAction>) {
    let id = (state.selection.len() == 1).then(|| state.selection[0]);
    let bounds = id.and_then(|id| {
        state
            .current_page()
            .and_then(|p| p.get_shape(id))
            .map(|o| o.bounds())
    });
    group(ui, "Size", false, |ui| {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.spacing_mut().item_spacing.y = 6.0;
            let unit = state.units;
            let Some(bounds) = bounds else {
                ui.add_enabled_ui(false, |ui| {
                    size_row(ui, Icon::ArrowBidirectionalUpDown, "Height", &mut 0.0, unit);
                    size_row(
                        ui,
                        Icon::ArrowBidirectionalLeftRight,
                        "Width",
                        &mut 0.0,
                        unit,
                    );
                });
                return;
            };
            let (mut h, mut w) = (
                unit.from_points(bounds.height()),
                unit.from_points(bounds.width()),
            );
            let changed_h = size_row(ui, Icon::ArrowBidirectionalUpDown, "Height", &mut h, unit);
            let changed_w = size_row(ui, Icon::ArrowBidirectionalLeftRight, "Width", &mut w, unit);
            if changed_h || changed_w {
                state.record_undo_coalesced("Size");
                if let (Some(id), Some(page)) = (id, state.current_page_mut()) {
                    if let Some(obj) = page.get_shape_mut(id) {
                        obj.set_size(unit.to_points(w).max(0.72), unit.to_points(h).max(0.72));
                    }
                }
                state.mark_dirty();
            }
        });
    });
}

/// The Font box: installed families (filterable), applied to the selection.
fn font_family_box(ui: &mut Ui, state: &AppState, enabled: bool, actions: &mut Vec<AppAction>) {
    let current = enabled
        .then(|| text_format::current_family(state))
        .flatten();
    let filter_id = egui::Id::new("ribbon_font_filter");
    egui::ComboBox::from_id_salt("ribbon_font")
        .width(130.0)
        .height(360.0)
        .selected_text(current.clone().unwrap_or_default())
        .show_ui(ui, |ui| {
            let Some(registry) = crate::sdf_text::fonts::try_registry() else {
                ui.label(
                    egui::RichText::new("Finding fonts…")
                        .color(crate::theme::colors::text_secondary()),
                );
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(200));
                return;
            };
            let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());
            let edit = ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Search fonts")
                    .desired_width(200.0),
            );
            if !edit.has_focus() && filter.is_empty() {
                edit.request_focus();
            }
            ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
            let needle = filter.to_lowercase();
            for family in registry
                .family_names()
                .into_iter()
                .filter(|f| f.to_lowercase().contains(&needle))
            {
                let on = current.as_deref() == Some(family.as_str());
                if ui.selectable_label(on, &family).clicked() {
                    actions.push(AppAction::SetFontFamily(family));
                    ui.data_mut(|d| d.remove::<String>(filter_id));
                    ui.close_menu();
                }
            }
        });
}

fn size_row(
    ui: &mut Ui,
    icon: Icon,
    tip: &str,
    value: &mut f64,
    unit: crate::state::DisplayUnit,
) -> bool {
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
        icon.paint_state(
            ui,
            rect.shrink(1.0),
            ui.is_enabled(),
            colors::text_primary(),
            colors::text_disabled(),
        );
        response.on_hover_text(tip);
        ui.add_sized(
            Vec2::new(70.0, 20.0),
            egui::DragValue::new(value)
                .speed(unit.from_points(0.72))
                .range(unit.from_points(0.72)..=unit.from_points(7200.0))
                .min_decimals(if unit == crate::state::DisplayUnit::Inches {
                    2
                } else {
                    1
                })
                .max_decimals(2)
                .suffix(unit.suffix()),
        )
        .changed()
    })
    .inner
}

fn format_size(size: f64) -> String {
    if (size - size.round()).abs() < 0.05 {
        format!("{}", size.round())
    } else {
        format!("{size:.1}")
    }
}

/// Standard colours (Publisher's colour menu: a greys row and the standard colours).
const SWATCHES: [[(u8, u8, u8, &str); 10]; 2] = [
    [
        (0xFF, 0xFF, 0xFF, "White"),
        (0xF2, 0xF2, 0xF2, "White, Darker 5%"),
        (0xD9, 0xD9, 0xD9, "White, Darker 15%"),
        (0xBF, 0xBF, 0xBF, "White, Darker 25%"),
        (0xA6, 0xA6, 0xA6, "White, Darker 35%"),
        (0x80, 0x80, 0x80, "Gray"),
        (0x59, 0x59, 0x59, "Black, Lighter 35%"),
        (0x40, 0x40, 0x40, "Black, Lighter 25%"),
        (0x26, 0x26, 0x26, "Black, Lighter 15%"),
        (0x00, 0x00, 0x00, "Black"),
    ],
    [
        (0xC0, 0x00, 0x00, "Dark Red"),
        (0xFF, 0x00, 0x00, "Red"),
        (0xFF, 0xC0, 0x00, "Orange"),
        (0xFF, 0xFF, 0x00, "Yellow"),
        (0x92, 0xD0, 0x50, "Light Green"),
        (0x00, 0xB0, 0x50, "Green"),
        (0x00, 0xB0, 0xF0, "Light Blue"),
        (0x00, 0x70, 0xC0, "Blue"),
        (0x00, 0x20, 0x60, "Dark Blue"),
        (0x70, 0x30, 0xA0, "Purple"),
    ],
];

/// A grid of colour swatches; returns the colour clicked this frame.
fn swatches(ui: &mut Ui) -> Option<repub_color::Color> {
    let mut picked = None;
    ui.add_space(4.0);
    for row in SWATCHES {
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing.x = 3.0;
            for (r, g, b, name) in row {
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_rgb(r, g, b));
                let border = if response.hovered() {
                    colors::accent()
                } else {
                    colors::border()
                };
                ui.painter()
                    .rect_stroke(rect, 0.0, Stroke::new(1.0, border));
                if response.on_hover_text(name).clicked() {
                    picked = Some(repub_color::Color::from_rgb8(r, g, b));
                }
            }
        });
        ui.add_space(3.0);
    }
    picked
}

fn checkbox_action(
    ui: &mut Ui,
    checked: bool,
    label: &str,
    action: AppAction,
    actions: &mut Vec<AppAction>,
) {
    let mut value = checked;
    if ui.checkbox(&mut value, label).changed() {
        actions.push(action);
    }
}

// ---------------------------------------------------------------------- widgets

/// A ribbon group: buttons in a fixed-height row, the group name on the shared
/// bottom baseline, a full-height separator on the right, and an optional dialog
/// launcher in the label row. Returns true when the launcher was clicked.
fn group(ui: &mut Ui, name: &str, launcher: bool, add: impl FnOnce(&mut Ui)) -> bool {
    let mut launched = false;
    let label =
        ui.fonts(|f| f.layout_no_wrap(name.to_owned(), fonts::small(), Color32::PLACEHOLDER));
    let min_width = label.size().x + if launcher { 28.0 } else { 12.0 };

    let response = ui
        .vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.set_height(CONTENT_HEIGHT);
                ui.set_min_width(min_width);
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 1.0);
                add(ui);
            });
            let width = ui.min_rect().width().max(min_width);
            let (label_rect, _) =
                ui.allocate_exact_size(Vec2::new(width, LABEL_HEIGHT), Sense::hover());
            let text_center = if launcher {
                label_rect.center() - Vec2::new(8.0, 0.0)
            } else {
                label_rect.center()
            };
            ui.painter().galley(
                text_center - label.size() / 2.0,
                label,
                colors::text_secondary(),
            );

            if launcher {
                let rect = Rect::from_center_size(
                    label_rect.right_center() - Vec2::new(8.0, 0.0),
                    Vec2::splat(14.0),
                );
                let response =
                    ui.interact(rect, ui.id().with(name).with("launcher"), Sense::click());
                if response.hovered() {
                    ui.painter()
                        .rect_filled(rect, metrics::ROUNDING, colors::hover());
                }
                Icon::ArrowDownRight.paint(ui, rect.shrink(2.0), colors::text_secondary());
                launched = super_tooltip(
                    response,
                    &format!("{name} Options"),
                    None,
                    "Open the dialog with more options.",
                )
                .clicked();
            }
        })
        .response;

    let x = response.rect.right() + 4.0;
    ui.painter().vline(
        x,
        response.rect.top() + 2.0..=response.rect.bottom() - 2.0,
        Stroke::new(1.0, colors::border()),
    );
    ui.add_space(9.0);
    launched
}

fn button_colors(response: &Response, b: &Btn) -> (Option<Color32>, Option<Stroke>) {
    if b.checked {
        (
            Some(colors::selected()),
            Some(Stroke::new(1.0, colors::accent())),
        )
    } else if b.enabled && response.is_pointer_button_down_on() {
        (Some(colors::pressed()), None)
    } else if b.enabled && response.hovered() {
        (Some(colors::hover()), None)
    } else {
        (None, None)
    }
}

fn text_color(enabled: bool) -> Color32 {
    if enabled {
        colors::text_primary()
    } else {
        colors::text_disabled()
    }
}

fn resolve_hit(ui: &Ui, response: &Response, b: &Btn, menu_zone: Rect) -> Hit {
    if !b.enabled || !response.clicked() {
        return Hit::None;
    }
    if !b.dropdown {
        return Hit::Main;
    }
    if !b.split {
        return Hit::Menu;
    }
    let pos = ui.input(|i| i.pointer.interact_pos());
    if pos.is_some_and(|p| menu_zone.contains(p)) {
        Hit::Menu
    } else {
        Hit::Main
    }
}

/// Large button: 32 px icon over a one- or two-line label.
fn large_button(ui: &mut Ui, b: Btn) -> (Response, Hit) {
    let lines: Vec<_> = b
        .label
        .split('\n')
        .map(|line| {
            ui.fonts(|f| f.layout_no_wrap(line.to_owned(), label_font(), Color32::PLACEHOLDER))
        })
        .collect();
    let chevron_inline = b.dropdown && lines.len() > 1;
    let text_width = lines.iter().map(|g| g.size().x).fold(0.0, f32::max)
        + if chevron_inline { 12.0 } else { 0.0 };
    let width = (text_width + 12.0).max(44.0);
    let sense = if b.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, CONTENT_HEIGHT), sense);

    let menu_zone = Rect::from_min_max(egui::pos2(rect.left(), rect.top() + 40.0), rect.max);
    let (fill, stroke) = button_colors(&response, &b);
    let painter = ui.painter();
    if let Some(fill) = fill {
        painter.rect_filled(rect, metrics::ROUNDING, fill);
    }
    if let Some(stroke) = stroke {
        painter.rect_stroke(rect, metrics::ROUNDING, stroke);
    }
    if b.split && b.enabled && response.hovered() && !b.checked {
        painter.hline(
            rect.x_range().shrink(3.0),
            menu_zone.top(),
            Stroke::new(1.0, colors::border_strong()),
        );
    }

    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 21.0),
        Vec2::splat(32.0),
    );
    b.icon.paint_state(
        ui,
        icon_rect,
        b.enabled,
        colors::text_primary(),
        colors::text_disabled(),
    );

    let color = text_color(b.enabled);
    let mut y = rect.top() + 41.0;
    let count = lines.len();
    for (i, galley) in lines.into_iter().enumerate() {
        let last = i + 1 == count;
        let extra = if last && chevron_inline { 12.0 } else { 0.0 };
        let x = rect.center().x - (galley.size().x + extra) / 2.0;
        let line_width = galley.size().x;
        ui.painter().galley(egui::pos2(x, y), galley, color);
        if last && chevron_inline {
            Icon::ChevronDown.paint(
                ui,
                Rect::from_min_size(egui::pos2(x + line_width + 3.0, y + 3.0), Vec2::splat(9.0)),
                color,
            );
        }
        y += 13.0;
    }
    if b.dropdown && !chevron_inline {
        Icon::ChevronDown.paint(
            ui,
            Rect::from_center_size(egui::pos2(rect.center().x, y + 5.0), Vec2::splat(9.0)),
            color,
        );
    }

    let response = super_tooltip(
        response,
        b.label.replace('\n', " ").as_str(),
        b.keys.map(shortcut).as_deref(),
        b.tip,
    );
    let hit = resolve_hit(ui, &response, &b, menu_zone);
    (response, hit)
}

/// Small button: 16 px icon with its label to the right.
fn small_button(ui: &mut Ui, b: Btn) -> (Response, Hit) {
    let galley =
        ui.fonts(|f| f.layout_no_wrap(b.label.to_owned(), label_font(), Color32::PLACEHOLDER));
    let chevron = if b.dropdown { 14.0 } else { 0.0 };
    let width = 4.0 + 16.0 + 5.0 + galley.size().x + chevron + 6.0;
    let sense = if b.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, SMALL_HEIGHT), sense);

    let menu_zone = Rect::from_min_max(
        egui::pos2(rect.right() - chevron - 2.0, rect.top()),
        rect.max,
    );
    let (fill, stroke) = button_colors(&response, &b);
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, metrics::ROUNDING, fill);
    }
    if let Some(stroke) = stroke {
        ui.painter().rect_stroke(rect, metrics::ROUNDING, stroke);
    }
    if b.split && b.enabled && response.hovered() {
        ui.painter().vline(
            menu_zone.left(),
            rect.y_range().shrink(3.0),
            Stroke::new(1.0, colors::border_strong()),
        );
    }

    let icon_rect = Rect::from_min_size(rect.left_top() + Vec2::new(4.0, 3.0), Vec2::splat(16.0));
    b.icon.paint_state(
        ui,
        icon_rect,
        b.enabled,
        colors::text_primary(),
        colors::text_disabled(),
    );
    let color = text_color(b.enabled);
    ui.painter().galley(
        egui::pos2(
            icon_rect.right() + 5.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    if b.dropdown {
        Icon::ChevronDown.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(rect.right() - 9.0, rect.center().y),
                Vec2::splat(9.0),
            ),
            color,
        );
    }

    let response = super_tooltip(response, b.label, b.keys.map(shortcut).as_deref(), b.tip);
    let hit = resolve_hit(ui, &response, &b, menu_zone);
    (response, hit)
}

/// Icon-only small button (formatting commands such as Bold).
fn icon_button(ui: &mut Ui, b: Btn) -> Response {
    let width = if b.dropdown { 32.0 } else { 22.0 };
    let sense = if b.enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, SMALL_HEIGHT), sense);
    let (fill, stroke) = button_colors(&response, &b);
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, metrics::ROUNDING, fill);
    }
    if let Some(stroke) = stroke {
        ui.painter().rect_stroke(rect, metrics::ROUNDING, stroke);
    }
    let icon_rect = Rect::from_min_size(rect.left_top() + Vec2::new(3.0, 3.0), Vec2::splat(16.0));
    b.icon.paint_state(
        ui,
        icon_rect,
        b.enabled,
        colors::text_primary(),
        colors::text_disabled(),
    );
    if b.dropdown {
        Icon::ChevronDown.paint(
            ui,
            Rect::from_center_size(
                egui::pos2(rect.right() - 6.0, rect.center().y),
                Vec2::splat(8.0),
            ),
            text_color(b.enabled),
        );
    }
    super_tooltip(response, b.tip, b.keys.map(shortcut).as_deref(), "")
}

/// Open a popup menu under `response` when `hit` is `Hit::Menu`.
fn dropdown(ui: &mut Ui, response: &Response, hit: Hit, id: &str, add: impl FnOnce(&mut Ui)) {
    let id = Id::new(id);
    if hit == Hit::Menu {
        ui.memory_mut(|m| m.toggle_popup(id));
    }
    egui::popup::popup_below_widget(
        ui,
        id,
        response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(190.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui);
        },
    );
}

/// One row in a ribbon dropdown menu, with an optional SVG icon and shortcut text.
fn menu_item(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    shortcut_text: Option<String>,
    enabled: bool,
) -> bool {
    let galley =
        ui.fonts(|f| f.layout_no_wrap(label.to_owned(), fonts::body(), Color32::PLACEHOLDER));
    let width = ui.available_width().max(190.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 26.0), sense);
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 0.0, colors::hover());
    }
    if let Some(icon) = icon {
        let icon_rect =
            Rect::from_min_size(rect.left_top() + Vec2::new(8.0, 5.0), Vec2::splat(16.0));
        icon.paint_state(
            ui,
            icon_rect,
            enabled,
            colors::text_primary(),
            colors::text_disabled(),
        );
    }
    let color = text_color(enabled);
    ui.painter().galley(
        egui::pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    if let Some(keys) = shortcut_text {
        ui.painter().text(
            rect.right_center() - Vec2::new(10.0, 0.0),
            Align2::RIGHT_CENTER,
            keys,
            fonts::body(),
            colors::text_secondary(),
        );
    }
    enabled && response.clicked()
}

/// Set the text fit mode for a text frame.
fn set_text_fit_mode(
    state: &mut AppState,
    obj_id: repub_model::ObjectId,
    mode: repub_model::shape::TextFitMode,
) {
    if let Some(page) = state.current_page_mut() {
        if let Some(Shape {
            kind: ShapeKind::TextFrame(tf),
            ..
        }) = page.get_shape_mut(obj_id)
        {
            tf.text_fit = mode;
        }
    }
    state.mark_dirty();
}
