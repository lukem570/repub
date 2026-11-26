//! Repub themes
//!
//! Microsoft Publisher-style Office chrome in a light and a dark palette.
//! [`apply_theme`] installs the egui style; the colours in [`colors`] follow the
//! same choice for everything we paint ourselves.

use egui::{Color32, FontFamily, FontId, Margin, Rounding, Stroke, Style, Visuals};

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Light theme (Publisher style)
    #[default]
    Light,
    /// Dark theme
    Dark,
    /// Follow the desktop's appearance
    System,
}

impl ThemeMode {
    /// The egui theme preference this mode asks for.
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeMode::Light => egui::ThemePreference::Light,
            ThemeMode::Dark => egui::ThemePreference::Dark,
            ThemeMode::System => egui::ThemePreference::System,
        }
    }

    /// Whether the dark palette is in use right now (`System` follows the
    /// desktop).
    pub fn is_dark(self, ctx: &egui::Context) -> bool {
        match self {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => ctx.system_theme() == Some(egui::Theme::Dark),
        }
    }

    /// Name for menus.
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
            ThemeMode::System => "Match System",
        }
    }

    /// Every mode, for menus.
    pub const ALL: [ThemeMode; 3] = [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System];
}

/// Colour palette.
///
/// Every colour is a function so it can follow the active theme: the light
/// values are Publisher/Office, the dark ones their night equivalents. Use
/// [`apply_theme`] to switch; the background texture in `background` reads
/// [`colors::is_dark`] too.
#[allow(dead_code)]
pub mod colors {
    use super::*;
    use std::cell::Cell;

    // The UI, and every colour lookup, happens on the window's thread. Keeping
    // the palette here (rather than in a process-wide static) also lets several
    // app instances in one process, as tests have, use different themes.
    thread_local! {
        /// The selected mode.
        static MODE: Cell<u8> = const { Cell::new(0) };
        /// The palette in use (resolved from `MODE`, so System can change at runtime).
        static DARK: Cell<bool> = const { Cell::new(false) };
    }

    /// Whether the dark palette is in use.
    pub fn is_dark() -> bool {
        DARK.with(Cell::get)
    }

    /// The selected theme mode.
    pub fn mode() -> ThemeMode {
        match MODE.with(Cell::get) {
            1 => ThemeMode::Dark,
            2 => ThemeMode::System,
            _ => ThemeMode::Light,
        }
    }

    pub(crate) fn set_mode(mode: ThemeMode) {
        MODE.with(|m| {
            m.set(match mode {
                ThemeMode::Light => 0,
                ThemeMode::Dark => 1,
                ThemeMode::System => 2,
            })
        });
    }

    pub(crate) fn set_dark(dark: bool) {
        DARK.with(|d| d.set(dark));
    }

    fn pick(light: Color32, dark: Color32) -> Color32 {
        if is_dark() { dark } else { light }
    }

    macro_rules! palette {
        ($($name:ident: $light:expr => $dark:expr;)*) => {
            $(
                /// A theme colour (see the module docs).
                pub fn $name() -> Color32 {
                    pick($light, $dark)
                }
            )*
        };
    }

