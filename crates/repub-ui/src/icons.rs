//! Ribbon and UI icons.
//!
//! SVGs from Microsoft Fluent UI System Icons (MIT, see `assets/icons/LICENSE`),
//! rasterized by the egui_extras SVG loader at the on-screen pixel size.
//! Monochrome icons are filled white so `Image::tint` gives them any color;
//! `*Color` variants keep their built-in multi-color artwork.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use egui::{Color32, Image, ImageSource, Vec2};

macro_rules! icons {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// An icon from `assets/icons/`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Icon { $($variant),* }

        impl Icon {
            fn file(self) -> &'static str {
                match self { $(Icon::$variant => $file),* }
            }

            fn svg(self) -> &'static [u8] {
                match self { $(Icon::$variant => include_bytes!(concat!("../assets/icons/", $file, ".svg"))),* }
            }
        }
    };
}

icons! {
    Add => "add",
    Delete => "delete",
    Eye => "eye",
    EyeOff => "eye_off",
    LockClosed => "lock_closed",
    LockOpen => "lock_open",
    AlignBottom => "align_bottom",
    AlignCenterHorizontal => "align_center_horizontal",
    AlignCenterVertical => "align_center_vertical",
    AlignLeft => "align_left",
    AlignRight => "align_right",
    AlignTop => "align_top",
    ArrowBidirectionalLeftRight => "arrow_bidirectional_left_right",
    ArrowBidirectionalUpDown => "arrow_bidirectional_up_down",
    ArrowDownRight => "arrow_down_right",
    ArrowExportLtr => "arrow_export_ltr",
    ArrowLeft => "arrow_left",
    ArrowMaximize => "arrow_maximize",
    ArrowRedo => "arrow_redo",
    ArrowRotateClockwise => "arrow_rotate_clockwise",
    ArrowRotateCounterclockwise => "arrow_rotate_counterclockwise",
    ArrowSwap => "arrow_swap",
    ArrowUndo => "arrow_undo",
    BookOpen => "book_open",
    BookOpenColor => "book_open_color",
    Calendar => "calendar",
    CalendarColor => "calendar_color",
    Checkmark => "checkmark",
    ChevronDown => "chevron_down",
    ChevronLeft => "chevron_left",
    ChevronRight => "chevron_right",
    ChevronUp => "chevron_up",
    CircleSmall => "circle_small",
    ClipboardPaste => "clipboard_paste",
    ColorFill => "color_fill",
    ColorLine => "color_line",
    Copy => "copy",
    Cursor => "cursor",
    CursorClick => "cursor_click",
    Cut => "cut",
    Dismiss => "dismiss",
    Document => "document",
    DocumentAdd => "document_add",
    DocumentAddColor => "document_add_color",
    DocumentColor => "document_color",
    DocumentCopy => "document_copy",
    DocumentFooter => "document_footer",
    DocumentHeader => "document_header",
    DocumentMargins => "document_margins",
    DocumentOnePage => "document_one_page",
    DocumentText => "document_text",
    DrawShape => "draw_shape",
    Edit => "edit",
    FlipHorizontal => "flip_horizontal",
    FlipVertical => "flip_vertical",
    FolderOpen => "folder_open",
    FontDecrease => "font_decrease",
    FontIncrease => "font_increase",
    FullScreenMaximize => "full_screen_maximize",
    Grid => "grid",
    Group => "group",
    GroupDismiss => "group_dismiss",
    HandLeft => "hand_left",
    Hexagon => "hexagon",
    Image => "image",
    ImageColor => "image_color",
    Info => "info",
    Layer => "layer",
    Line => "line",
    Link => "link",
    LinkColor => "link_color",
    LinkDismiss => "link_dismiss",
    NumberSymbol => "number_symbol",
    Orientation => "orientation",
    Oval => "oval",
    PageFit => "page_fit",
    PaintBrush => "paint_brush",
    PaintBrushColor => "paint_brush_color",
    PaintBucket => "paint_bucket",
    PositionBackward => "position_backward",
    PositionForward => "position_forward",
    PositionToBack => "position_to_back",
    PositionToFront => "position_to_front",
    Print => "print",
    Record => "record",
    RectangleLandscape => "rectangle_landscape",
    Resize => "resize",
    Ruler => "ruler",
    Save => "save",
    Search => "search",
    SelectObject => "select_object",
    Shapes => "shapes",
    Star => "star",
    StarColor => "star_color",
    Subtract => "subtract",
    Table => "table",
    TableColor => "table_color",
    TableDeleteColumn => "table_delete_column",
    TableDeleteRow => "table_delete_row",
    TableDismiss => "table_dismiss",
    TableInsertColumn => "table_insert_column",
    TableInsertRow => "table_insert_row",
    TextAlignCenter => "text_align_center",
    TextAlignJustify => "text_align_justify",
    TextAlignLeft => "text_align_left",
    TextAlignRight => "text_align_right",
    TextBold => "text_bold",
    TextBoxSettings => "text_box_settings",
    TextBulletListLtr => "text_bullet_list_ltr",
    TextCaseTitle => "text_case_title",
    TextColor => "text_color",
    TextColumnTwo => "text_column_two",
    TextEffects => "text_effects",
    TextFont => "text_font",
    TextIndentDecreaseLtr => "text_indent_decrease_ltr",
    TextIndentIncreaseLtr => "text_indent_increase_ltr",
    TextItalic => "text_italic",
    TextLineSpacing => "text_line_spacing",
    TextNumberListLtr => "text_number_list_ltr",
    TextParagraph => "text_paragraph",
    TextSubscript => "text_subscript",
    TextSuperscript => "text_superscript",
    TextUnderline => "text_underline",
    TextWrap => "text_wrap",
    Textbox => "textbox",
    ZoomIn => "zoom_in",
    ZoomOut => "zoom_out",
}

