//! Object manipulation tools.

use repub_color::Color;
use repub_geometry::{Point, Rect};
use repub_model::geometry::ShapeGeometry;
use repub_model::shape::{ImageFit, ImageSource, Shape, ShapeKind, TextTransform, WordArtPreset};
use repub_model::styles::{Fill, GradientStop, LinearGradient, Stroke};
use repub_model::text::StyledText;
use serde::Deserialize;
use serde_json::{json, Value};

use super::Tool;
use crate::error::{McpError, McpResult};
use crate::protocol::{ToolDefinition, ToolResult};
use crate::session::Session;

/// Add a text frame.
pub struct AddTextFrame;

#[derive(Debug, Deserialize)]
struct AddTextFrameArgs {
    page: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    text: Option<String>,
    style: Option<TextStyleArgs>,
}

#[derive(Debug, Deserialize)]
struct TextStyleArgs {
    font_family: Option<String>,
    font_size: Option<f64>,
    color: Option<String>,
}

impl Tool for AddTextFrame {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_add_text_frame".to_string(),
            description: "Add a text frame to a page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page index (0-based)"
                    },
                    "x": {
                        "type": "number",
                        "description": "X position in points"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y position in points"
                    },
                    "width": {
                        "type": "number",
                        "description": "Width in points"
                    },
                    "height": {
                        "type": "number",
                        "description": "Height in points"
                    },
                    "text": {
                        "type": "string",
                        "description": "Initial text content"
                    },
                    "style": {
                        "type": "object",
                        "description": "Text styling",
                        "properties": {
                            "font_family": {"type": "string"},
                            "font_size": {"type": "number"},
                            "color": {"type": "string", "description": "Hex color (e.g., 'ff0000')"}
                        }
                    }
                },
                "required": ["page", "x", "y", "width", "height"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: AddTextFrameArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document_mut()?;
        let page_count = doc.pages().len();

        if args.page >= page_count {
            return Err(McpError::InvalidPage(args.page, page_count));
        }

        let bounds = Rect::new(args.x, args.y, args.width, args.height);
        let mut frame = Shape::text_frame(bounds);

        if let Some(text) = args.text {
            if let ShapeKind::TextFrame(data) = &mut frame.kind {
                data.content = StyledText::plain(&text);
            }
        }

        // Note: Style application would need to be implemented in the text frame.
        // For now, we just create the frame with default styling.

        let obj_id = frame.id();

        doc.pages_mut()[args.page].add_shape(frame);
        let str_id = session.register_object(obj_id);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Added text frame to page {}", args.page),
            "object_id": str_id,
            "bounds": {
                "x": args.x,
                "y": args.y,
                "width": args.width,
                "height": args.height
            }
        })))
    }
}

/// Add a shape.
pub struct AddShape;