    palette! {
        // Publisher teal accent (sampled from Publisher 2016/2019)
        accent: Color32::from_rgb(0x07, 0x75, 0x68) => Color32::from_rgb(0x2F, 0xB5, 0xA5);
        accent_hover: Color32::from_rgb(0x18, 0x7E, 0x72) => Color32::from_rgb(0x47, 0xC9, 0xB8);
        accent_light: Color32::from_rgb(0x9E, 0xC8, 0xB7) => Color32::from_rgb(0x26, 0x56, 0x4F); // Thumbnail number strip

        // Backgrounds
        bg_ribbon: Color32::from_rgb(0xF3, 0xF3, 0xF3) => Color32::from_rgb(0x2B, 0x2B, 0x2B);   // Ribbon body and active tab
        bg_panel: Color32::from_rgb(0xE6, 0xE6, 0xE6) => Color32::from_rgb(0x1F, 0x1F, 0x1F);    // Pages pane
        bg_window: Color32::from_rgb(255, 255, 255) => Color32::from_rgb(0x25, 0x25, 0x25);
        bg_toolbar: Color32::from_rgb(0xF3, 0xF3, 0xF3) => Color32::from_rgb(0x25, 0x25, 0x25);
        bg_status: Color32::from_rgb(0xF3, 0xF3, 0xF3) => Color32::from_rgb(0x20, 0x20, 0x20);   // Status bar
        title_bar: Color32::from_rgb(0xE9, 0xEE, 0xF2) => Color32::from_rgb(0x1C, 0x1C, 0x1C);   // Publisher 2021/365 title bar

        // Legacy names for compatibility
        bg_dark: bg_toolbar() => bg_toolbar();
        bg_medium: bg_ribbon() => bg_ribbon();
        bg_light: bg_panel() => bg_panel();
        bg_elevated: bg_window() => bg_window();

        // Workspace (the grain texture in background.rs supplies the actual pixels)
        workspace: Color32::from_rgb(0xE4, 0xE4, 0xE4) => Color32::from_rgb(0x29, 0x29, 0x29);
        page: Color32::WHITE => Color32::WHITE;   // The publication page stays white in dark mode
        page_shadow: Color32::from_rgba_premultiplied(0, 0, 0, 28) => Color32::from_rgba_premultiplied(0, 0, 0, 110);
        page_border: Color32::from_rgb(0xC8, 0xC8, 0xC8) => Color32::from_rgb(0x5A, 0x5A, 0x5A);
        margin_guide: Color32::from_rgb(0x9C, 0xB0, 0xF0) => Color32::from_rgb(0x6C, 0x82, 0xC8);

        // Text
        text_primary: Color32::from_rgb(32, 31, 30) => Color32::from_rgb(0xE6, 0xE6, 0xE6);      // Near black / near white
        text_secondary: Color32::from_rgb(96, 94, 92) => Color32::from_rgb(0xB3, 0xB3, 0xB3);    // Gray
        text_muted: Color32::from_rgb(138, 136, 134) => Color32::from_rgb(0x8C, 0x8C, 0x8C);     // Light gray
        text_disabled: Color32::from_rgb(161, 159, 157) => Color32::from_rgb(0x6A, 0x6A, 0x6A);
        text_inverted: Color32::WHITE => Color32::from_rgb(0x1A, 0x1A, 0x1A);

        // Borders
        border: Color32::from_rgb(225, 223, 221) => Color32::from_rgb(0x3A, 0x3A, 0x3A);
        border_strong: Color32::from_rgb(200, 198, 196) => Color32::from_rgb(0x52, 0x52, 0x52);

        // Interactive
        hover: Color32::from_rgb(237, 235, 233) => Color32::from_rgb(0x3A, 0x3A, 0x3A);
        pressed: Color32::from_rgb(225, 223, 221) => Color32::from_rgb(0x45, 0x45, 0x45);
        selected: Color32::from_rgb(0xCD, 0xE6, 0xDE) => Color32::from_rgb(0x16, 0x50, 0x4A);    // Light / dark teal selection

        // Selection handles
        handle_fill: Color32::WHITE => Color32::WHITE;   // Drawn on the white page
        handle_stroke: accent() => accent();
        selection_outline: accent() => accent();
        rotation_handle: Color32::from_rgb(16, 124, 16) => Color32::from_rgb(0x4C, 0xC9, 0x4C);   // Green
        rotation_line: Color32::from_rgb(16, 124, 16) => Color32::from_rgb(0x4C, 0xC9, 0x4C);

        // Marquee
        marquee_fill: Color32::from_rgba_premultiplied(1, 14, 12, 30) => Color32::from_rgba_premultiplied(47, 181, 165, 30);
        marquee_stroke: accent() => accent();

        // Rulers and guides (Office style)
        ruler_bg: Color32::from_rgb(0xF1, 0xF1, 0xF1) => Color32::from_rgb(0x25, 0x25, 0x25);
        ruler_page_bg: Color32::WHITE => Color32::WHITE;  // Ruler span covering the page
        ruler_tick: Color32::from_rgb(96, 94, 92) => Color32::from_rgb(0xA0, 0xA0, 0xA0);
        ruler_text: Color32::from_rgb(96, 94, 92) => Color32::from_rgb(0xA0, 0xA0, 0xA0);
        ruler_position_indicator: accent() => accent();
        guide: Color32::from_rgb(0, 176, 240) => Color32::from_rgb(0, 176, 240);            // Cyan
        guide_hover: Color32::from_rgb(0, 200, 255) => Color32::from_rgb(0, 200, 255);

        // Grid (drawn on the white page)
        grid_minor: Color32::from_rgba_premultiplied(200, 200, 200, 60) => Color32::from_rgba_premultiplied(200, 200, 200, 60);
        grid_major: Color32::from_rgba_premultiplied(180, 180, 180, 100) => Color32::from_rgba_premultiplied(180, 180, 180, 100);

        // Snap indicators
        snap_indicator: Color32::from_rgb(255, 0, 144) => Color32::from_rgb(255, 0, 144);   // Magenta

        // Status colors
        success: Color32::from_rgb(16, 124, 16) => Color32::from_rgb(0x4C, 0xC9, 0x4C);
        warning: Color32::from_rgb(255, 185, 0) => Color32::from_rgb(0xFF, 0xC8, 0x3D);
        error: Color32::from_rgb(196, 43, 28) => Color32::from_rgb(0xFF, 0x6B, 0x5B);

        // Page navigation pane
        page_thumb_bg: Color32::WHITE => Color32::WHITE;
        page_thumb_border: Color32::from_rgb(200, 198, 196) => Color32::from_rgb(0x55, 0x55, 0x55);
        page_thumb_selected: accent() => accent();

        // Canvas and rulers (Publisher 2016+ style)
        ruler_border: Color32::from_rgb(0xC6, 0xC6, 0xC6) => Color32::from_rgb(0x3E, 0x3E, 0x3E);
        ruler_mouse_indicator: Color32::from_rgb(0x44, 0x44, 0x44) => Color32::from_rgb(0xC8, 0xC8, 0xC8);
        selection_frame: Color32::from_rgb(0x7A, 0x8C, 0xA5) => Color32::from_rgb(0x9F, 0xB3, 0xCC);   // Blue-gray outline
        handle_outline: Color32::from_rgb(0x8A, 0x8A, 0x8A) => Color32::from_rgb(0x6A, 0x6A, 0x6A);
        rotation_glyph: Color32::from_rgb(0x80, 0x80, 0x80) => Color32::from_rgb(0xA0, 0xA0, 0xA0);
        text_boundary: Color32::from_rgb(0xA6, 0xA6, 0xA6) => Color32::from_rgb(0x6A, 0x6A, 0x6A);    // Non-printing text box edge
        text_selection: Color32::from_rgb(0xB4, 0xD5, 0xFE) => Color32::from_rgb(0xB4, 0xD5, 0xFE);   // Selected text highlight (on the white page)
        scrollbar_track: Color32::from_rgb(0xF1, 0xF1, 0xF1) => Color32::from_rgb(0x25, 0x25, 0x25);
        scrollbar_thumb: Color32::from_rgb(0xC2, 0xC2, 0xC2) => Color32::from_rgb(0x4A, 0x4A, 0x4A);
        scrollbar_thumb_hover: Color32::from_rgb(0xA8, 0xA8, 0xA8) => Color32::from_rgb(0x5C, 0x5C, 0x5C);
        scrollbar_thumb_active: Color32::from_rgb(0x8C, 0x8C, 0x8C) => Color32::from_rgb(0x6E, 0x6E, 0x6E);
        scrollbar_arrow: Color32::from_rgb(0x60, 0x60, 0x60) => Color32::from_rgb(0xA0, 0xA0, 0xA0);

        // Start screen / Backstage
        backstage_rail: accent() => accent();
        backstage_rail_hover: Color32::from_rgb(0x1B, 0x86, 0x7A) => Color32::from_rgb(0x3A, 0xA8, 0x9A);
        backstage_rail_selected: Color32::from_rgb(0x0A, 0x5C, 0x52) => Color32::from_rgb(0x2A, 0x7A, 0x70);
        backstage_rail_divider: Color32::from_rgb(0x3A, 0x95, 0x8A) => Color32::from_rgb(0x4E, 0x8F, 0x87);
        backstage_body: Color32::WHITE => Color32::from_rgb(0x1E, 0x1E, 0x1E);
        tile_hover: Color32::from_rgb(0xCD, 0xE6, 0xDE) => Color32::from_rgb(0x2E, 0x4F, 0x4A);
        tile_border: Color32::from_rgb(0xD6, 0xD6, 0xD6) => Color32::from_rgb(0x3C, 0x3C, 0x3C);
    }
}