impl Icon {
    /// Whether this icon carries its own colors (no tint applied).
    pub fn is_multicolor(self) -> bool {
        self.file().ends_with("_color")
    }

    /// SVG bytes ready for the loader. Monochrome icons get `fill="#FFFFFF"` injected.
    fn bytes(self) -> Arc<[u8]> {
        static CACHE: OnceLock<Mutex<HashMap<Icon, Arc<[u8]>>>> = OnceLock::new();
        let mut cache = CACHE
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry(self)
            .or_insert_with(|| {
                if self.is_multicolor() {
                    Arc::from(self.svg())
                } else {
                    let text = String::from_utf8_lossy(self.svg());
                    Arc::from(
                        text.replacen("<svg ", "<svg fill=\"#FFFFFF\" ", 1)
                            .into_bytes(),
                    )
                }
            })
            .clone()
    }

    /// An `egui::Image` of this icon at `size` points, tinted `color` (ignored for multi-color icons).
    pub fn image(self, size: f32, color: Color32) -> Image<'static> {
        let source = ImageSource::Bytes {
            uri: Cow::Owned(format!("bytes://icon/{}.svg", self.file())),
            bytes: egui::load::Bytes::Shared(self.bytes()),
        };
        let image = Image::new(source).fit_to_exact_size(Vec2::splat(size));
        if self.is_multicolor() {
            image
        } else {
            image.tint(color)
        }
    }

    /// Paint this icon into `rect`, tinted `color` (ignored for multi-color icons).
    pub fn paint(self, ui: &egui::Ui, rect: egui::Rect, color: Color32) {
        self.image(rect.width().min(rect.height()), color)
            .paint_at(ui, rect);
    }

    /// Paint this icon for a command that may be disabled: monochrome icons use
    /// `color` or `disabled`, multi-color icons are faded when disabled.
    pub fn paint_state(
        self,
        ui: &egui::Ui,
        rect: egui::Rect,
        enabled: bool,
        color: Color32,
        disabled: Color32,
    ) {
        let size = rect.width().min(rect.height());
        let image = if self.is_multicolor() {
            let image = self.image(size, color);
            if enabled {
                image
            } else {
                image.tint(Color32::from_white_alpha(90))
            }
        } else {
            self.image(size, if enabled { color } else { disabled })
        };
        image.paint_at(ui, rect);
    }
}