#[derive(Debug, Deserialize)]
struct AddShapeArgs {
    page: usize,
    shape_type: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: Option<FillArgs>,
    stroke: Option<StrokeArgs>,
    corner_radius: Option<f64>,
    sides: Option<u32>,
    points: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FillArgs {
    color: Option<String>,
    opacity: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct StrokeArgs {
    color: Option<String>,
    width: Option<f64>,
}

impl Tool for AddShape {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_add_shape".to_string(),
            description: "Add a shape to a page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page index (0-based)"
                    },
                    "shape_type": {
                        "type": "string",
                        "enum": ["rectangle", "ellipse", "polygon", "star", "line"],
                        "description": "Type of shape"
                    },
                    "x": {"type": "number", "description": "X position in points"},
                    "y": {"type": "number", "description": "Y position in points"},
                    "width": {"type": "number", "description": "Width in points"},
                    "height": {"type": "number", "description": "Height in points"},
                    "fill": {
                        "type": "object",
                        "properties": {
                            "color": {"type": "string", "description": "Hex color"},
                            "opacity": {"type": "number", "minimum": 0, "maximum": 1}
                        }
                    },
                    "stroke": {
                        "type": "object",
                        "properties": {
                            "color": {"type": "string", "description": "Hex color"},
                            "width": {"type": "number"}
                        }
                    },
                    "corner_radius": {"type": "number", "description": "Corner radius for rectangles"},
                    "sides": {"type": "integer", "description": "Number of sides for polygons", "minimum": 3},
                    "points": {"type": "integer", "description": "Number of points for stars", "minimum": 3}
                },
                "required": ["page", "shape_type", "x", "y", "width", "height"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: AddShapeArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document_mut()?;
        let page_count = doc.pages().len();

        if args.page >= page_count {
            return Err(McpError::InvalidPage(args.page, page_count));
        }

        let bounds = Rect::new(args.x, args.y, args.width, args.height);

        let mut shape = match args.shape_type.to_lowercase().as_str() {
            "rectangle" => {
                if let Some(r) = args.corner_radius {
                    Shape::rounded_rectangle(bounds, r)
                } else {
                    Shape::rectangle(bounds)
                }
            }
            "ellipse" => Shape::ellipse(bounds),
            "polygon" => {
                let sides = args.sides.unwrap_or(6);
                Shape::polygon(bounds, sides)
            }
            "star" => {
                let points = args.points.unwrap_or(5);
                Shape::star(bounds, points, 0.5)
            }
            _ => {
                return Err(McpError::InvalidArgument(format!(
                    "Unknown shape type: {}",
                    args.shape_type
                )))
            }
        };

        // Apply fill
        if let Some(fill) = args.fill {
            if let Some(color_hex) = fill.color {
                if let Some(mut color) = Color::from_hex(&color_hex) {
                    if let Some(opacity) = fill.opacity {
                        color.a = opacity;
                    }
                    shape.fill = Fill::Solid(color);
                }
            }
        }

        // Apply stroke
        if let Some(stroke_args) = args.stroke {
            let color = stroke_args
                .color
                .and_then(|h| Color::from_hex(&h))
                .unwrap_or(Color::BLACK);
            let width = stroke_args.width.unwrap_or(1.0);
            shape.line = Some(Stroke::new(color, width));
        }

        let obj_id = shape.id();

        doc.pages_mut()[args.page].add_shape(shape);
        let str_id = session.register_object(obj_id);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Added {} to page {}", args.shape_type, args.page),
            "object_id": str_id,
            "shape_type": args.shape_type
        })))
    }
}

/// Add an image.
pub struct AddImage;

#[derive(Debug, Deserialize)]
struct AddImageArgs {
    page: usize,
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
    source: String,
    fit: Option<String>,
}

impl Tool for AddImage {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_add_image".to_string(),
            description: "Add an image to a page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {"type": "integer", "description": "Page index (0-based)"},
                    "x": {"type": "number", "description": "X position in points"},
                    "y": {"type": "number", "description": "Y position in points"},
                    "width": {"type": "number", "description": "Width in points (optional)"},
                    "height": {"type": "number", "description": "Height in points (optional)"},
                    "source": {"type": "string", "description": "Image file path or URL"},
                    "fit": {
                        "type": "string",
                        "enum": ["fill", "fit", "stretch", "tile", "center"],
                        "default": "fit"
                    }
                },
                "required": ["page", "x", "y", "source"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: AddImageArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document_mut()?;
        let page_count = doc.pages().len();

        if args.page >= page_count {
            return Err(McpError::InvalidPage(args.page, page_count));
        }

        let width = args.width.unwrap_or(200.0);
        let height = args.height.unwrap_or(200.0);
        let bounds = Rect::new(args.x, args.y, width, height);

        let mut frame = Shape::picture(bounds);

        if let ShapeKind::Picture(picture) = &mut frame.kind {
            // Set image source
            if args.source.starts_with("http://") || args.source.starts_with("https://") {
                picture.source = ImageSource::Url(args.source.clone());
            } else {
                picture.source = ImageSource::File(args.source.clone());
            }

            // Set fit mode
            if let Some(fit) = args.fit {
                picture.fit = match fit.to_lowercase().as_str() {
                    "fill" => ImageFit::Fill,
                    "fit" => ImageFit::Fit,
                    "stretch" => ImageFit::Stretch,
                    "tile" => ImageFit::Tile,
                    "center" => ImageFit::Center,
                    _ => ImageFit::Fit,
                };
            }
        }

        let obj_id = frame.id();

        doc.pages_mut()[args.page].add_shape(frame);
        let str_id = session.register_object(obj_id);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Added image to page {}", args.page),
            "object_id": str_id,
            "source": args.source
        })))
    }
}