/// UI metrics.
pub mod metrics {
    pub const SPACE_1: f32 = 4.0;
    pub const SPACE_2: f32 = 8.0;
    pub const SPACE_3: f32 = 12.0;
    pub const SPACE_4: f32 = 16.0;
    pub const SPACE_6: f32 = 24.0;
    pub const SPACE_8: f32 = 32.0;
    
    pub const BUTTON_HEIGHT: f32 = 24.0;
    pub const ICON_SIZE: f32 = 16.0;
    pub const ICON_SIZE_LARGE: f32 = 32.0;
    pub const TOOLBAR_HEIGHT: f32 = 32.0;
    pub const MENU_BAR_HEIGHT: f32 = 24.0;
    /// Tab row (~30) + command body (~94).
    pub const RIBBON_HEIGHT: f32 = 124.0;
    pub const STATUS_BAR_HEIGHT: f32 = 24.0;
    pub const PANEL_WIDTH: f32 = 200.0;
    pub const PAGE_NAV_WIDTH: f32 = 120.0;
    
    pub const ROUNDING: f32 = 2.0;
    /// Custom title row on macOS (matches the native title bar height).
    pub const TITLE_BAR_HEIGHT: f32 = 30.0;
    /// Space reserved for the native traffic-light buttons.
    pub const TRAFFIC_LIGHTS_WIDTH: f32 = 78.0;
    
