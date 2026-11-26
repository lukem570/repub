//! Dialog windows.

pub mod about;
pub mod alerts;
pub mod controls;
pub mod document_properties;
pub mod export;
pub mod find_replace;
pub mod font;
pub mod format_shape;

pub mod hyperlink;
pub mod insert_table;
pub mod move_page;
pub mod new_document;
pub mod page_properties;
pub mod paragraph;
pub mod rename_page;
pub mod spelling;
pub mod table_properties;
pub mod templates;
pub mod text_box_margins;
pub mod text_columns;
pub mod text_fit;
pub mod text_wrap;
pub mod wordart;

pub use about::AboutDialog;
pub use alerts::{ErrorAlert, SaveChangesPrompt, SaveChoice};
pub use document_properties::DocumentPropertiesDialog;
pub use export::ExportDialog;
pub use find_replace::FindReplaceDialog;
pub use font::FontDialog;
pub use format_shape::FormatShapeDialog;

pub use hyperlink::HyperlinkDialog;
pub use insert_table::InsertTableDialog;
pub use move_page::MovePageDialog;
pub use new_document::NewDocumentDialog;
pub use page_properties::PagePropertiesDialog;
pub use paragraph::ParagraphDialog;
pub use rename_page::RenamePageDialog;
pub use spelling::SpellingDialog;
pub use table_properties::TablePropertiesDialog;
pub use templates::TemplatesDialog;
pub use text_box_margins::TextBoxMarginsDialog;
pub use text_columns::TextColumnsDialog;
pub use text_fit::TextFitDialog;
pub use text_wrap::TextWrapDialog;
pub use wordart::WordArtDialog;