/// List objects on a page.
pub struct ListObjects;

#[derive(Debug, Deserialize)]
struct ListObjectsArgs {
    page: Option<usize>,
}

impl Tool for ListObjects {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_list_objects".to_string(),
            description: "List objects in the document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page index to list (omit for all pages)"
                    }
                }
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: ListObjectsArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document()?;

        let mut objects = Vec::new();

        let pages_to_check: Vec<usize> = if let Some(p) = args.page {
            if p >= doc.pages().len() {
                return Err(McpError::InvalidPage(p, doc.pages().len()));
            }
            vec![p]
        } else {
            (0..doc.pages().len()).collect()
        };

        for page_idx in pages_to_check {
            let page = &doc.pages()[page_idx];
            for layer in page.layers() {
                for obj in layer.shapes() {
                    let bounds = &obj.common.bounds;
                    let obj_type = match &obj.kind {
                        ShapeKind::AutoShape(auto) => {
                            let shape_type = match &auto.geometry {
                                ShapeGeometry::Rectangle { .. } => "rectangle",
                                ShapeGeometry::Ellipse => "ellipse",
                                ShapeGeometry::Polygon { .. } => "polygon",
                                ShapeGeometry::Star { .. } => "star",
                                ShapeGeometry::Path(_) => "path",
                            };
                            format!("shape:{}", shape_type)
                        }
                        ShapeKind::TextFrame(_) => "text_frame".to_string(),
                        ShapeKind::Picture(_) => "image".to_string(),
                        ShapeKind::Table(_) => "table".to_string(),
                        ShapeKind::Line(_) => continue, // Skip lines for now
                        ShapeKind::Group(_) => "group".to_string(),
                        ShapeKind::TextEffect(_) => "wordart".to_string(),
                    };

                    objects.push(json!({
                        "page": page_idx,
                        "layer": layer.name,
                        "type": obj_type,
                        "bounds": {
                            "x": bounds.origin.x,
                            "y": bounds.origin.y,
                            "width": bounds.width(),
                            "height": bounds.height()
                        }
                    }));
                }
            }
        }

        Ok(ToolResult::json(json!({
            "total": objects.len(),
            "objects": objects
        })))
    }
}

/// Add WordArt.
pub struct AddWordArt;

#[derive(Debug, Deserialize)]
struct AddWordArtArgs {
    page: usize,
    text: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    preset: Option<String>,
    fill: Option<WordArtFillArgs>,
    outline: Option<StrokeArgs>,
}