    pub const SELECTION_OUTLINE_WIDTH: f32 = 2.0;
    pub const HANDLE_SIZE: f32 = 8.0;
    pub const HANDLE_BORDER: f32 = 1.0;
    pub const ROTATION_HANDLE_SIZE: f32 = 10.0;
    pub const ROTATION_HANDLE_DISTANCE: f32 = 20.0;
    pub const GUIDE_HIT_TOLERANCE: f32 = 5.0;
}

/// Fonts.
pub mod fonts {
    use super::*;
    
    /// Font family name for Selawik Semibold (registered in `apply_theme`).
    pub const SEMIBOLD: &str = "selawik-semibold";
    /// Font family name for Selawik Bold (registered in `apply_theme`).
    pub const BOLD: &str = "selawik-bold";

    // Sizes follow Office's 9 pt Segoe UI at 96 dpi (12 px) for most chrome.
    pub fn body() -> FontId {
        FontId::new(12.5, FontFamily::Proportional)
    }
    
    pub fn caption() -> FontId {
        FontId::new(11.5, FontFamily::Proportional)
    }
    
    pub fn small() -> FontId {
        FontId::new(11.0, FontFamily::Proportional)
    }
    
    pub fn heading() -> FontId {
        FontId::new(15.0, FontFamily::Proportional)
    }
    
    pub fn ribbon_tab() -> FontId {
        FontId::new(12.5, FontFamily::Proportional)
    }
    
    pub fn body_sm() -> FontId {
        FontId::new(11.5, FontFamily::Proportional)
    }

    /// Semibold text, e.g. tooltip titles and dialog headings.
    pub fn semibold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
    }

    /// Bold text.
    pub fn bold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(BOLD.into()))
    }
}

/// Register Selawik (Microsoft's open-source Segoe UI fallback, OFL) as the UI font.
/// egui's default fonts stay as fallbacks for symbols Selawik lacks.
fn install_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions};

    let mut defs = FontDefinitions::default();
    let faces = [
        ("selawik", &include_bytes!("../assets/fonts/selawk.ttf")[..]),
        (fonts::SEMIBOLD, &include_bytes!("../assets/fonts/selawksb.ttf")[..]),
        (fonts::BOLD, &include_bytes!("../assets/fonts/selawkb.ttf")[..]),
    ];
    for (name, bytes) in faces {
        defs.font_data.insert(name.to_owned(), FontData::from_static(bytes));
    }

    let fallbacks = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(f) = defs.families.get_mut(&FontFamily::Proportional) {
        f.insert(0, "selawik".to_owned());
    }
    for name in [fonts::SEMIBOLD, fonts::BOLD] {
        let mut family = vec![name.to_owned()];
        family.extend(fallbacks.iter().cloned());
        defs.families.insert(FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(defs);
}

/// One-time setup: UI fonts and the icon/image loaders.
pub fn init(ctx: &egui::Context) {
    install_fonts(ctx);
    // SVG/PNG loaders for `icons::Icon`
    egui_extras::install_image_loaders(ctx);
}

/// Switch to `mode` and rebuild the egui style.
///
/// Both egui theme slots get a matching style, so "Match System Appearance"
/// can hand control to the desktop without a light flash. Cheap enough to call
/// whenever the mode changes; [`sync`] does that for System.
pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    let light = build_style(false);
    let dark = build_style(true);
    colors::set_mode(mode);
    colors::set_dark(mode.is_dark(ctx));
    ctx.set_theme(mode.preference());
    ctx.set_style_of(egui::Theme::Light, light);
    ctx.set_style_of(egui::Theme::Dark, dark);
}

/// Follow the desktop when the mode is `System`, applying the other palette
/// when the appearance changes. A no-op in the fixed modes.
pub fn sync(ctx: &egui::Context, mode: ThemeMode) {
    if mode.is_dark(ctx) != colors::is_dark() {
        apply_theme(ctx, mode);
    }
}

/// The egui style for one palette.
fn build_style(dark: bool) -> Style {
    colors::set_dark(dark);
    let mut style = Style::default();
    let r = Rounding::same(metrics::ROUNDING);

    let mut visuals = if dark { Visuals::dark() } else { Visuals::light() };

    // Windows and dialogs: body, hairline border, small soft shadow
    visuals.window_fill = colors::bg_window();
    visuals.window_stroke = Stroke::new(1.0, colors::border_strong());
    visuals.window_rounding = r;
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark { 120 } else { 40 }),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark { 90 } else { 30 }),
    };
    visuals.menu_rounding = r;
    visuals.panel_fill = colors::bg_panel();

    // Labels, separators, dialog title bands
    visuals.widgets.noninteractive.bg_fill = colors::bg_window();
    visuals.widgets.noninteractive.weak_bg_fill = colors::bg_window();
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors::border());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors::text_primary());
    visuals.widgets.noninteractive.rounding = r;

    // Office buttons: filled with a hairline border
    visuals.widgets.inactive.bg_fill = colors::bg_window();
    visuals.widgets.inactive.weak_bg_fill = colors::bg_window();
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors::border_strong());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::text_primary());
    visuals.widgets.inactive.rounding = r;
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.bg_fill = colors::hover();
    visuals.widgets.hovered.weak_bg_fill = colors::hover();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, if dark { Color32::from_gray(0x6A) } else { Color32::from_gray(0xA6) });
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors::text_primary());
    visuals.widgets.hovered.rounding = r;
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = colors::selected();
    visuals.widgets.active.weak_bg_fill = colors::selected();
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors::accent());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors::text_primary());
    visuals.widgets.active.rounding = r;
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open.bg_fill = colors::selected();
    visuals.widgets.open.weak_bg_fill = colors::selected();
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors::accent());
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors::text_primary());
    visuals.widgets.open.rounding = r;

    // Selection and focus ring
    visuals.selection.bg_fill = colors::selected();
    visuals.selection.stroke = Stroke::new(1.0, colors::accent());
    visuals.hyperlink_color = colors::accent();

    // Text fields and slider rails draw on extreme_bg_color
    visuals.extreme_bg_color = colors::bg_window();
    visuals.faint_bg_color = colors::bg_ribbon();
    visuals.text_cursor.stroke = Stroke::new(1.5, colors::text_primary());

    // Slider: rectangular thumb like Office's zoom slider
    visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.45 };
    visuals.slider_trailing_fill = false;

    style.visuals = visuals;

    // Spacing
    style.spacing.item_spacing = egui::vec2(metrics::SPACE_2, metrics::SPACE_1);
    style.spacing.button_padding = egui::vec2(10.0, 3.0);
    style.spacing.indent = metrics::SPACE_4;
    style.spacing.interact_size.y = 22.0;
    style.spacing.slider_rail_height = 3.0;
    style.spacing.combo_width = 120.0;
    style.spacing.window_margin = Margin::same(16.0);

    // Text styles
    use egui::TextStyle;
    style.text_styles = [
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(17.0, FontFamily::Name(fonts::SEMIBOLD.into()))),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();

    style
}