#[derive(Debug, Deserialize)]
struct WordArtFillArgs {
    #[serde(rename = "type")]
    fill_type: Option<String>,
    color: Option<String>,
    gradient_stops: Option<Vec<GradientStopArgs>>,
    gradient_angle: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GradientStopArgs {
    position: f64,
    color: String,
}

impl Tool for AddWordArt {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_add_wordart".to_string(),
            description:
                "Add a WordArt object (decorative text with fill, outline, and transform effects)"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page index (0-based)"
                    },
                    "text": {
                        "type": "string",
                        "description": "The text to display"
                    },
                    "x": {"type": "number", "description": "X position in points"},
                    "y": {"type": "number", "description": "Y position in points"},
                    "width": {"type": "number", "description": "Width in points"},
                    "height": {"type": "number", "description": "Height in points"},
                    "preset": {
                        "type": "string",
                        "enum": [
                            "plain", "outline", "shadow", "threed",
                            "arch_up", "arch_down", "circle", "wave",
                            "inflate", "deflate", "slant_up", "slant_down",
                            "fade_right", "fade_left", "chevron_up", "chevron_down",
                            "triangle_up", "triangle_down", "can_up", "can_down",
                            "cascade_up", "cascade_down", "stop", "ring_inside",
                            "ring_outside", "button"
                        ],
                        "description": "WordArt preset style"
                    },
                    "fill": {
                        "type": "object",
                        "description": "Fill styling",
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["solid", "gradient"],
                                "default": "solid"
                            },
                            "color": {"type": "string", "description": "Hex color for solid fill"},
                            "gradient_stops": {
                                "type": "array",
                                "description": "Gradient color stops",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "position": {"type": "number", "minimum": 0, "maximum": 1},
                                        "color": {"type": "string"}
                                    },
                                    "required": ["position", "color"]
                                }
                            },
                            "gradient_angle": {"type": "number", "description": "Gradient angle in degrees"}
                        }
                    },
                    "outline": {
                        "type": "object",
                        "properties": {
                            "color": {"type": "string", "description": "Hex color"},
                            "width": {"type": "number", "default": 1.0}
                        }
                    }
                },
                "required": ["page", "text", "x", "y", "width", "height"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: AddWordArtArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document_mut()?;
        let page_count = doc.pages().len();

        if args.page >= page_count {
            return Err(McpError::InvalidPage(args.page, page_count));
        }

        let bounds = Rect::new(args.x, args.y, args.width, args.height);
        let mut wordart = Shape::text_effect(bounds, &args.text);

        // Parse preset
        if let Some(preset_str) = &args.preset {
            let preset = match preset_str.to_lowercase().as_str() {
                "plain" => WordArtPreset::Plain,
                "outline" => WordArtPreset::Outline,
                "shadow" => WordArtPreset::Shadow,
                "threed" | "3d" => WordArtPreset::ThreeD,
                "arch_up" | "archup" => WordArtPreset::ArchUp,
                "arch_down" | "archdown" => WordArtPreset::ArchDown,
                "circle" => WordArtPreset::Circle,
                "wave" => WordArtPreset::Wave,
                "inflate" => WordArtPreset::Inflate,
                "deflate" => WordArtPreset::Deflate,
                "slant_up" | "slantup" => WordArtPreset::SlantUp,
                "slant_down" | "slantdown" => WordArtPreset::SlantDown,
                "fade_right" | "faderight" => WordArtPreset::FadeRight,
                "fade_left" | "fadeleft" => WordArtPreset::FadeLeft,
                "chevron_up" | "chevronup" => WordArtPreset::ChevronUp,
                "chevron_down" | "chevrondown" => WordArtPreset::ChevronDown,
                "triangle_up" | "triangleup" => WordArtPreset::TriangleUp,
                "triangle_down" | "triangledown" => WordArtPreset::TriangleDown,
                "can_up" | "canup" => WordArtPreset::CanUp,
                "can_down" | "candown" => WordArtPreset::CanDown,
                "cascade_up" | "cascadeup" => WordArtPreset::CascadeUp,
                "cascade_down" | "cascadedown" => WordArtPreset::CascadeDown,
                "stop" => WordArtPreset::Stop,
                "ring_inside" | "ringinside" => WordArtPreset::RingInside,
                "ring_outside" | "ringoutside" => WordArtPreset::RingOutside,
                "button" => WordArtPreset::Button,
                _ => WordArtPreset::Plain,
            };
            if let ShapeKind::TextEffect(data) = &mut wordart.kind {
                data.transform = TextTransform::Preset(preset);
            }
        }

        // Parse fill
        if let Some(fill_args) = args.fill {
            let fill_type = fill_args.fill_type.as_deref().unwrap_or("solid");

            if fill_type == "gradient" {
                if let Some(stops) = fill_args.gradient_stops {
                    let gradient_stops: Vec<GradientStop> = stops
                        .iter()
                        .filter_map(|s| {
                            Color::from_hex(&s.color).map(|c| GradientStop {
                                position: s.position,
                                color: c,
                            })
                        })
                        .collect();

                    if !gradient_stops.is_empty() {
                        let angle = fill_args.gradient_angle.unwrap_or(90.0);
                        // Convert angle to start/end points
                        let radians = angle.to_radians();
                        let (sin, cos) = radians.sin_cos();
                        let gradient = LinearGradient {
                            start: Point::new(0.5 - cos * 0.5, 0.5 - sin * 0.5),
                            end: Point::new(0.5 + cos * 0.5, 0.5 + sin * 0.5),
                            stops: gradient_stops,
                        };
                        wordart.fill = Fill::LinearGradient(gradient);
                    }
                }
            } else if let Some(color_hex) = fill_args.color {
                if let Some(color) = Color::from_hex(&color_hex) {
                    wordart.fill = Fill::Solid(color);
                }
            }
        }

        // Parse outline
        if let Some(outline_args) = args.outline {
            let color = outline_args
                .color
                .and_then(|h| Color::from_hex(&h))
                .unwrap_or(Color::BLACK);
            let width = outline_args.width.unwrap_or(1.0);
            wordart.line = Some(Stroke::new(color, width));
        }

        let obj_id = wordart.id();

        doc.pages_mut()[args.page].add_shape(wordart);
        let str_id = session.register_object(obj_id);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Added WordArt '{}' to page {}", args.text, args.page),
            "object_id": str_id,
            "preset": args.preset.unwrap_or_else(|| "plain".to_string())
        })))
    }
}