/// Office "super tooltip": semibold command name with its shortcut, then a one-line description.
pub fn super_tooltip(response: egui::Response, title: &str, shortcut: Option<&str>, description: &str) -> egui::Response {
    response.on_hover_ui(|ui| {
        ui.set_max_width(260.0);
        let heading = match shortcut {
            Some(keys) => format!("{title} ({keys})"),
            None => title.to_owned(),
        };
        ui.label(egui::RichText::new(heading).font(fonts::semibold(12.5)).color(colors::text_primary()));
        if !description.is_empty() {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(description).font(fonts::body()).color(colors::text_secondary()));
        }
    })
}

/// Shortcut label using the platform's command key: `shortcut("Shift+N")` gives
/// "⇧⌘N" on macOS and "Ctrl+Shift+N" elsewhere.
pub fn shortcut(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        let shift = if keys.contains("Shift+") { "⇧" } else { "" };
        let alt = if keys.contains("Alt+") { "⌥" } else { "" };
        let key = keys.replace("Shift+", "").replace("Alt+", "");
        format!("{alt}{shift}⌘{key}")
    } else {
        format!("Ctrl+{keys}")
    }
}

/// Frame style for the ribbon/toolbar area.
pub fn ribbon_frame() -> egui::Frame {
    // Panel fill is the command-body gray; the toolbar paints the tab row white itself
    egui::Frame::none()
        .fill(colors::bg_ribbon())
        .inner_margin(Margin { left: 0.0, right: 0.0, top: 2.0, bottom: 0.0 })
}

/// Frame style for the simple toolbar.
pub fn toolbar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::bg_toolbar())
        .stroke(Stroke::new(1.0, colors::border()))
        .inner_margin(Margin::symmetric(4.0, 2.0))
}

/// Frame style for side panels.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::bg_panel())
        .stroke(Stroke::new(1.0, colors::border()))
        .inner_margin(Margin::same(0.0))
}

/// Frame style for side panels (alias).
pub fn side_panel_frame() -> egui::Frame {
    panel_frame()
}

/// Frame style for the page navigation pane.
pub fn page_nav_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::bg_panel())
        .stroke(Stroke::new(1.0, colors::border()))
        .inner_margin(Margin::same(4.0))
}

/// Frame style for the status bar.
pub fn status_bar_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::bg_status())
        .stroke(Stroke::new(1.0, colors::border()))
        .inner_margin(Margin::symmetric(8.0, 2.0))
}

/// Frame style for the canvas area.
pub fn canvas_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::workspace())
        .inner_margin(Margin::same(0.0))
}

/// Panel header with Office-style formatting.
pub fn panel_header(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(metrics::SPACE_2);
        ui.label(
            egui::RichText::new(text)
                .font(fonts::heading())
                .color(colors::text_primary())
                .strong(),
        );
    });
    ui.add_space(metrics::SPACE_1);
    ui.separator();
    ui.add_space(metrics::SPACE_1);
}

/// Section header within a panel.
pub fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(metrics::SPACE_2);
    ui.horizontal(|ui| {
        ui.add_space(metrics::SPACE_2);
        ui.label(
            egui::RichText::new(text)
                .font(fonts::caption())
                .color(colors::text_secondary())
                .strong(),
        );
    });
    ui.add_space(metrics::SPACE_1);
}