/// Delete an object.
pub struct DeleteObject;

#[derive(Debug, Deserialize)]
struct DeleteObjectArgs {
    object_id: String,
}

impl Tool for DeleteObject {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_delete_object".to_string(),
            description: "Delete an object from the document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "object_id": {
                        "type": "string",
                        "description": "ID of the object to delete"
                    }
                },
                "required": ["object_id"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: DeleteObjectArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        // For now, return a placeholder - full implementation would need
        // object tracking in the session
        Ok(ToolResult::json(json!({
            "success": false,
            "message": "Delete by object_id not yet implemented. Objects can be identified by listing them."
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{Document, PageOrientation, PageSize};

    fn setup_session() -> Session {
        let mut session = Session::new();
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        session.create_document(doc);
        session
    }

    #[test]
    fn test_add_text_frame() {
        let tool = AddTextFrame;
        let mut session = setup_session();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "page": 0,
                    "x": 72.0,
                    "y": 72.0,
                    "width": 200.0,
                    "height": 100.0,
                    "text": "Hello, World!"
                }),
            )
            .unwrap();

        assert!(!result.is_error);
    }

    #[test]
    fn test_add_shape() {
        let tool = AddShape;
        let mut session = setup_session();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "page": 0,
                    "shape_type": "rectangle",
                    "x": 100.0,
                    "y": 100.0,
                    "width": 150.0,
                    "height": 100.0,
                    "fill": {"color": "ff0000"}
                }),
            )
            .unwrap();

        assert!(!result.is_error);
    }

    #[test]
    fn test_list_objects() {
        let tool = ListObjects;
        let mut session = setup_session();

        // Add an object first
        let add_tool = AddShape;
        add_tool
            .execute(
                &mut session,
                json!({
                    "page": 0,
                    "shape_type": "ellipse",
                    "x": 50.0,
                    "y": 50.0,
                    "width": 100.0,
                    "height": 100.0
                }),
            )
            .unwrap();

        let result = tool.execute(&mut session, json!({})).unwrap();
        assert!(!result.is_error);
    }
}
