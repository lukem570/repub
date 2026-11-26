//! Microsoft Office Escher (Office Drawing) parser.
//!
//! This module implements parsing of the Escher binary format used by Microsoft Office
//! applications (including Publisher) for storing drawing objects. The format is documented
//! in the MS-ODRAW specification.
//!
//! ## Format Overview
//!
//! Escher uses a hierarchical record-based structure:
//! - **Containers**: Hold other records (DggContainer, DgContainer, SpContainer, etc.)
//! - **Atoms**: Leaf records containing actual data (FSP, FOPT, etc.)
//!
//! Each record has an 8-byte header:
//! - Bytes 0-1: Version (4 bits) + Instance (12 bits)
//! - Bytes 2-3: Record type
//! - Bytes 4-7: Record length (not including header)
//!
//! ## Units
//!
//! Escher uses EMUs (English Metric Units) for coordinates:
//! - 914,400 EMUs = 1 inch
//! - 12,700 EMUs = 1 point
//!
//! ## References
//!
//! - [MS-ODRAW]: Microsoft Office Drawing Binary File Format
//! - libmspub: Open source Publisher format implementation

use std::collections::HashMap;
use std::io::{Read, Seek};

use repub_model::Color;
use repub_model::Rect;

use crate::error::{FormatError, FormatResult};
use crate::pub_import::{emus_to_points, EMUS_PER_POINT};

// Constants: Escher Record Types

/// DggContainer - Drawing Group Container
pub const ESCHER_DGG_CONTAINER: u16 = 0xF000;
/// BStoreContainer - Blip Store Container
pub const ESCHER_BSTORE_CONTAINER: u16 = 0xF001;
/// DgContainer - Drawing Container
pub const ESCHER_DG_CONTAINER: u16 = 0xF002;
/// SpgrContainer - Shape Group Container
pub const ESCHER_SPGR_CONTAINER: u16 = 0xF003;
/// SpContainer - Shape Container
pub const ESCHER_SP_CONTAINER: u16 = 0xF004;
/// SolverContainer
pub const ESCHER_SOLVER_CONTAINER: u16 = 0xF005;
/// FDGGBlock - File Drawing Group Block
pub const ESCHER_FDGGBLOCK: u16 = 0xF006;
/// BSE - Blip Store Entry
pub const ESCHER_BSE: u16 = 0xF007;
/// FDG - File Drawing
pub const ESCHER_FDG: u16 = 0xF008;
/// FSPGR - File Shape Group
pub const ESCHER_FSPGR: u16 = 0xF009;
/// FSP - File Shape
pub const ESCHER_FSP: u16 = 0xF00A;
/// FOPT - File Options (shape properties)
pub const ESCHER_FOPT: u16 = 0xF00B;
/// FOPT - Secondary options
pub const ESCHER_SECONDARY_FOPT: u16 = 0xF121;
/// FOPT - Tertiary options
pub const ESCHER_TERTIARY_FOPT: u16 = 0xF122;
/// TextBox
pub const ESCHER_TEXTBOX: u16 = 0xF00C;
/// ClientTextBox
pub const ESCHER_CLIENT_TEXTBOX: u16 = 0xF00D;
/// Anchor
pub const ESCHER_ANCHOR: u16 = 0xF00E;
/// ChildAnchor
pub const ESCHER_CHILD_ANCHOR: u16 = 0xF00F;
/// ClientAnchor
pub const ESCHER_CLIENT_ANCHOR: u16 = 0xF010;
/// ClientData
pub const ESCHER_CLIENT_DATA: u16 = 0xF011;
/// ConnectorRule
pub const ESCHER_CONNECTOR_RULE: u16 = 0xF012;
/// AlignRule
pub const ESCHER_ALIGN_RULE: u16 = 0xF013;
/// ArcRule
pub const ESCHER_ARC_RULE: u16 = 0xF014;
/// ClientRule
pub const ESCHER_CLIENT_RULE: u16 = 0xF015;
/// CalloutRule
pub const ESCHER_CALLOUT_RULE: u16 = 0xF017;
/// Selection
pub const ESCHER_SELECTION: u16 = 0xF119;
/// ColorMRU - Most Recently Used colors
pub const ESCHER_COLOR_MRU: u16 = 0xF11A;
/// SplitMenuColors
pub const ESCHER_SPLIT_MENU_COLORS: u16 = 0xF11E;
/// RegroupItems
pub const ESCHER_REGROUP_ITEMS: u16 = 0xF118;

// Blip record types (images)
/// BlipEMF - Enhanced Metafile
pub const ESCHER_BLIP_EMF: u16 = 0xF01A;
/// BlipWMF - Windows Metafile
pub const ESCHER_BLIP_WMF: u16 = 0xF01B;
/// BlipPICT - Macintosh PICT
pub const ESCHER_BLIP_PICT: u16 = 0xF01C;
/// BlipJPEG - JPEG image
pub const ESCHER_BLIP_JPEG: u16 = 0xF01D;
/// BlipPNG - PNG image
pub const ESCHER_BLIP_PNG: u16 = 0xF01E;
/// BlipDIB - Device Independent Bitmap
pub const ESCHER_BLIP_DIB: u16 = 0xF01F;
/// BlipTIFF - TIFF image
pub const ESCHER_BLIP_TIFF: u16 = 0xF029;
/// BlipJPEG (alternative)
pub const ESCHER_BLIP_JPEG_ALT: u16 = 0xF02A;

// Constants: FOPT Property IDs

// Transform properties (0x0000-0x003F)
/// Rotation angle in degrees * 65536 (fixed point 16.16)
pub const FOPT_ROTATION: u16 = 0x0004;

// Protection properties (0x0040-0x007F)
/// Lock rotation
pub const FOPT_LOCK_ROTATION: u16 = 0x0077;
/// Lock aspect ratio
pub const FOPT_LOCK_ASPECT_RATIO: u16 = 0x0078;
/// Lock position
pub const FOPT_LOCK_POSITION: u16 = 0x0079;

// Position properties (0x0080-0x00BF) - in EMUs
/// Left position (x coordinate)
pub const FOPT_POS_LEFT: u16 = 0x0080;
/// Top position (y coordinate)
pub const FOPT_POS_TOP: u16 = 0x0081;
/// Right position (x + width)
pub const FOPT_POS_RIGHT: u16 = 0x0082;
/// Bottom position (y + height)
pub const FOPT_POS_BOTTOM: u16 = 0x0083;

// Picture/Blip crop properties (0x0100-0x0103)
/// Picture crop from left edge (in EMUs)
pub const FOPT_PICTURE_CROP_LEFT: u16 = 0x0100;
/// Picture crop from top edge (in EMUs)
pub const FOPT_PICTURE_CROP_TOP: u16 = 0x0101;
/// Picture crop from right edge (in EMUs)
pub const FOPT_PICTURE_CROP_RIGHT: u16 = 0x0102;
/// Picture crop from bottom edge (in EMUs)
pub const FOPT_PICTURE_CROP_BOTTOM: u16 = 0x0103;
/// Shape path
pub const FOPT_SHAPE_PATH: u16 = 0x0104;
/// Adjust value 1 (for adjustable shapes)
pub const FOPT_ADJUST1: u16 = 0x0147;
/// Adjust value 2
pub const FOPT_ADJUST2: u16 = 0x0148;
/// Adjust value 3
pub const FOPT_ADJUST3: u16 = 0x0149;

// Geometry properties (0x0140-0x0143)
/// Geometry left coordinate
pub const FOPT_GEO_LEFT: u16 = 0x0140;
/// Geometry top coordinate
pub const FOPT_GEO_TOP: u16 = 0x0141;
/// Geometry right coordinate (coordinate system width)
pub const FOPT_GEO_RIGHT: u16 = 0x0142;
/// Geometry bottom coordinate (coordinate system height)
pub const FOPT_GEO_BOTTOM: u16 = 0x0143;
/// Complex property: freeform path vertices
pub const FOPT_P_VERTICES: u16 = 0xC145;
/// Complex property: freeform path segments
pub const FOPT_P_SEGMENTS: u16 = 0xC146;

// Fill properties (0x0180-0x01BF)
/// Fill type (solid, gradient, pattern, texture, etc.)
pub const FOPT_FILL_TYPE: u16 = 0x0180;
/// Fill color (BGR format)
pub const FOPT_FILL_COLOR: u16 = 0x0181;
/// Fill opacity (0-65536, where 65536 = fully opaque)
pub const FOPT_FILL_OPACITY: u16 = 0x0182;
/// Fill background color (for patterns)
pub const FOPT_FILL_BACK_COLOR: u16 = 0x0183;
/// Fill background opacity
pub const FOPT_FILL_BACK_OPACITY: u16 = 0x0184;
/// Fill CrMod (color modification)
pub const FOPT_FILL_CRMOD: u16 = 0x0185;
/// Fill blip index (for texture fills)
pub const FOPT_FILL_BLIP: u16 = 0x0186;
/// Fill blip name
pub const FOPT_FILL_BLIP_NAME: u16 = 0x0187;
/// Fill angle (for gradient fills, in fixed-point degrees)
pub const FOPT_FILL_ANGLE: u16 = 0x018B;
/// Fill focus (for gradient fills, 0-100)
pub const FOPT_FILL_FOCUS: u16 = 0x018C;
/// Fill to left (gradient positioning)
pub const FOPT_FILL_TO_LEFT: u16 = 0x018D;
/// Fill to top (gradient positioning)
pub const FOPT_FILL_TO_TOP: u16 = 0x018E;
/// Fill to right (gradient positioning)
pub const FOPT_FILL_TO_RIGHT: u16 = 0x018F;
/// Fill to bottom (gradient positioning)
pub const FOPT_FILL_TO_BOTTOM: u16 = 0x0190;
/// Fill shade preset (gradient preset type)
pub const FOPT_FILL_SHADE_PRESET: u16 = 0x0195;
/// Fill shade colors (complex property with gradient stops)
pub const FOPT_FILL_SHADE_COLORS: u16 = 0x0196;
/// Fill shade complex (gradient color data)
pub const FOPT_FILL_SHADE_COMPLEX: u16 = 0xC197;
/// Fill flags (fNoFillHitTest, fillUseRect, etc.)
pub const FOPT_FILL_FLAGS: u16 = 0x01BF;

// Line/stroke properties (0x01C0-0x01FF)
/// Line color (BGR format)
pub const FOPT_LINE_COLOR: u16 = 0x01C0;
/// Line opacity
pub const FOPT_LINE_OPACITY: u16 = 0x01C1;
/// Line back color
pub const FOPT_LINE_BACK_COLOR: u16 = 0x01C2;
/// Line CrMod
pub const FOPT_LINE_CRMOD: u16 = 0x01C3;
/// Line type (single, double, etc.)
pub const FOPT_LINE_TYPE: u16 = 0x01C4;
/// Line fill blip
pub const FOPT_LINE_FILL_BLIP: u16 = 0x01C5;
/// Line width in EMUs
pub const FOPT_LINE_WIDTH: u16 = 0x01CB;
/// Line miter limit
pub const FOPT_LINE_MITER_LIMIT: u16 = 0x01CC;
/// Line style (solid, dash, dot, etc.)
pub const FOPT_LINE_STYLE: u16 = 0x01CD;
/// Line dash style
pub const FOPT_LINE_DASH_STYLE: u16 = 0x01CE;
/// Start arrow head style
pub const FOPT_LINE_START_ARROW_HEAD: u16 = 0x01CF;
/// End arrow head style
pub const FOPT_LINE_END_ARROW_HEAD: u16 = 0x01D0;
/// Start arrow width
pub const FOPT_LINE_START_ARROW_WIDTH: u16 = 0x01D1;
/// Start arrow length
pub const FOPT_LINE_START_ARROW_LENGTH: u16 = 0x01D2;
/// End arrow width
pub const FOPT_LINE_END_ARROW_WIDTH: u16 = 0x01D3;
/// End arrow length
pub const FOPT_LINE_END_ARROW_LENGTH: u16 = 0x01D4;
/// Line join style (round, bevel, miter)
pub const FOPT_LINE_JOIN_STYLE: u16 = 0x01D5;
/// Line end cap style (flat, round, square)
pub const FOPT_LINE_END_CAP_STYLE: u16 = 0x01D6;
/// Line flags
pub const FOPT_LINE_FLAGS: u16 = 0x01FF;

// Shadow properties (0x0200-0x023F)
/// Shadow type (offset, double, etc.)
pub const FOPT_SHADOW_TYPE: u16 = 0x0200;
/// Shadow color
pub const FOPT_SHADOW_COLOR: u16 = 0x0201;
/// Shadow highlight color
pub const FOPT_SHADOW_HIGHLIGHT: u16 = 0x0202;
/// Shadow CrMod
pub const FOPT_SHADOW_CRMOD: u16 = 0x0203;
/// Shadow opacity
pub const FOPT_SHADOW_OPACITY: u16 = 0x0204;
/// Shadow offset X
pub const FOPT_SHADOW_OFFSET_X: u16 = 0x0205;
/// Shadow offset Y
pub const FOPT_SHADOW_OFFSET_Y: u16 = 0x0206;
/// Shadow flags
pub const FOPT_SHADOW_FLAGS: u16 = 0x023F;

// Perspective/3D Object properties (0x0500-0x053F)
/// c3DSpecularAmt - Specular amount (FixedPoint 16.16)
pub const FOPT_3D_SPECULAR_AMT: u16 = 0x0500;
/// c3DDiffuseAmt - Diffuse amount (FixedPoint 16.16)
pub const FOPT_3D_DIFFUSE_AMT: u16 = 0x0501;
/// c3DShininess - Shininess (integer)
pub const FOPT_3D_SHININESS: u16 = 0x0502;
/// c3DEdgeThickness - Edge thickness (EMUs)
pub const FOPT_3D_EDGE_THICKNESS: u16 = 0x0503;
/// c3DExtrudeForward - Forward extrusion depth (EMUs)
pub const FOPT_3D_EXTRUDE_FORWARD: u16 = 0x0504;
/// c3DExtrudeBackward - Backward extrusion depth (EMUs, default 457200 = 0.5 inch)
pub const FOPT_3D_EXTRUDE_BACKWARD: u16 = 0x0505;
/// c3DExtrusionColor - Extrusion color (MSOCLR)
pub const FOPT_3D_EXTRUSION_COLOR: u16 = 0x0506;
/// c3DCrMod - Color modification for BW views
pub const FOPT_3D_CRMOD: u16 = 0x0507;
/// 3D Object Boolean properties
pub const FOPT_3D_BOOL: u16 = 0x053F;

// 3D Object Boolean flags (in FOPT_3D_BOOL)
/// f3D - Shape has 3D properties
pub const F3D_HAS_3D: u32 = 0x0001_0000;
/// fc3DMetallic - Use metallic lighting
pub const F3D_METALLIC: u32 = 0x0002_0000;
/// fc3DUseExtrusionColor - Use separate extrusion color
pub const F3D_USE_EXTRUSION_COLOR: u32 = 0x0004_0000;
/// fc3DLightFace - Light the front face
pub const F3D_LIGHT_FACE: u32 = 0x0008_0000;

// 3D Style properties (0x0580-0x05BF)
/// c3DYRotationAngle - Y-axis rotation (FixedPoint degrees, -90 to 90)
pub const FOPT_3D_Y_ROTATION: u16 = 0x0580;
/// c3DXRotationAngle - X-axis rotation (FixedPoint degrees, -90 to 90)
pub const FOPT_3D_X_ROTATION: u16 = 0x0581;
/// c3DRotationAxisX - Rotation axis X component
pub const FOPT_3D_ROT_AXIS_X: u16 = 0x0582;
/// c3DRotationAxisY - Rotation axis Y component
pub const FOPT_3D_ROT_AXIS_Y: u16 = 0x0583;
/// c3DRotationAxisZ - Rotation axis Z component
pub const FOPT_3D_ROT_AXIS_Z: u16 = 0x0584;
/// c3DRotationAngle - Rotation angle around axis
pub const FOPT_3D_ROT_ANGLE: u16 = 0x0585;
/// c3DRotationCenterX - Rotation center X
pub const FOPT_3D_ROT_CENTER_X: u16 = 0x0586;
/// c3DRotationCenterY - Rotation center Y
pub const FOPT_3D_ROT_CENTER_Y: u16 = 0x0587;
/// c3DRotationCenterZ - Rotation center Z
pub const FOPT_3D_ROT_CENTER_Z: u16 = 0x0588;
/// c3DRenderMode - Render mode (wireframe, solid, etc.)
pub const FOPT_3D_RENDER_MODE: u16 = 0x0589;
/// c3DTolerance - Tolerance for rendering
pub const FOPT_3D_TOLERANCE: u16 = 0x058A;
/// c3DXViewpoint - Viewpoint X
pub const FOPT_3D_VIEWPOINT_X: u16 = 0x058B;
/// c3DYViewpoint - Viewpoint Y
pub const FOPT_3D_VIEWPOINT_Y: u16 = 0x058C;
/// c3DZViewpoint - Viewpoint Z (default 8750000 EMUs)
pub const FOPT_3D_VIEWPOINT_Z: u16 = 0x058D;
/// c3DOriginX - Origin X (FixedPoint, -0.5 = left, 0.5 = right)
pub const FOPT_3D_ORIGIN_X: u16 = 0x058E;
/// c3DOriginY - Origin Y (FixedPoint, -0.5 = top, 0.5 = bottom)
pub const FOPT_3D_ORIGIN_Y: u16 = 0x058F;
/// c3DSkewAngle - Skew angle (FixedPoint degrees)
pub const FOPT_3D_SKEW_ANGLE: u16 = 0x0590;
/// c3DSkewAmount - Skew amount (percentage)
pub const FOPT_3D_SKEW_AMOUNT: u16 = 0x0591;
/// c3DAmbientIntensity - Ambient light intensity (FixedPoint)
pub const FOPT_3D_AMBIENT_INTENSITY: u16 = 0x0592;
/// c3DKeyX - Key light X direction
pub const FOPT_3D_KEY_X: u16 = 0x0593;
/// c3DKeyY - Key light Y direction
pub const FOPT_3D_KEY_Y: u16 = 0x0594;
/// c3DKeyZ - Key light Z direction
pub const FOPT_3D_KEY_Z: u16 = 0x0595;
/// c3DKeyIntensity - Key light intensity (FixedPoint)
pub const FOPT_3D_KEY_INTENSITY: u16 = 0x0596;
/// c3DFillX - Fill light X direction
pub const FOPT_3D_FILL_X: u16 = 0x0597;
/// c3DFillY - Fill light Y direction
pub const FOPT_3D_FILL_Y: u16 = 0x0598;
/// c3DFillZ - Fill light Z direction
pub const FOPT_3D_FILL_Z: u16 = 0x0599;
/// c3DFillIntensity - Fill light intensity (FixedPoint)
pub const FOPT_3D_FILL_INTENSITY: u16 = 0x059A;
/// 3D Style Boolean properties
pub const FOPT_3D_STYLE_BOOL: u16 = 0x05BF;

// 3D Style Boolean flags
/// fc3DConstrainRotation - Constrain rotation
pub const F3D_CONSTRAIN_ROTATION: u32 = 0x0001_0000;
/// fc3DRotationCenterAuto - Auto rotation center
pub const F3D_ROT_CENTER_AUTO: u32 = 0x0002_0000;
/// fc3DParallel - Parallel projection
pub const F3D_PARALLEL: u32 = 0x0004_0000;
/// fc3DKeyHarsh - Harsh key light
pub const F3D_KEY_HARSH: u32 = 0x0008_0000;
/// fc3DFillHarsh - Harsh fill light
pub const F3D_FILL_HARSH: u32 = 0x0010_0000;

// Perspective/3D properties (0x0240-0x027F)
/// 3D specification present
pub const FOPT_3D_SPEC_PP: u16 = 0x0240;

// Text properties (0x0080-0x00BF in secondary FOPT)
/// Left text margin
pub const FOPT_TEXT_LEFT: u16 = 0x0081;
/// Top text margin
pub const FOPT_TEXT_TOP: u16 = 0x0082;
/// Right text margin
pub const FOPT_TEXT_RIGHT: u16 = 0x0083;
/// Bottom text margin
pub const FOPT_TEXT_BOTTOM: u16 = 0x0084;
/// Word wrap
pub const FOPT_TEXT_WORD_WRAP: u16 = 0x0085;
/// Number of text columns
pub const FOPT_NUM_COLUMNS: u16 = 0x008C;
/// Spacing between columns (EMUs)
pub const FOPT_COLUMN_SPACING: u16 = 0x008D;

// Group shape properties
/// Group shape flags
pub const FOPT_GROUP_FLAGS: u16 = 0x03BF;

// Constants: Geometry Text (WordArt) Properties (0x00C0-0x00DF)

/// WordArt text string (complex property - Unicode string)
pub const FOPT_GTEXT_UNICODE: u16 = 0xC0C0;
/// WordArt RTF text (complex property)
pub const FOPT_GTEXT_RTF: u16 = 0xC0C1;
/// WordArt text alignment
pub const FOPT_GTEXT_ALIGN: u16 = 0x00C2;
/// WordArt text size (EMUs)
pub const FOPT_GTEXT_SIZE: u16 = 0x00C3;
/// WordArt letter spacing (EMUs)
pub const FOPT_GTEXT_SPACING: u16 = 0x00C4;
/// WordArt font family (complex property - string)
pub const FOPT_GTEXT_FONT: u16 = 0xC0C5;
/// WordArt text reverse rows (complex property)
pub const FOPT_GTEXT_CSS_FONT: u16 = 0xC0C6;
/// WordArt boolean flags
pub const FOPT_GTEXT_FLAGS: u16 = 0x00FF;
/// Flag: Text has vertical orientation
pub const GTEXT_FLAG_VERTICAL: u32 = 0x0020;
/// Flag: Text is bold
pub const GTEXT_FLAG_BOLD: u32 = 0x0040;
/// Flag: Text is italic
pub const GTEXT_FLAG_ITALIC: u32 = 0x0080;
/// Flag: Text has underline
pub const GTEXT_FLAG_UNDERLINE: u32 = 0x0100;
/// Flag: Text has shadow
pub const GTEXT_FLAG_SHADOW: u32 = 0x0200;
/// Flag: Text has small caps
pub const GTEXT_FLAG_SMALLCAPS: u32 = 0x0400;
/// Flag: Text has strikethrough
pub const GTEXT_FLAG_STRIKETHROUGH: u32 = 0x0800;
/// Flag: Text is kerned
pub const GTEXT_FLAG_KERN: u32 = 0x1000;
/// Flag: Text is tight (no extra spacing)
pub const GTEXT_FLAG_TIGHT: u32 = 0x2000;
/// Flag: Text is stretched to fit
pub const GTEXT_FLAG_STRETCH: u32 = 0x4000;
/// Flag: Shrink text to fit
pub const GTEXT_FLAG_SHRINK: u32 = 0x8000;
/// Flag: Normalize text height
pub const GTEXT_FLAG_NORMALIZE: u32 = 0x10000;
/// Flag: Best fit mode
pub const GTEXT_FLAG_BESTFIT: u32 = 0x20000;
/// Flag: Reverse text rows
pub const GTEXT_FLAG_REVERSE_ROWS: u32 = 0x80000;

// Shape boolean properties (bit flags)
/// Shape boolean properties
pub const FOPT_SHAPE_BOOL: u16 = 0x017F;

// Constants: Shape Types

/// Shape type: Not a primitive shape
pub const SHAPE_TYPE_NOT_PRIMITIVE: u32 = 0;
/// Shape type: Rectangle
pub const SHAPE_TYPE_RECTANGLE: u32 = 1;
/// Shape type: Rounded rectangle
pub const SHAPE_TYPE_ROUND_RECT: u32 = 2;
/// Shape type: Ellipse
pub const SHAPE_TYPE_ELLIPSE: u32 = 3;
/// Shape type: Diamond
pub const SHAPE_TYPE_DIAMOND: u32 = 4;
/// Shape type: Isosceles triangle
pub const SHAPE_TYPE_ISOSCELES_TRIANGLE: u32 = 5;
/// Shape type: Right triangle
pub const SHAPE_TYPE_RIGHT_TRIANGLE: u32 = 6;
/// Shape type: Parallelogram
pub const SHAPE_TYPE_PARALLELOGRAM: u32 = 7;
/// Shape type: Trapezoid
pub const SHAPE_TYPE_TRAPEZOID: u32 = 8;
/// Shape type: Hexagon
pub const SHAPE_TYPE_HEXAGON: u32 = 9;
/// Shape type: Octagon
pub const SHAPE_TYPE_OCTAGON: u32 = 10;
/// Shape type: Plus sign
pub const SHAPE_TYPE_PLUS: u32 = 11;
/// Shape type: Star (5-pointed)
pub const SHAPE_TYPE_STAR: u32 = 12;
/// Shape type: Arrow (right)
pub const SHAPE_TYPE_ARROW: u32 = 13;
/// Shape type: Thick arrow
pub const SHAPE_TYPE_THICK_ARROW: u32 = 14;
/// Shape type: Home plate
pub const SHAPE_TYPE_HOME_PLATE: u32 = 15;
/// Shape type: Cube
pub const SHAPE_TYPE_CUBE: u32 = 16;
/// Shape type: Balloon (callout)
pub const SHAPE_TYPE_BALLOON: u32 = 17;
/// Shape type: Seal (burst)
pub const SHAPE_TYPE_SEAL: u32 = 18;
/// Shape type: Arc
pub const SHAPE_TYPE_ARC: u32 = 19;
/// Shape type: Line
pub const SHAPE_TYPE_LINE: u32 = 20;
/// Shape type: Plaque
pub const SHAPE_TYPE_PLAQUE: u32 = 21;
/// Shape type: Can (cylinder)
pub const SHAPE_TYPE_CAN: u32 = 22;
/// Shape type: Donut
pub const SHAPE_TYPE_DONUT: u32 = 23;
/// Shape type: Text (simple)
pub const SHAPE_TYPE_TEXT_SIMPLE: u32 = 24;
/// Shape type: Text box
pub const SHAPE_TYPE_TEXT_BOX: u32 = 202;

// WordArt text effect shape types (spt 24-31, 136-175)
/// Shape type: Text octagon (WordArt)
pub const SHAPE_TYPE_TEXT_OCTAGON: u32 = 25;
/// Shape type: Text hexagon (WordArt)
pub const SHAPE_TYPE_TEXT_HEXAGON: u32 = 26;
/// Shape type: Text curve (WordArt)
pub const SHAPE_TYPE_TEXT_CURVE: u32 = 27;
/// Shape type: Text wave (WordArt)
pub const SHAPE_TYPE_TEXT_WAVE: u32 = 28;
/// Shape type: Text ring (WordArt)
pub const SHAPE_TYPE_TEXT_RING: u32 = 29;
/// Shape type: Text on curve (WordArt)
pub const SHAPE_TYPE_TEXT_ON_CURVE: u32 = 30;
/// Shape type: Text on ring (WordArt)
pub const SHAPE_TYPE_TEXT_ON_RING: u32 = 31;
/// Shape type: Plain text (WordArt)
pub const SHAPE_TYPE_TEXT_PLAIN_TEXT: u32 = 136;
/// Shape type: Text stop (WordArt)
pub const SHAPE_TYPE_TEXT_STOP: u32 = 137;
/// Shape type: Text triangle (WordArt)
pub const SHAPE_TYPE_TEXT_TRIANGLE: u32 = 138;
/// Shape type: Text triangle inverted (WordArt)
pub const SHAPE_TYPE_TEXT_TRIANGLE_INVERTED: u32 = 139;
/// Shape type: Text chevron (WordArt)
pub const SHAPE_TYPE_TEXT_CHEVRON: u32 = 140;
/// Shape type: Text chevron inverted (WordArt)
pub const SHAPE_TYPE_TEXT_CHEVRON_INVERTED: u32 = 141;
/// Shape type: Text ring inside (WordArt)
pub const SHAPE_TYPE_TEXT_RING_INSIDE: u32 = 142;
/// Shape type: Text ring outside (WordArt)
pub const SHAPE_TYPE_TEXT_RING_OUTSIDE: u32 = 143;
/// Shape type: Text arch up curve (WordArt)
pub const SHAPE_TYPE_TEXT_ARCH_UP_CURVE: u32 = 144;
/// Shape type: Text arch down curve (WordArt)
pub const SHAPE_TYPE_TEXT_ARCH_DOWN_CURVE: u32 = 145;
/// Shape type: Text circle curve (WordArt)
pub const SHAPE_TYPE_TEXT_CIRCLE_CURVE: u32 = 146;
/// Shape type: Text button curve (WordArt)
pub const SHAPE_TYPE_TEXT_BUTTON_CURVE: u32 = 147;
/// Shape type: Text arch up pour (WordArt)
pub const SHAPE_TYPE_TEXT_ARCH_UP_POUR: u32 = 148;
/// Shape type: Text arch down pour (WordArt)
pub const SHAPE_TYPE_TEXT_ARCH_DOWN_POUR: u32 = 149;
/// Shape type: Text circle pour (WordArt)
pub const SHAPE_TYPE_TEXT_CIRCLE_POUR: u32 = 150;
/// Shape type: Text button pour (WordArt)
pub const SHAPE_TYPE_TEXT_BUTTON_POUR: u32 = 151;
/// Shape type: Text curve up (WordArt)
pub const SHAPE_TYPE_TEXT_CURVE_UP: u32 = 152;
/// Shape type: Text curve down (WordArt)
pub const SHAPE_TYPE_TEXT_CURVE_DOWN: u32 = 153;
/// Shape type: Text cascade up (WordArt)
pub const SHAPE_TYPE_TEXT_CASCADE_UP: u32 = 154;
/// Shape type: Text cascade down (WordArt)
pub const SHAPE_TYPE_TEXT_CASCADE_DOWN: u32 = 155;
/// Shape type: Text wave 1 (WordArt)
pub const SHAPE_TYPE_TEXT_WAVE1: u32 = 156;
/// Shape type: Text wave 2 (WordArt)
pub const SHAPE_TYPE_TEXT_WAVE2: u32 = 157;
/// Shape type: Text wave 3 (WordArt)
pub const SHAPE_TYPE_TEXT_WAVE3: u32 = 158;
/// Shape type: Text wave 4 (WordArt)
pub const SHAPE_TYPE_TEXT_WAVE4: u32 = 159;
/// Shape type: Text inflate (WordArt)
pub const SHAPE_TYPE_TEXT_INFLATE: u32 = 160;
/// Shape type: Text deflate (WordArt)
pub const SHAPE_TYPE_TEXT_DEFLATE: u32 = 161;
/// Shape type: Text inflate bottom (WordArt)
pub const SHAPE_TYPE_TEXT_INFLATE_BOTTOM: u32 = 162;
/// Shape type: Text deflate bottom (WordArt)
pub const SHAPE_TYPE_TEXT_DEFLATE_BOTTOM: u32 = 163;
/// Shape type: Text inflate top (WordArt)
pub const SHAPE_TYPE_TEXT_INFLATE_TOP: u32 = 164;
/// Shape type: Text deflate top (WordArt)
pub const SHAPE_TYPE_TEXT_DEFLATE_TOP: u32 = 165;
/// Shape type: Text deflate inflate (WordArt)
pub const SHAPE_TYPE_TEXT_DEFLATE_INFLATE: u32 = 166;
/// Shape type: Text deflate inflate deflate (WordArt)
pub const SHAPE_TYPE_TEXT_DEFLATE_INFLATE_DEFLATE: u32 = 167;
/// Shape type: Text fade right (WordArt)
pub const SHAPE_TYPE_TEXT_FADE_RIGHT: u32 = 168;
/// Shape type: Text fade left (WordArt)
pub const SHAPE_TYPE_TEXT_FADE_LEFT: u32 = 169;
/// Shape type: Text fade up (WordArt)
pub const SHAPE_TYPE_TEXT_FADE_UP: u32 = 170;
/// Shape type: Text fade down (WordArt)
pub const SHAPE_TYPE_TEXT_FADE_DOWN: u32 = 171;
/// Shape type: Text slant up (WordArt)
pub const SHAPE_TYPE_TEXT_SLANT_UP: u32 = 172;
/// Shape type: Text slant down (WordArt)
pub const SHAPE_TYPE_TEXT_SLANT_DOWN: u32 = 173;
/// Shape type: Text can up (WordArt)
pub const SHAPE_TYPE_TEXT_CAN_UP: u32 = 174;
/// Shape type: Text can down (WordArt)
pub const SHAPE_TYPE_TEXT_CAN_DOWN: u32 = 175;
/// Shape type: Freeform (custom path)
pub const SHAPE_TYPE_FREEFORM: u32 = 0xFFFF;
/// Shape type: Picture frame
pub const SHAPE_TYPE_PICTURE_FRAME: u32 = 75;
/// Shape type: Host control
pub const SHAPE_TYPE_HOST_CONTROL: u32 = 201;

// Additional shape types from MS-ODRAW
/// Shape type: Pentagon
pub const SHAPE_TYPE_PENTAGON: u32 = 56;
/// Shape type: Hexagon (regular)
pub const SHAPE_TYPE_REGULAR_HEXAGON: u32 = 9;
/// Shape type: Heptagon
pub const SHAPE_TYPE_HEPTAGON: u32 = 57;
/// Shape type: Decagon
pub const SHAPE_TYPE_DECAGON: u32 = 58;
/// Shape type: Dodecagon (12-sided)
pub const SHAPE_TYPE_DODECAGON: u32 = 59;
/// Shape type: 4-pointed star
pub const SHAPE_TYPE_STAR4: u32 = 187;
/// Shape type: 6-pointed star
pub const SHAPE_TYPE_STAR6: u32 = 188;
/// Shape type: 7-pointed star
pub const SHAPE_TYPE_STAR7: u32 = 189;
/// Shape type: 8-pointed star (MS-ODRAW: msosptSeal8)
pub const SHAPE_TYPE_STAR8: u32 = 58; // Note: Same as DECAGON in some versions
/// Shape type: 10-pointed star
pub const SHAPE_TYPE_STAR10: u32 = 190;
/// Shape type: 12-pointed star
pub const SHAPE_TYPE_STAR12: u32 = 191;
/// Shape type: 16-pointed star (MS-ODRAW: msosptSeal16)
pub const SHAPE_TYPE_STAR16: u32 = 59; // Note: Same as DODECAGON in some versions
/// Shape type: 24-pointed star (MS-ODRAW: msosptSeal24)
pub const SHAPE_TYPE_STAR24: u32 = 60;
/// Shape type: 32-pointed star (MS-ODRAW: msosptSeal32)
pub const SHAPE_TYPE_STAR32: u32 = 61;
/// Shape type: Left arrow
pub const SHAPE_TYPE_LEFT_ARROW: u32 = 66;
/// Shape type: Up arrow
pub const SHAPE_TYPE_UP_ARROW: u32 = 68;
/// Shape type: Down arrow
pub const SHAPE_TYPE_DOWN_ARROW: u32 = 67;
/// Shape type: Left-right arrow
pub const SHAPE_TYPE_LEFT_RIGHT_ARROW: u32 = 69;
/// Shape type: Up-down arrow
pub const SHAPE_TYPE_UP_DOWN_ARROW: u32 = 70;
/// Shape type: Bent arrow
pub const SHAPE_TYPE_BENT_ARROW: u32 = 91;
/// Shape type: U-turn arrow
pub const SHAPE_TYPE_UTURN_ARROW: u32 = 101;
/// Shape type: Curved left arrow
pub const SHAPE_TYPE_CURVED_LEFT_ARROW: u32 = 103;
/// Shape type: Curved right arrow
pub const SHAPE_TYPE_CURVED_RIGHT_ARROW: u32 = 102;
/// Shape type: Curved up arrow
pub const SHAPE_TYPE_CURVED_UP_ARROW: u32 = 104;
/// Shape type: Curved down arrow
pub const SHAPE_TYPE_CURVED_DOWN_ARROW: u32 = 105;
/// Shape type: Striped right arrow
pub const SHAPE_TYPE_STRIPED_RIGHT_ARROW: u32 = 93;
/// Shape type: Notched right arrow
pub const SHAPE_TYPE_NOTCHED_RIGHT_ARROW: u32 = 94;
/// Shape type: Circular arrow
pub const SHAPE_TYPE_CIRCULAR_ARROW: u32 = 99;

// Connector shape types (from MS-ODRAW)
/// Shape type: Straight connector (line)
pub const SHAPE_TYPE_STRAIGHT_CONNECTOR_1: u32 = 32;
/// Shape type: Bent connector (2 segments, L-shape)
pub const SHAPE_TYPE_BENT_CONNECTOR_2: u32 = 33;
/// Shape type: Bent connector (3 segments)
pub const SHAPE_TYPE_BENT_CONNECTOR_3: u32 = 34;
/// Shape type: Bent connector (4 segments)
pub const SHAPE_TYPE_BENT_CONNECTOR_4: u32 = 35;
/// Shape type: Bent connector (5 segments)
pub const SHAPE_TYPE_BENT_CONNECTOR_5: u32 = 36;
/// Shape type: Curved connector (2 segments)
pub const SHAPE_TYPE_CURVED_CONNECTOR_2: u32 = 37;
/// Shape type: Curved connector (3 segments)
pub const SHAPE_TYPE_CURVED_CONNECTOR_3: u32 = 38;
/// Shape type: Curved connector (4 segments)
pub const SHAPE_TYPE_CURVED_CONNECTOR_4: u32 = 39;
/// Shape type: Curved connector (5 segments)
pub const SHAPE_TYPE_CURVED_CONNECTOR_5: u32 = 40;

// Constants: Fill Types

/// No fill
pub const FILL_TYPE_SOLID: u32 = 0;
/// Pattern fill
pub const FILL_TYPE_PATTERN: u32 = 1;
/// Texture fill
pub const FILL_TYPE_TEXTURE: u32 = 2;
/// Picture fill
pub const FILL_TYPE_PICTURE: u32 = 3;
/// Shade fill (gradient)
pub const FILL_TYPE_SHADE: u32 = 4;
/// Shade from center
pub const FILL_TYPE_SHADE_CENTER: u32 = 5;
/// Shade from shape
pub const FILL_TYPE_SHADE_SHAPE: u32 = 6;
/// Shade from scale
pub const FILL_TYPE_SHADE_SCALE: u32 = 7;
/// Shade from title
pub const FILL_TYPE_SHADE_TITLE: u32 = 8;
/// Background fill
pub const FILL_TYPE_BACKGROUND: u32 = 9;

// Constants: Line Styles

/// Solid line
pub const LINE_STYLE_SOLID: u32 = 0;
/// Dash pattern
pub const LINE_STYLE_DASH: u32 = 1;
/// Dot pattern
pub const LINE_STYLE_DOT: u32 = 2;
/// Dash-dot pattern
pub const LINE_STYLE_DASH_DOT: u32 = 3;
/// Dash-dot-dot pattern
pub const LINE_STYLE_DASH_DOT_DOT: u32 = 4;

// Constants: FSP Flags

/// Shape is a group shape
pub const FSP_FLAG_GROUP: u32 = 0x0001;
/// Shape is a child of a group
pub const FSP_FLAG_CHILD: u32 = 0x0002;
/// Shape is the patriarch (root)
pub const FSP_FLAG_PATRIARCH: u32 = 0x0004;
/// Shape is deleted
pub const FSP_FLAG_DELETED: u32 = 0x0008;
/// Shape is an OLE object
pub const FSP_FLAG_OLE_SHAPE: u32 = 0x0010;
/// Shape has a master
pub const FSP_FLAG_HAVE_MASTER: u32 = 0x0020;
/// Shape is flipped horizontally
pub const FSP_FLAG_FLIP_H: u32 = 0x0040;
/// Shape is flipped vertically
pub const FSP_FLAG_FLIP_V: u32 = 0x0080;
/// Shape is a connector
pub const FSP_FLAG_CONNECTOR: u32 = 0x0100;
/// Shape has an anchor
pub const FSP_FLAG_HAVE_ANCHOR: u32 = 0x0200;
/// Shape is in background
pub const FSP_FLAG_BACKGROUND: u32 = 0x0400;
/// Shape has shape-type property
pub const FSP_FLAG_HAVE_SPT: u32 = 0x0800;

// Types: Image Type

/// Image type enumeration for Escher blips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    /// Enhanced Metafile
    Emf,
    /// Windows Metafile
    Wmf,
    /// Macintosh PICT
    Pict,
    /// JPEG image
    Jpeg,
    /// PNG image
    Png,
    /// Device Independent Bitmap
    Dib,
    /// TIFF image
    Tiff,
    /// Unknown format
    Unknown,
}

impl ImageType {
    /// Get the MIME type for this image format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageType::Emf => "image/x-emf",
            ImageType::Wmf => "image/x-wmf",
            ImageType::Pict => "image/x-pict",
            ImageType::Jpeg => "image/jpeg",
            ImageType::Png => "image/png",
            ImageType::Dib => "image/bmp",
            ImageType::Tiff => "image/tiff",
            ImageType::Unknown => "application/octet-stream",
        }
    }

    /// Get the typical file extension for this image format.
    pub fn extension(&self) -> &'static str {
        match self {
            ImageType::Emf => "emf",
            ImageType::Wmf => "wmf",
            ImageType::Pict => "pict",
            ImageType::Jpeg => "jpg",
            ImageType::Png => "png",
            ImageType::Dib => "bmp",
            ImageType::Tiff => "tiff",
            ImageType::Unknown => "bin",
        }
    }

    /// Create from Escher blip record type.
    pub fn from_record_type(record_type: u16) -> Self {
        match record_type {
            ESCHER_BLIP_EMF => ImageType::Emf,
            ESCHER_BLIP_WMF => ImageType::Wmf,
            ESCHER_BLIP_PICT => ImageType::Pict,
            ESCHER_BLIP_JPEG | ESCHER_BLIP_JPEG_ALT => ImageType::Jpeg,
            ESCHER_BLIP_PNG => ImageType::Png,
            ESCHER_BLIP_DIB => ImageType::Dib,
            ESCHER_BLIP_TIFF => ImageType::Tiff,
            _ => ImageType::Unknown,
        }
    }
}

// Types: Escher Record Header

/// Header for an Escher record.
#[derive(Debug, Clone, Copy)]
pub struct EscherRecordHeader {
    /// Version (4 bits, usually 0x0 for atoms, 0xF for containers)
    pub version: u8,
    /// Instance value (12 bits, meaning varies by record type)
    pub instance: u16,
    /// Record type identifier
    pub record_type: u16,
    /// Length of record data (excluding 8-byte header)
    pub length: u32,
}

impl EscherRecordHeader {
    /// Size of the header in bytes.
    pub const SIZE: usize = 8;

    /// Parse a record header from bytes.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        let ver_inst = read_le_u16(data);
        let record_type = read_le_u16(&data[2..]);
        let length = read_le_u32(&data[4..]);

        Some(Self {
            version: (ver_inst & 0x000F) as u8,
            instance: (ver_inst >> 4) & 0x0FFF,
            record_type,
            length,
        })
    }

    /// Check if this is a container record (version == 0xF).
    pub fn is_container(&self) -> bool {
        self.version == 0x0F
    }

    /// Get the total record size including header.
    pub fn total_size(&self) -> usize {
        Self::SIZE + self.length as usize
    }

    /// Get a human-readable name for the record type.
    pub fn type_name(&self) -> &'static str {
        match self.record_type {
            ESCHER_DGG_CONTAINER => "DggContainer",
            ESCHER_BSTORE_CONTAINER => "BStoreContainer",
            ESCHER_DG_CONTAINER => "DgContainer",
            ESCHER_SPGR_CONTAINER => "SpgrContainer",
            ESCHER_SP_CONTAINER => "SpContainer",
            ESCHER_SOLVER_CONTAINER => "SolverContainer",
            ESCHER_FDGGBLOCK => "FDGGBlock",
            ESCHER_BSE => "BSE",
            ESCHER_FDG => "FDG",
            ESCHER_FSPGR => "FSPGR",
            ESCHER_FSP => "FSP",
            ESCHER_FOPT => "FOPT",
            ESCHER_SECONDARY_FOPT => "SecondaryFOPT",
            ESCHER_TERTIARY_FOPT => "TertiaryFOPT",
            ESCHER_TEXTBOX => "TextBox",
            ESCHER_CLIENT_TEXTBOX => "ClientTextBox",
            ESCHER_ANCHOR => "Anchor",
            ESCHER_CHILD_ANCHOR => "ChildAnchor",
            ESCHER_CLIENT_ANCHOR => "ClientAnchor",
            ESCHER_CLIENT_DATA => "ClientData",
            ESCHER_BLIP_EMF => "BlipEMF",
            ESCHER_BLIP_WMF => "BlipWMF",
            ESCHER_BLIP_PICT => "BlipPICT",
            ESCHER_BLIP_JPEG | ESCHER_BLIP_JPEG_ALT => "BlipJPEG",
            ESCHER_BLIP_PNG => "BlipPNG",
            ESCHER_BLIP_DIB => "BlipDIB",
            ESCHER_BLIP_TIFF => "BlipTIFF",
            _ => "Unknown",
        }
    }
}

// Types: FOPT Property

/// A single property from an FOPT record.
#[derive(Debug, Clone)]
pub struct FoptProperty {
    /// Property ID (14 bits)
    pub id: u16,
    /// Whether this property is a BLIP reference
    pub is_blip: bool,
    /// Whether this property has complex data following the value
    pub is_complex: bool,
    /// Property value (32 bits)
    pub value: u32,
    /// Complex data (if is_complex is true)
    pub complex_data: Option<Vec<u8>>,
}

impl FoptProperty {
    /// Parse properties from an FOPT record.
    ///
    /// The `num_props` parameter specifies how many properties to read. If None,
    /// properties are read until the end of the data (which may read too many
    /// and miss complex data at the end).
    pub fn parse_all(data: &[u8]) -> Vec<Self> {
        Self::parse_with_count(data, None)
    }

    /// Parse properties from an FOPT record with a known property count.
    ///
    /// The count comes from the FOPT header's instance field (bits 4-15).
    pub fn parse_with_count(data: &[u8], num_props: Option<usize>) -> Vec<Self> {
        let mut properties = Vec::new();
        let mut offset = 0;

        // First pass: read property headers
        let mut prop_headers = Vec::new();
        let count = num_props.unwrap_or(data.len() / 6);

        for _ in 0..count {
            if offset + 6 > data.len() {
                break;
            }
            let id_flags = read_le_u16(&data[offset..]);
            let value = read_le_u32(&data[offset + 2..]);

            let id = id_flags & 0x3FFF;
            let is_blip = (id_flags & 0x4000) != 0;
            let is_complex = (id_flags & 0x8000) != 0;

            prop_headers.push((id, is_blip, is_complex, value));
            offset += 6;
        }

        // Second pass: read complex data (follows immediately after all property headers)
        for (id, is_blip, is_complex, value) in prop_headers {
            let complex_data = if is_complex && value > 0 && offset + value as usize <= data.len() {
                let complex = data[offset..offset + value as usize].to_vec();
                offset += value as usize;
                Some(complex)
            } else {
                None
            };

            properties.push(FoptProperty {
                id,
                is_blip,
                is_complex,
                value,
                complex_data,
            });
        }

        properties
    }
}

// Types: Escher Shape

/// A parsed Escher shape.
#[derive(Debug, Clone)]
pub struct EscherShape {
    /// Shape ID (from FSP record)
    pub id: u32,
    /// Shape type (from FSP instance or shape type property)
    pub shape_type: u32,
    /// Bounding rectangle in points
    pub bounds: Rect,
    /// Fill color (if any)
    pub fill_color: Option<Color>,
    /// Fill opacity (0.0-1.0)
    pub fill_opacity: f64,
    /// Fill type (solid, gradient, pattern, etc.)
    pub fill_type: FillType,
    /// Gradient fill info (if fill_type is a gradient)
    pub gradient_fill: Option<GradientFill>,
    /// Line/stroke color (if any)
    pub line_color: Option<Color>,
    /// Line opacity (0.0-1.0)
    pub line_opacity: f64,
    /// Line width in points
    pub line_width: f64,
    /// Rotation angle in degrees
    pub rotation: f64,
    /// FSP flags
    pub flags: u32,
    /// Whether this is a group shape
    pub is_group: bool,
    /// Child shapes (if this is a group)
    pub children: Vec<EscherShape>,
    /// Index of associated blip (image) if any
    pub blip_index: Option<usize>,
    /// All parsed properties
    pub properties: HashMap<u16, u32>,
    /// Complex property data
    pub complex_properties: HashMap<u16, Vec<u8>>,
    /// Client data (application-specific)
    pub client_data: Option<Vec<u8>>,
    /// Anchor data
    pub anchor: Option<EscherAnchor>,
    /// Number of text columns
    pub num_columns: u32,
    /// Spacing between columns (in points)
    pub column_spacing: f64,
    /// Shadow type (0=offset, 1=double, 2=rich, etc.)
    pub shadow_type: Option<u32>,
    /// Shadow color
    pub shadow_color: Option<Color>,
    /// Shadow opacity (0.0-1.0)
    pub shadow_opacity: f64,
    /// Shadow offset X in points
    pub shadow_offset_x: f64,
    /// Shadow offset Y in points
    pub shadow_offset_y: f64,
    /// Text wrap mode (0=none, 1=square, 2=by points, 3=none, 4=tight, 5=through)
    pub text_wrap_mode: Option<u32>,
    /// Image crop from left edge (in EMUs, amount to remove)
    pub crop_left: Option<i32>,
    /// Image crop from top edge (in EMUs, amount to remove)
    pub crop_top: Option<i32>,
    /// Image crop from right edge (in EMUs, amount to remove)
    pub crop_right: Option<i32>,
    /// Image crop from bottom edge (in EMUs, amount to remove)
    pub crop_bottom: Option<i32>,
    /// Custom path vertices (for freeform shapes)
    pub custom_vertices: Option<Vec<PathVertex>>,
    /// Custom path segments (for freeform shapes)
    pub custom_segments: Option<Vec<PathSegment>>,
    /// Geometry coordinate system width (default 21600)
    pub geo_right: u32,
    /// Geometry coordinate system height (default 21600)
    pub geo_bottom: u32,
    /// WordArt text content (from GTEXT Unicode property)
    pub wordart_text: Option<String>,
    /// WordArt font name
    pub wordart_font: Option<String>,
    /// WordArt text size in points
    pub wordart_size: Option<f64>,
    /// WordArt letter spacing in points
    pub wordart_spacing: Option<f64>,
    /// WordArt flags (bold, italic, etc.)
    pub wordart_flags: u32,
}

/// A vertex in a freeform path
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathVertex {
    /// X coordinate in shape coordinate system (0-21600 typically)
    pub x: i32,
    /// Y coordinate in shape coordinate system (0-21600 typically)
    pub y: i32,
}

/// A segment in a freeform path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSegment {
    /// Move to vertex (start new subpath)
    MoveTo,
    /// Line to vertex
    LineTo,
    /// Cubic bezier curve (uses 3 vertices: control1, control2, endpoint)
    CurveTo,
    /// Close the current subpath
    Close,
    /// End of path
    End,
}

/// Fill type for shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillType {
    /// Solid color fill
    #[default]
    Solid,
    /// Pattern fill
    Pattern,
    /// Texture fill (image)
    Texture,
    /// Picture fill
    Picture,
    /// Linear gradient
    GradientLinear,
    /// Center gradient (radial)
    GradientCenter,
    /// Shape gradient
    GradientShape,
    /// Scale gradient
    GradientScale,
    /// Title gradient
    GradientTitle,
    /// Background fill
    Background,
    /// No fill
    None,
}

/// Gradient fill information.
#[derive(Debug, Clone)]
pub struct GradientFill {
    /// Gradient stops (position 0.0-1.0, color)
    pub stops: Vec<GradientStop>,
    /// Rotation angle in degrees
    pub angle: f64,
    /// Gradient focus (for radial gradients, 0-100)
    pub focus: u32,
    /// Fill position (left, top, right, bottom) as fractions
    pub position: (f64, f64, f64, f64),
}

/// A gradient color stop.
#[derive(Debug, Clone)]
pub struct GradientStop {
    /// Position (0.0-1.0)
    pub position: f64,
    /// Color at this stop
    pub color: Color,
    /// Opacity at this stop (0.0-1.0)
    pub opacity: f64,
}

impl Default for EscherShape {
    fn default() -> Self {
        Self {
            id: 0,
            shape_type: 0,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            fill_color: None,
            fill_opacity: 1.0,
            fill_type: FillType::Solid,
            gradient_fill: None,
            line_color: Some(Color::BLACK),
            line_opacity: 1.0,
            line_width: 0.75, // Default 0.75pt line
            rotation: 0.0,
            flags: 0,
            is_group: false,
            children: Vec::new(),
            blip_index: None,
            properties: HashMap::new(),
            complex_properties: HashMap::new(),
            client_data: None,
            anchor: None,
            num_columns: 1,
            column_spacing: 0.0,
            shadow_type: None,
            shadow_color: None,
            shadow_opacity: 1.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            text_wrap_mode: None,
            crop_left: None,
            crop_top: None,
            crop_right: None,
            crop_bottom: None,
            custom_vertices: None,
            custom_segments: None,
            geo_right: 21600,
            geo_bottom: 21600,
            wordart_text: None,
            wordart_font: None,
            wordart_size: None,
            wordart_spacing: None,
            wordart_flags: 0,
        }
    }
}

impl EscherShape {
    /// Create a new empty shape.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this shape is a text box.
    pub fn is_text_box(&self) -> bool {
        self.shape_type == SHAPE_TYPE_TEXT_BOX || self.shape_type == SHAPE_TYPE_TEXT_SIMPLE
    }

    /// Check if this shape is a picture frame.
    pub fn is_picture(&self) -> bool {
        self.shape_type == SHAPE_TYPE_PICTURE_FRAME || self.blip_index.is_some()
    }

    /// Check if this shape is a WordArt text effect.
    /// WordArt shapes have shape types in the range 24-31 (except 24 which is TextSimple)
    /// and 136-175.
    pub fn is_wordart(&self) -> bool {
        matches!(self.shape_type,
            SHAPE_TYPE_TEXT_OCTAGON..=SHAPE_TYPE_TEXT_ON_RING |
            SHAPE_TYPE_TEXT_PLAIN_TEXT..=SHAPE_TYPE_TEXT_CAN_DOWN
        ) || self.wordart_text.is_some()
    }

    /// Check if this shape is flipped horizontally.
    pub fn is_flip_h(&self) -> bool {
        (self.flags & FSP_FLAG_FLIP_H) != 0
    }

    /// Check if this shape is flipped vertically.
    pub fn is_flip_v(&self) -> bool {
        (self.flags & FSP_FLAG_FLIP_V) != 0
    }

    /// Check if this shape is a connector.
    pub fn is_connector(&self) -> bool {
        (self.flags & FSP_FLAG_CONNECTOR) != 0
    }

    /// Check if this shape is deleted.
    pub fn is_deleted(&self) -> bool {
        (self.flags & FSP_FLAG_DELETED) != 0
    }

    /// Get a human-readable shape type name.
    pub fn shape_type_name(&self) -> &'static str {
        match self.shape_type {
            SHAPE_TYPE_NOT_PRIMITIVE => "NotPrimitive",
            SHAPE_TYPE_RECTANGLE => "Rectangle",
            SHAPE_TYPE_ROUND_RECT => "RoundedRectangle",
            SHAPE_TYPE_ELLIPSE => "Ellipse",
            SHAPE_TYPE_DIAMOND => "Diamond",
            SHAPE_TYPE_ISOSCELES_TRIANGLE => "IsoscelesTriangle",
            SHAPE_TYPE_RIGHT_TRIANGLE => "RightTriangle",
            SHAPE_TYPE_PARALLELOGRAM => "Parallelogram",
            SHAPE_TYPE_TRAPEZOID => "Trapezoid",
            SHAPE_TYPE_HEXAGON => "Hexagon",
            SHAPE_TYPE_OCTAGON => "Octagon",
            SHAPE_TYPE_PLUS => "Plus",
            SHAPE_TYPE_STAR => "Star",
            SHAPE_TYPE_ARROW => "Arrow",
            SHAPE_TYPE_THICK_ARROW => "ThickArrow",
            SHAPE_TYPE_HOME_PLATE => "HomePlate",
            SHAPE_TYPE_CUBE => "Cube",
            SHAPE_TYPE_BALLOON => "Balloon",
            SHAPE_TYPE_SEAL => "Seal",
            SHAPE_TYPE_ARC => "Arc",
            SHAPE_TYPE_LINE => "Line",
            SHAPE_TYPE_PLAQUE => "Plaque",
            SHAPE_TYPE_CAN => "Can",
            SHAPE_TYPE_DONUT => "Donut",
            SHAPE_TYPE_TEXT_SIMPLE => "TextSimple",
            SHAPE_TYPE_TEXT_BOX => "TextBox",
            SHAPE_TYPE_PICTURE_FRAME => "PictureFrame",
            _ => "Custom",
        }
    }
}

// Types: Escher Anchor

/// Anchor information for a shape.
#[derive(Debug, Clone)]
pub struct EscherAnchor {
    /// Left position in EMUs or cell units (depends on context)
    pub left: i32,
    /// Top position
    pub top: i32,
    /// Right position
    pub right: i32,
    /// Bottom position
    pub bottom: i32,
}

impl EscherAnchor {
    /// Convert to a Rect in points.
    pub fn to_rect_points(&self) -> Rect {
        let x = emus_to_points(self.left as i64);
        let y = emus_to_points(self.top as i64);
        let w = emus_to_points((self.right - self.left) as i64);
        let h = emus_to_points((self.bottom - self.top) as i64);
        Rect::new(x, y, w.max(0.0), h.max(0.0))
    }
}

// Types: Escher Image

/// An extracted image from the Escher blip store.
#[derive(Debug, Clone)]
pub struct EscherImage {
    /// Index in the blip store (1-based, as referenced by shapes)
    pub index: usize,
    /// Image format
    pub image_type: ImageType,
    /// Raw image data
    pub data: Vec<u8>,
    /// MD4 checksum (16 bytes) if present
    pub checksum: Option<[u8; 16]>,
}

impl EscherImage {
    /// Get the MIME type.
    pub fn mime_type(&self) -> &'static str {
        self.image_type.mime_type()
    }

    /// Get typical file extension.
    pub fn extension(&self) -> &'static str {
        self.image_type.extension()
    }
}

// Types: Blip Store Entry

/// Entry in the blip store (BSE record).
#[derive(Debug, Clone)]
pub struct BlipStoreEntry {
    /// Blip type (JPEG, PNG, etc.)
    pub blip_type: u8,
    /// Blip type (from record header instance)
    pub blip_type_win: u8,
    /// Blip type for Mac
    pub blip_type_mac: u8,
    /// MD4 checksum of the blip
    pub checksum: [u8; 16],
    /// Reference count
    pub ref_count: u32,
    /// Offset to the blip in the delay stream (or 0 if embedded)
    pub offset: u32,
    /// Size of the blip data
    pub size: u32,
    /// Whether blip is in delay stream
    pub in_delay_stream: bool,
    /// Embedded blip data (if not in delay stream)
    pub embedded_blip: Option<EscherImage>,
}

// Types: Drawing Group

/// Drawing group information (from FDGGBlock).
#[derive(Debug, Clone)]
pub struct DrawingGroup {
    /// Maximum shape ID used
    pub max_shape_id: u32,
    /// Number of ID clusters
    pub id_cluster_count: u32,
    /// Total shapes saved
    pub shapes_saved: u32,
    /// Total drawings saved
    pub drawings_saved: u32,
}

// Types: Drawing

/// A single drawing (from FDG record).
#[derive(Debug, Clone)]
pub struct Drawing {
    /// Number of shapes in this drawing
    pub shape_count: u32,
    /// Last shape ID used in this drawing
    pub last_shape_id: u32,
}

// Parser

/// Parser for Escher streams.
#[derive(Debug)]
pub struct EscherParser {
    /// Parsed shapes
    shapes: Vec<EscherShape>,
    /// Blip store entries
    blip_store: Vec<BlipStoreEntry>,
    /// Extracted images
    images: Vec<EscherImage>,
    /// Drawing group info
    drawing_group: Option<DrawingGroup>,
    /// Drawings
    drawings: Vec<Drawing>,
    /// Whether to extract image data
    extract_images: bool,
    /// Warnings accumulated during parsing
    warnings: Vec<String>,
}

impl EscherParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self {
            shapes: Vec::new(),
            blip_store: Vec::new(),
            images: Vec::new(),
            drawing_group: None,
            drawings: Vec::new(),
            extract_images: true,
            warnings: Vec::new(),
        }
    }

    /// Set whether to extract image data.
    pub fn set_extract_images(&mut self, extract: bool) {
        self.extract_images = extract;
    }

    /// Parse an Escher stream.
    pub fn parse(&mut self, data: &[u8]) -> FormatResult<()> {
        self.shapes.clear();
        self.blip_store.clear();
        self.images.clear();
        self.drawing_group = None;
        self.drawings.clear();
        self.warnings.clear();

        self.parse_records(data, 0, data.len())?;
        Ok(())
    }

    /// Parse Escher stream from a Read+Seek source.
    pub fn parse_stream<R: Read + Seek>(&mut self, reader: &mut R) -> FormatResult<()> {
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| FormatError::Io(e))?;
        self.parse(&data)
    }

    /// Parse records within a range.
    fn parse_records(&mut self, data: &[u8], start: usize, end: usize) -> FormatResult<()> {
        let mut offset = start;

        while offset + EscherRecordHeader::SIZE <= end {
            let Some(header) = EscherRecordHeader::parse(&data[offset..]) else {
                break;
            };

            let record_start = offset + EscherRecordHeader::SIZE;
            let record_end = record_start + header.length as usize;

            if record_end > end {
                self.warnings.push(format!(
                    "Record at offset {} extends beyond data (type={:#06x}, len={})",
                    offset, header.record_type, header.length
                ));
                break;
            }

            let record_data = &data[record_start..record_end];

            match header.record_type {
                // Containers - recurse into them
                ESCHER_DGG_CONTAINER | ESCHER_DG_CONTAINER | ESCHER_SOLVER_CONTAINER => {
                    self.parse_records(data, record_start, record_end)?;
                }

                // BStore container - contains blip store entries
                ESCHER_BSTORE_CONTAINER => {
                    self.parse_bstore_container(data, record_start, record_end)?;
                }

                // Shape group container
                ESCHER_SPGR_CONTAINER => {
                    let group = self.parse_shape_group(data, record_start, record_end)?;
                    self.shapes.push(group);
                }

                // Shape container
                ESCHER_SP_CONTAINER => {
                    let shape = self.parse_shape_container(data, record_start, record_end)?;
                    self.shapes.push(shape);
                }

                // Drawing group block
                ESCHER_FDGGBLOCK => {
                    self.parse_fdgg_block(record_data)?;
                }

                // Drawing
                ESCHER_FDG => {
                    self.parse_fdg(record_data)?;
                }

                // Other atom records are handled within their containers
                _ => {}
            }

            offset = record_end;
        }

        Ok(())
    }

    /// Parse a BStore container.
    fn parse_bstore_container(
        &mut self,
        data: &[u8],
        start: usize,
        end: usize,
    ) -> FormatResult<()> {
        let mut offset = start;
        let mut index = 1usize; // Blip indices are 1-based

        while offset + EscherRecordHeader::SIZE <= end {
            let Some(header) = EscherRecordHeader::parse(&data[offset..]) else {
                break;
            };

            let record_start = offset + EscherRecordHeader::SIZE;
            let record_end = record_start + header.length as usize;

            if record_end > end {
                break;
            }

            if header.record_type == ESCHER_BSE {
                self.parse_bse(&data[record_start..record_end], index)?;
                index += 1;
            }

            offset = record_end;
        }

        Ok(())
    }

    /// Parse a Blip Store Entry (BSE).
    fn parse_bse(&mut self, data: &[u8], index: usize) -> FormatResult<()> {
        if data.len() < 44 {
            return Ok(()); // BSE header is at least 44 bytes
        }

        let blip_type_win = data[0];
        let blip_type_mac = data[1];

        let mut checksum = [0u8; 16];
        checksum.copy_from_slice(&data[2..18]);

        // Skip tag (2 bytes at offset 18)
        let size = read_le_u32(&data[20..]);
        let ref_count = read_le_u32(&data[24..]);
        let offset_in_delay = read_le_u32(&data[28..]);
        // Usage byte at offset 32
        // Name length byte at offset 33
        // Unused bytes 34-35
        // Unused bytes 36-43

        let in_delay_stream = offset_in_delay != 0 && size > 0;

        let mut entry = BlipStoreEntry {
            blip_type: blip_type_win,
            blip_type_win,
            blip_type_mac,
            checksum,
            ref_count,
            offset: offset_in_delay,
            size,
            in_delay_stream,
            embedded_blip: None,
        };

        // Check for embedded blip after the BSE header (at offset 44)
        if !in_delay_stream && data.len() > 44 && self.extract_images {
            let blip_data = &data[44..];
            if let Some(image) = self.parse_blip(blip_data, index) {
                entry.embedded_blip = Some(image);
            }
        }

        self.blip_store.push(entry);
        Ok(())
    }

    /// Parse a blip (image) record.
    fn parse_blip(&mut self, data: &[u8], index: usize) -> Option<EscherImage> {
        if data.len() < EscherRecordHeader::SIZE {
            return None;
        }

        let header = EscherRecordHeader::parse(data)?;
        let record_data = data.get(EscherRecordHeader::SIZE..)?;

        let image_type = ImageType::from_record_type(header.record_type);

        // Blip records have a header with checksum before the actual data
        // The header size varies by format:
        // - EMF/WMF: 34 bytes (with filter, 50 bytes)
        // - PICT: 34 bytes (with filter, 50 bytes)
        // - JPEG/PNG/DIB/TIFF: 17 bytes (with filter, 33 bytes)

        let header_size = match header.record_type {
            ESCHER_BLIP_EMF | ESCHER_BLIP_WMF | ESCHER_BLIP_PICT => {
                // Check if there's a secondary UID (filtered)
                if header.instance == 0x3D4 || header.instance == 0x216 || header.instance == 0x542
                {
                    50
                } else {
                    34
                }
            }
            ESCHER_BLIP_JPEG | ESCHER_BLIP_JPEG_ALT | ESCHER_BLIP_PNG | ESCHER_BLIP_DIB
            | ESCHER_BLIP_TIFF => {
                // Check if there's a secondary UID (filtered)
                if header.instance == 0x46A || header.instance == 0x6E0 || header.instance == 0x7A8
                {
                    33
                } else {
                    17
                }
            }
            _ => 17,
        };

        if record_data.len() < header_size {
            return None;
        }

        // Extract checksum (first 16 bytes)
        let mut checksum = [0u8; 16];
        if record_data.len() >= 16 {
            checksum.copy_from_slice(&record_data[..16]);
        }

        // Extract image data
        let image_data = record_data.get(header_size..)?.to_vec();

        if image_data.is_empty() {
            return None;
        }

        Some(EscherImage {
            index,
            image_type,
            data: image_data,
            checksum: Some(checksum),
        })
    }

    /// Parse a shape group container.
    fn parse_shape_group(
        &mut self,
        data: &[u8],
        start: usize,
        end: usize,
    ) -> FormatResult<EscherShape> {
        let mut group = EscherShape::new();
        group.is_group = true;

        let mut offset = start;

        while offset + EscherRecordHeader::SIZE <= end {
            let Some(header) = EscherRecordHeader::parse(&data[offset..]) else {
                break;
            };

            let record_start = offset + EscherRecordHeader::SIZE;
            let record_end = record_start + header.length as usize;

            if record_end > end {
                break;
            }

            match header.record_type {
                ESCHER_SP_CONTAINER => {
                    let child = self.parse_shape_container(data, record_start, record_end)?;
                    group.children.push(child);
                }
                ESCHER_SPGR_CONTAINER => {
                    let child = self.parse_shape_group(data, record_start, record_end)?;
                    group.children.push(child);
                }
                ESCHER_FSPGR => {
                    // Group anchor - defines coordinate space
                    if let Some(anchor) = self.parse_anchor(&data[record_start..record_end]) {
                        group.bounds = anchor.to_rect_points();
                        group.anchor = Some(anchor);
                    }
                }
                _ => {}
            }

            offset = record_end;
        }

        // If group has no explicit bounds, compute from children
        if group.bounds.width() == 0.0 && group.bounds.height() == 0.0 && !group.children.is_empty()
        {
            let mut union = group.children[0].bounds;
            for child in &group.children[1..] {
                union = union.union(&child.bounds);
            }
            group.bounds = union;
        }

        Ok(group)
    }

    /// Parse a shape container.
    fn parse_shape_container(
        &mut self,
        data: &[u8],
        start: usize,
        end: usize,
    ) -> FormatResult<EscherShape> {
        let mut shape = EscherShape::new();
        let mut offset = start;

        // Collect all FOPT properties first
        let mut all_props = Vec::new();

        while offset + EscherRecordHeader::SIZE <= end {
            let Some(header) = EscherRecordHeader::parse(&data[offset..]) else {
                break;
            };

            let record_start = offset + EscherRecordHeader::SIZE;
            let record_end = record_start + header.length as usize;

            if record_end > end {
                break;
            }

            let record_data = &data[record_start..record_end];

            match header.record_type {
                ESCHER_FSP => {
                    self.parse_fsp(record_data, header.instance, &mut shape);
                }
                ESCHER_FOPT | ESCHER_SECONDARY_FOPT | ESCHER_TERTIARY_FOPT => {
                    all_props.extend(FoptProperty::parse_all(record_data));
                }
                ESCHER_CLIENT_ANCHOR | ESCHER_CHILD_ANCHOR => {
                    log::trace!(
                        "Parsing anchor record: type=0x{:04X}, len={}, data={:02X?}",
                        header.record_type,
                        record_data.len(),
                        &record_data[..record_data.len().min(32)]
                    );
                    if let Some(anchor) = self.parse_anchor(record_data) {
                        log::debug!(
                            "Anchor parsed: ({}, {}, {}, {}) EMU -> bounds: {:?}",
                            anchor.left,
                            anchor.top,
                            anchor.right,
                            anchor.bottom,
                            anchor.to_rect_points()
                        );
                        shape.anchor = Some(anchor.clone());
                        shape.bounds = anchor.to_rect_points();
                    } else {
                        log::trace!(
                            "Anchor parsing returned None for {} bytes",
                            record_data.len()
                        );
                    }
                }
                ESCHER_CLIENT_DATA => {
                    shape.client_data = Some(record_data.to_vec());
                }
                _ => {}
            }

            offset = record_end;
        }

        // Apply all FOPT properties to the shape
        self.apply_fopt_properties(&mut shape, &all_props);

        Ok(shape)
    }

    /// Parse FSP (File Shape) record.
    fn parse_fsp(&mut self, data: &[u8], instance: u16, shape: &mut EscherShape) {
        if data.len() < 8 {
            return;
        }

        shape.id = read_le_u32(data);
        shape.flags = read_le_u32(&data[4..]);
        shape.shape_type = instance as u32;
        shape.is_group = (shape.flags & FSP_FLAG_GROUP) != 0;
    }

    /// Parse anchor record.
    ///
    /// Publisher 2016+ uses a different format for FClientAnchor - instead of
    /// simple LTRB coordinates, it contains Publisher-style tagged blocks.
    fn parse_anchor(&self, data: &[u8]) -> Option<EscherAnchor> {
        // Standard Escher anchor format: 16 bytes of LTRB in EMUs
        if data.len() == 16 {
            let left = read_le_i32(data);
            let top = read_le_i32(&data[4..]);
            let right = read_le_i32(&data[8..]);
            let bottom = read_le_i32(&data[12..]);

            // Check if values are reasonable (not all zero or garbage)
            if left != 0 || top != 0 || right != 0 || bottom != 0 {
                return Some(EscherAnchor {
                    left,
                    top,
                    right,
                    bottom,
                });
            }
        }

        // Publisher 2016+ format: First 4 bytes are length, then tagged blocks
        // Format: length (u32), then blocks with: id (u8), type (u8), data (varies)
        if data.len() >= 8 {
            let length = read_le_u32(data) as usize;

            // Check if this looks like Publisher tagged format
            // The length should roughly match the data length minus 4
            if length > 0 && length <= data.len() && data.len() >= 6 {
                // Check if bytes 4-5 look like a block header (id, type)
                let block_type = data.get(5).copied().unwrap_or(0);
                if block_type == 0x20 || block_type == 0x28 {
                    // u32 or i64 types
                    return self.parse_publisher_anchor_blocks(&data[4..]);
                }
            }
        }

        // Also try parsing directly as Publisher blocks (for 4-byte anchors with just unit type)
        if data.len() >= 4 && data.len() < 16 {
            // This is likely just a unit indicator (e.g., 0x04 = EMU)
            // No position data available
            return None;
        }

        None
    }

    /// Parse Publisher 2016+ FClientAnchor tagged blocks format.
    ///
    /// Format: id (u8), type (u8), data (1/2/4/8 bytes based on type)
    /// Block IDs: 0x01=left, 0x02=top, 0x03=right, 0x04=bottom
    fn parse_publisher_anchor_blocks(&self, data: &[u8]) -> Option<EscherAnchor> {
        let mut left: Option<i32> = None;
        let mut top: Option<i32> = None;
        let mut right: Option<i32> = None;
        let mut bottom: Option<i32> = None;

        let mut pos = 0;
        while pos + 2 <= data.len() {
            let block_id = data[pos];
            let block_type = data[pos + 1];
            pos += 2;

            let (data_len, value): (usize, i32) = match block_type {
                0x08 => (1, data.get(pos).map(|&b| b as i8 as i32).unwrap_or(0)),
                0x10 => (
                    2,
                    data.get(pos..pos + 2)
                        .map(|b| i16::from_le_bytes([b[0], b[1]]) as i32)
                        .unwrap_or(0),
                ),
                0x20 => (
                    4,
                    data.get(pos..pos + 4)
                        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .unwrap_or(0),
                ),
                0x28 => (
                    8,
                    data.get(pos..pos + 8)
                        .map(|b| {
                            i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                                as i32
                        })
                        .unwrap_or(0),
                ),
                _ => break, // Unknown type, stop parsing
            };

            match block_id {
                0x01 => left = Some(value),
                0x02 => top = Some(value),
                0x03 => right = Some(value),
                0x04 => bottom = Some(value),
                _ => {} // Other blocks ignored
            }

            pos += data_len;
        }

        // Only return an anchor if we have all four coordinates
        if let (Some(l), Some(t), Some(r), Some(b)) = (left, top, right, bottom) {
            log::trace!(
                "Parsed Publisher anchor blocks: ({}, {}, {}, {}) EMU",
                l,
                t,
                r,
                b
            );
            Some(EscherAnchor {
                left: l,
                top: t,
                right: r,
                bottom: b,
            })
        } else {
            None
        }
    }

    /// Apply FOPT properties to a shape.
    fn apply_fopt_properties(&self, shape: &mut EscherShape, props: &[FoptProperty]) {
        // First pass: collect all values
        for prop in props {
            shape.properties.insert(prop.id, prop.value);
            if let Some(ref complex) = prop.complex_data {
                shape.complex_properties.insert(prop.id, complex.clone());
            }
        }

        // Second pass: apply known properties
        for prop in props {
            match prop.id {
                FOPT_ROTATION => {
                    // Rotation is in fixed-point 16.16 format (degrees * 65536)
                    let fixed_point = prop.value as i32;
                    shape.rotation = fixed_point as f64 / 65536.0;
                }

                // Position properties (EMU coordinates)
                FOPT_POS_LEFT => {
                    let x = emus_to_points(prop.value as i64);
                    shape.bounds.origin.x = x;
                }
                FOPT_POS_TOP => {
                    let y = emus_to_points(prop.value as i64);
                    shape.bounds.origin.y = y;
                }
                FOPT_POS_RIGHT => {
                    let right = emus_to_points(prop.value as i64);
                    shape.bounds.size.width = right - shape.bounds.origin.x;
                }
                FOPT_POS_BOTTOM => {
                    let bottom = emus_to_points(prop.value as i64);
                    shape.bounds.size.height = bottom - shape.bounds.origin.y;
                }

                // Geometry properties (may override position)
                FOPT_GEO_LEFT => {
                    let x = emus_to_points(prop.value as i64);
                    if shape.bounds.origin.x == 0.0 {
                        shape.bounds.origin.x = x;
                    }
                }
                FOPT_GEO_TOP => {
                    let y = emus_to_points(prop.value as i64);
                    if shape.bounds.origin.y == 0.0 {
                        shape.bounds.origin.y = y;
                    }
                }
                FOPT_GEO_RIGHT => {
                    let w = emus_to_points(prop.value as i64);
                    if shape.bounds.size.width <= 0.0 {
                        shape.bounds.size.width = w;
                    }
                }
                FOPT_GEO_BOTTOM => {
                    let h = emus_to_points(prop.value as i64);
                    if shape.bounds.size.height <= 0.0 {
                        shape.bounds.size.height = h;
                    }
                }

                // Fill properties
                FOPT_FILL_TYPE => {
                    shape.fill_type = match prop.value {
                        FILL_TYPE_SOLID => FillType::Solid,
                        FILL_TYPE_PATTERN => FillType::Pattern,
                        FILL_TYPE_TEXTURE => FillType::Texture,
                        FILL_TYPE_PICTURE => FillType::Picture,
                        FILL_TYPE_SHADE => FillType::GradientLinear,
                        FILL_TYPE_SHADE_CENTER => FillType::GradientCenter,
                        FILL_TYPE_SHADE_SHAPE => FillType::GradientShape,
                        FILL_TYPE_SHADE_SCALE => FillType::GradientScale,
                        FILL_TYPE_SHADE_TITLE => FillType::GradientTitle,
                        FILL_TYPE_BACKGROUND => FillType::Background,
                        _ => FillType::Solid,
                    };
                }
                FOPT_FILL_COLOR => {
                    shape.fill_color = Some(bgr_to_color(prop.value));
                }
                FOPT_FILL_OPACITY => {
                    // Opacity is in fixed 16.16 format where 65536 = 1.0
                    shape.fill_opacity = (prop.value as f64 / 65536.0).clamp(0.0, 1.0);
                }
                FOPT_FILL_FLAGS => {
                    // Check fNoFillHitTest (bit 4) and fFilled (bit 0x10 inverted via fUsefFilled)
                    // If bit 0x10 is clear and fUsefFilled (bit 0x100000) is set, no fill
                    let f_filled = (prop.value & 0x10) != 0;
                    let f_use_f_filled = (prop.value & 0x100000) != 0;
                    if f_use_f_filled && !f_filled {
                        shape.fill_color = None;
                        shape.fill_type = FillType::None;
                    }
                }
                FOPT_FILL_BLIP => {
                    // Reference to blip for texture/picture fill
                    if prop.value > 0 {
                        shape.blip_index = Some(prop.value as usize);
                    }
                }
                FOPT_FILL_ANGLE => {
                    // Angle in fixed 16.16 format, convert to degrees
                    let angle = (prop.value as i32 as f64) / 65536.0;
                    if shape.gradient_fill.is_none() {
                        shape.gradient_fill = Some(GradientFill {
                            stops: Vec::new(),
                            angle,
                            focus: 0,
                            position: (0.0, 0.0, 1.0, 1.0),
                        });
                    } else if let Some(ref mut gf) = shape.gradient_fill {
                        gf.angle = angle;
                    }
                }
                FOPT_FILL_FOCUS => {
                    if let Some(ref mut gf) = shape.gradient_fill {
                        gf.focus = prop.value;
                    } else {
                        shape.gradient_fill = Some(GradientFill {
                            stops: Vec::new(),
                            angle: 0.0,
                            focus: prop.value,
                            position: (0.0, 0.0, 1.0, 1.0),
                        });
                    }
                }

                // Line properties
                FOPT_LINE_COLOR => {
                    shape.line_color = Some(bgr_to_color(prop.value));
                }
                FOPT_LINE_OPACITY => {
                    shape.line_opacity = (prop.value as f64 / 65536.0).clamp(0.0, 1.0);
                }
                FOPT_LINE_WIDTH => {
                    // Line width in EMUs
                    shape.line_width = emus_to_points(prop.value as i64);
                }
                FOPT_LINE_FLAGS => {
                    // Check fNoLine (bit 3) and fLine (bit 0x08 inverted via fUsefLine)
                    let f_line = (prop.value & 0x08) != 0;
                    let f_use_f_line = (prop.value & 0x80000) != 0;
                    if f_use_f_line && !f_line {
                        shape.line_color = None;
                        shape.line_width = 0.0;
                    }
                }

                // Text column properties
                FOPT_NUM_COLUMNS => {
                    shape.num_columns = prop.value;
                }
                FOPT_COLUMN_SPACING => {
                    // Column spacing in EMUs
                    shape.column_spacing = emus_to_points(prop.value as i64);
                }

                // Shadow properties
                FOPT_SHADOW_TYPE => {
                    shape.shadow_type = Some(prop.value);
                }
                FOPT_SHADOW_COLOR => {
                    shape.shadow_color = Some(bgr_to_color(prop.value));
                }
                FOPT_SHADOW_OPACITY => {
                    // Opacity is in fixed 16.16 format where 65536 = 1.0
                    shape.shadow_opacity = (prop.value as f64 / 65536.0).clamp(0.0, 1.0);
                }
                FOPT_SHADOW_OFFSET_X => {
                    // Offset in EMUs
                    shape.shadow_offset_x = emus_to_points(prop.value as i32 as i64);
                }
                FOPT_SHADOW_OFFSET_Y => {
                    // Offset in EMUs
                    shape.shadow_offset_y = emus_to_points(prop.value as i32 as i64);
                }
                FOPT_SHADOW_FLAGS => {
                    // Check if shadow is disabled
                    // Bit 1 = fShadow (shape has shadow)
                    // Bit 17 = fUsefShadow (use the fShadow flag)
                    let f_shadow = (prop.value & 0x02) != 0;
                    let f_use_f_shadow = (prop.value & 0x20000) != 0;
                    if f_use_f_shadow && !f_shadow {
                        shape.shadow_type = None;
                        shape.shadow_color = None;
                    }
                }

                // Text wrap mode (from TXID group, 0x0085)
                FOPT_TEXT_WORD_WRAP => {
                    // Values: 0=none, 1=square, 2=by points, 3=none, 4=tight, 5=through
                    shape.text_wrap_mode = Some(prop.value);
                }

                // WordArt (geometry text) properties
                FOPT_GTEXT_SIZE => {
                    // Size in EMUs
                    shape.wordart_size = Some(emus_to_points(prop.value as i64));
                }
                FOPT_GTEXT_SPACING => {
                    // Spacing in EMUs
                    shape.wordart_spacing = Some(emus_to_points(prop.value as i64));
                }
                FOPT_GTEXT_FLAGS => {
                    shape.wordart_flags = prop.value;
                }

                _ => {}
            }
        }

        // Third pass: process complex properties for gradients
        if let Some(shade_data) = shape.complex_properties.get(&FOPT_FILL_SHADE_COMPLEX) {
            let stops = Self::parse_gradient_stops(shade_data);
            if !stops.is_empty() {
                if let Some(ref mut gf) = shape.gradient_fill {
                    gf.stops = stops;
                } else {
                    shape.gradient_fill = Some(GradientFill {
                        stops,
                        angle: 0.0,
                        focus: 0,
                        position: (0.0, 0.0, 1.0, 1.0),
                    });
                }
            }
        }

        // If we have a gradient fill type but no gradient info, create default
        if matches!(
            shape.fill_type,
            FillType::GradientLinear
                | FillType::GradientCenter
                | FillType::GradientShape
                | FillType::GradientScale
                | FillType::GradientTitle
        ) {
            if shape.gradient_fill.is_none() {
                // Create default two-stop gradient from fill color to background color
                let start_color = shape.fill_color.unwrap_or(Color::WHITE);
                let end_color = shape
                    .properties
                    .get(&FOPT_FILL_BACK_COLOR)
                    .map(|&c| bgr_to_color(c))
                    .unwrap_or(Color::BLACK);

                shape.gradient_fill = Some(GradientFill {
                    stops: vec![
                        GradientStop {
                            position: 0.0,
                            color: start_color,
                            opacity: 1.0,
                        },
                        GradientStop {
                            position: 1.0,
                            color: end_color,
                            opacity: 1.0,
                        },
                    ],
                    angle: 0.0,
                    focus: 0,
                    position: (0.0, 0.0, 1.0, 1.0),
                });
            }
        }

        // Parse custom path vertices and segments for freeform shapes
        if let Some(vertex_data) = shape.complex_properties.get(&FOPT_P_VERTICES) {
            shape.custom_vertices = Some(Self::parse_path_vertices(vertex_data));
        }
        if let Some(segment_data) = shape.complex_properties.get(&FOPT_P_SEGMENTS) {
            shape.custom_segments = Some(Self::parse_path_segments(segment_data));
        }

        // Parse WordArt text and font from complex properties
        if let Some(text_data) = shape.complex_properties.get(&FOPT_GTEXT_UNICODE) {
            shape.wordart_text = Self::parse_unicode_string(text_data);
        }
        if let Some(font_data) = shape.complex_properties.get(&FOPT_GTEXT_FONT) {
            shape.wordart_font = Self::parse_unicode_string(font_data);
        }
    }

    /// Parse a Unicode (UTF-16LE) string from complex property data
    fn parse_unicode_string(data: &[u8]) -> Option<String> {
        if data.is_empty() {
            return None;
        }

        // The data is UTF-16LE with possible null terminator
        let mut chars: Vec<u16> = Vec::with_capacity(data.len() / 2);
        let mut i = 0;
        while i + 1 < data.len() {
            let c = u16::from_le_bytes([data[i], data[i + 1]]);
            if c == 0 {
                break; // Null terminator
            }
            chars.push(c);
            i += 2;
        }

        String::from_utf16(&chars).ok().filter(|s| !s.is_empty())
    }

    /// Parse gradient shade complex data to extract gradient stops.
    /// Format: 2-byte count, then 8 bytes per entry (4-byte color, 4-byte position)
    pub fn parse_gradient_stops(data: &[u8]) -> Vec<GradientStop> {
        let mut stops = Vec::new();

        if data.len() < 6 {
            return stops;
        }

        let num_entries = read_le_u16(data) as usize;
        let mut offset = 6; // Skip header (count + 4 unknown bytes)

        let max_entries = (data.len() - offset) / 8;
        let num_entries = num_entries.min(max_entries);

        for _ in 0..num_entries {
            if offset + 8 > data.len() {
                break;
            }

            // Color is BGR format in first 4 bytes
            let color = bgr_to_color(read_le_u32(&data[offset..]));
            offset += 4;

            // Position is fixed-point 16.16 (0x0000 = 0%, 0x10000 = 100%)
            let pos_raw = read_le_u32(&data[offset..]);
            let position = (pos_raw as f64 / 65536.0).clamp(0.0, 1.0);
            offset += 4;

            stops.push(GradientStop {
                position,
                color,
                opacity: 1.0,
            });
        }

        // If we don't have any stops, create default black-white gradient
        if stops.is_empty() {
            stops.push(GradientStop {
                position: 0.0,
                color: Color::BLACK,
                opacity: 1.0,
            });
            stops.push(GradientStop {
                position: 1.0,
                color: Color::WHITE,
                opacity: 1.0,
            });
        }

        stops
    }

    /// Parse freeform path vertices from P_VERTICES complex property.
    /// Format: 2-byte count, 2-byte size per vertex, then vertices (4 or 8 bytes each)
    pub fn parse_path_vertices(data: &[u8]) -> Vec<PathVertex> {
        if data.len() < 6 {
            return Vec::new();
        }

        let num_vertices = read_le_u16(data) as usize;
        let _total_size = read_le_u16(&data[2..]) as usize; // Not used but indicates total byte count
        let vertex_size = read_le_u16(&data[4..]) as usize;

        let mut vertices = Vec::with_capacity(num_vertices);
        let mut offset = 6;

        for _ in 0..num_vertices {
            if offset + vertex_size > data.len() {
                break;
            }

            let (x, y) = if vertex_size >= 8 {
                // 32-bit coordinates
                let x = read_le_i32(&data[offset..]);
                let y = read_le_i32(&data[offset + 4..]);
                (x, y)
            } else if vertex_size >= 4 {
                // 16-bit coordinates
                let x = read_le_i16(&data[offset..]) as i32;
                let y = read_le_i16(&data[offset + 2..]) as i32;
                (x, y)
            } else {
                break;
            };

            vertices.push(PathVertex { x, y });
            offset += vertex_size;
        }

        vertices
    }

    /// Parse freeform path segments from P_SEGMENTS complex property.
    /// Format: 2-byte count, 2-byte size per segment, then segment types (2 bytes each)
    pub fn parse_path_segments(data: &[u8]) -> Vec<PathSegment> {
        if data.len() < 6 {
            return Vec::new();
        }

        let num_segments = read_le_u16(data) as usize;
        let _total_size = read_le_u16(&data[2..]) as usize;
        let segment_size = read_le_u16(&data[4..]) as usize;

        let mut segments = Vec::with_capacity(num_segments);
        let mut offset = 6;

        for _ in 0..num_segments {
            if offset + segment_size > data.len() {
                break;
            }

            // Segment type is in the low byte
            let segment_type = data[offset];

            let segment = match segment_type {
                0x00 | 0x40 => PathSegment::LineTo,
                0x20 | 0x60 => PathSegment::CurveTo,
                0x01 => PathSegment::MoveTo,
                0x80 | 0xC0 => PathSegment::Close,
                0xFF => PathSegment::End,
                _ => PathSegment::LineTo, // Default to line
            };

            segments.push(segment);
            offset += segment_size;
        }

        segments
    }

    fn parse_fdgg_block(&mut self, data: &[u8]) -> FormatResult<()> {
        if data.len() < 16 {
            return Ok(());
        }

        let max_shape_id = read_le_u32(data);
        let id_cluster_count = read_le_u32(&data[4..]);
        let shapes_saved = read_le_u32(&data[8..]);
        let drawings_saved = read_le_u32(&data[12..]);

        self.drawing_group = Some(DrawingGroup {
            max_shape_id,
            id_cluster_count,
            shapes_saved,
            drawings_saved,
        });

        Ok(())
    }

    /// Parse FDG (drawing) record.
    fn parse_fdg(&mut self, data: &[u8]) -> FormatResult<()> {
        if data.len() < 8 {
            return Ok(());
        }

        let shape_count = read_le_u32(data);
        let last_shape_id = read_le_u32(&data[4..]);

        self.drawings.push(Drawing {
            shape_count,
            last_shape_id,
        });

        Ok(())
    }

    /// Parse images from an EscherDelayStm.
    pub fn parse_delay_stream(&mut self, data: &[u8]) -> FormatResult<()> {
        if !self.extract_images {
            return Ok(());
        }

        // The delay stream contains blips at offsets specified in BSE records
        for i in 0..self.blip_store.len() {
            let entry = &self.blip_store[i];
            if entry.in_delay_stream && entry.embedded_blip.is_none() {
                let start = entry.offset as usize;
                let end = start + entry.size as usize;

                if end <= data.len() {
                    let index = i + 1; // 1-based index
                    if let Some(image) = self.parse_blip(&data[start..end], index) {
                        self.blip_store[i].embedded_blip = Some(image);
                    }
                }
            }
        }

        // Also scan for any blips directly in the delay stream
        self.parse_delay_stream_records(data)?;

        Ok(())
    }

    /// Parse records directly in delay stream.
    fn parse_delay_stream_records(&mut self, data: &[u8]) -> FormatResult<()> {
        let mut offset = 0;

        while offset + EscherRecordHeader::SIZE <= data.len() {
            let Some(header) = EscherRecordHeader::parse(&data[offset..]) else {
                break;
            };

            let record_start = offset + EscherRecordHeader::SIZE;
            let record_end = record_start + header.length as usize;

            if record_end > data.len() {
                break;
            }

            // Check for blip records
            match header.record_type {
                ESCHER_BLIP_EMF | ESCHER_BLIP_WMF | ESCHER_BLIP_PICT | ESCHER_BLIP_JPEG
                | ESCHER_BLIP_JPEG_ALT | ESCHER_BLIP_PNG | ESCHER_BLIP_DIB | ESCHER_BLIP_TIFF => {
                    let index = self.images.len() + 1;
                    if let Some(image) = self.parse_blip(&data[offset..record_end], index) {
                        self.images.push(image);
                    }
                }
                _ => {}
            }

            offset = record_end;
        }

        Ok(())
    }

    /// Get parsed shapes.
    pub fn shapes(&self) -> &[EscherShape] {
        &self.shapes
    }

    /// Get parsed shapes (mutable).
    pub fn shapes_mut(&mut self) -> &mut Vec<EscherShape> {
        &mut self.shapes
    }

    /// Take ownership of parsed shapes.
    pub fn take_shapes(&mut self) -> Vec<EscherShape> {
        std::mem::take(&mut self.shapes)
    }

    /// Get blip store entries.
    pub fn blip_store(&self) -> &[BlipStoreEntry] {
        &self.blip_store
    }

    /// Get extracted images.
    pub fn images(&self) -> &[EscherImage] {
        &self.images
    }

    /// Take ownership of extracted images.
    pub fn take_images(&mut self) -> Vec<EscherImage> {
        std::mem::take(&mut self.images)
    }

    /// Get all images (from blip store and directly extracted).
    pub fn all_images(&self) -> Vec<&EscherImage> {
        let mut images: Vec<&EscherImage> = Vec::new();

        // Add images from blip store entries
        for entry in &self.blip_store {
            if let Some(ref img) = entry.embedded_blip {
                images.push(img);
            }
        }

        // Add directly extracted images
        for img in &self.images {
            images.push(img);
        }

        images
    }

    /// Get image by index (1-based, as referenced by shapes).
    pub fn get_image(&self, index: usize) -> Option<&EscherImage> {
        // First check blip store
        if index > 0 && index <= self.blip_store.len() {
            if let Some(ref img) = self.blip_store[index - 1].embedded_blip {
                return Some(img);
            }
        }

        // Fall back to directly extracted images
        self.images.iter().find(|img| img.index == index)
    }

    /// Get drawing group info.
    pub fn drawing_group(&self) -> Option<&DrawingGroup> {
        self.drawing_group.as_ref()
    }

    /// Get drawings.
    pub fn drawings(&self) -> &[Drawing] {
        &self.drawings
    }

    /// Get warnings accumulated during parsing.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Flatten the shape hierarchy into a single list.
    pub fn flatten_shapes(&self) -> Vec<&EscherShape> {
        let mut result = Vec::new();
        self.flatten_shapes_recursive(&self.shapes, &mut result);
        result
    }

    fn flatten_shapes_recursive<'a>(
        &self,
        shapes: &'a [EscherShape],
        result: &mut Vec<&'a EscherShape>,
    ) {
        for shape in shapes {
            result.push(shape);
            if !shape.children.is_empty() {
                self.flatten_shapes_recursive(&shape.children, result);
            }
        }
    }
}

impl Default for EscherParser {
    fn default() -> Self {
        Self::new()
    }
}

// Helper Functions

/// Read a little-endian u16.
#[inline]
fn read_le_u16(data: &[u8]) -> u16 {
    u16::from_le_bytes([data[0], data[1]])
}

/// Read a little-endian u32.
#[inline]
fn read_le_u32(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

/// Read a little-endian i16.
#[inline]
fn read_le_i16(data: &[u8]) -> i16 {
    i16::from_le_bytes([data[0], data[1]])
}

/// Read a little-endian i32.
#[inline]
fn read_le_i32(data: &[u8]) -> i32 {
    i32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

/// Convert BGR color value to Color.
fn bgr_to_color(bgr: u32) -> Color {
    let b = ((bgr >> 16) & 0xFF) as u8;
    let g = ((bgr >> 8) & 0xFF) as u8;
    let r = (bgr & 0xFF) as u8;
    Color::from_rgb8(r, g, b)
}

/// Convert EMU to points.
#[inline]
pub fn emu_to_points(emu: i32) -> f64 {
    emu as f64 / EMUS_PER_POINT
}

// Public API Functions

/// Parse an EscherStm stream and return shapes and images.
///
/// This is the main entry point for parsing Escher data from Publisher files.
pub fn parse_escher_stm(data: &[u8]) -> FormatResult<(Vec<EscherShape>, Vec<EscherImage>)> {
    let mut parser = EscherParser::new();
    parser.parse(data)?;
    Ok((parser.take_shapes(), parser.take_images()))
}

/// Parse an EscherStm with an associated EscherDelayStm.
///
/// The delay stream contains large image data that is referenced from the main stream.
pub fn parse_escher_with_delay(
    escher_data: &[u8],
    delay_data: &[u8],
) -> FormatResult<(Vec<EscherShape>, Vec<EscherImage>)> {
    let mut parser = EscherParser::new();
    parser.parse(escher_data)?;
    parser.parse_delay_stream(delay_data)?;

    // Collect all images
    let mut images = parser.take_images();
    for entry in parser.blip_store() {
        if let Some(ref img) = entry.embedded_blip {
            images.push(img.clone());
        }
    }

    Ok((parser.take_shapes(), images))
}

// Shape Type Mapping

/// Mapped shape information for creating Repub objects.
#[derive(Debug, Clone)]
pub enum MappedShapeType {
    /// Rectangle with optional corner radius (in points)
    Rectangle { corner_radius: f64 },
    /// Ellipse (use bounds as-is)
    Ellipse,
    /// Regular polygon with N sides
    Polygon { sides: u32 },
    /// Star with N points and inner radius ratio
    Star { points: u32, inner_ratio: f64 },
    /// Custom path shape
    Path(MappedPath),
    /// Line (use Line object instead of Shape)
    Line {
        /// Start arrow style (from FOPT properties)
        start_arrow: ArrowStyle,
        /// End arrow style
        end_arrow: ArrowStyle,
    },
    /// Connector (line with routing type)
    Connector {
        /// Type of connector routing
        connector_type: ConnectorRouting,
        /// Start arrow style
        start_arrow: ArrowStyle,
        /// End arrow style
        end_arrow: ArrowStyle,
    },
    /// Text box (should use TextFrame)
    TextBox,
    /// Picture frame (should use ImageFrame)
    PictureFrame,
    /// Unsupported or unknown - fall back to rectangle
    Unknown,
}

/// Connector routing type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectorRouting {
    /// Straight line connector
    #[default]
    Straight,
    /// Elbow connector (right-angle bends)
    Elbow,
    /// Curved connector
    Curved,
}

/// Arrow style for line endings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowStyle {
    /// No arrow
    #[default]
    None,
    /// Triangle arrow
    Arrow,
    /// Open arrow (V shape)
    OpenArrow,
    /// Stealth arrow (sharp triangle)
    StealthArrow,
    /// Diamond
    Diamond,
    /// Circle/oval
    Oval,
}

impl ArrowStyle {
    /// Create from Escher arrow head type.
    pub fn from_escher(arrow_type: u32) -> Self {
        match arrow_type {
            0 => ArrowStyle::None,
            1 => ArrowStyle::Arrow,
            2 => ArrowStyle::StealthArrow,
            3 => ArrowStyle::Diamond,
            4 => ArrowStyle::Oval,
            5 => ArrowStyle::OpenArrow,
            _ => ArrowStyle::Arrow,
        }
    }
}

/// A path for custom shapes, normalized to 0-1 coordinate space.
#[derive(Debug, Clone)]
pub struct MappedPath {
    /// Path commands
    pub commands: Vec<MappedPathCommand>,
}

/// A single path command.
#[derive(Debug, Clone, Copy)]
pub enum MappedPathCommand {
    /// Move to point (x, y in 0-1 range relative to bounds)
    MoveTo { x: f64, y: f64 },
    /// Line to point
    LineTo { x: f64, y: f64 },
    /// Quadratic bezier (control point, end point)
    QuadTo { cx: f64, cy: f64, x: f64, y: f64 },
    /// Cubic bezier (two control points, end point)
    CubicTo {
        c1x: f64,
        c1y: f64,
        c2x: f64,
        c2y: f64,
        x: f64,
        y: f64,
    },
    /// Close the path
    Close,
}

impl MappedPath {
    /// Create an empty path.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Create a right triangle path (right angle at bottom-left).
    pub fn right_triangle() -> Self {
        Self {
            commands: vec![
                MappedPathCommand::MoveTo { x: 0.0, y: 1.0 },
                MappedPathCommand::LineTo { x: 1.0, y: 1.0 },
                MappedPathCommand::LineTo { x: 0.0, y: 0.0 },
                MappedPathCommand::Close,
            ],
        }
    }

    /// Create a parallelogram path.
    pub fn parallelogram(skew: f64) -> Self {
        // Default skew is 0.25 (quarter of width offset)
        let s = skew.clamp(0.0, 0.5);
        Self {
            commands: vec![
                MappedPathCommand::MoveTo { x: s, y: 0.0 },
                MappedPathCommand::LineTo { x: 1.0, y: 0.0 },
                MappedPathCommand::LineTo { x: 1.0 - s, y: 1.0 },
                MappedPathCommand::LineTo { x: 0.0, y: 1.0 },
                MappedPathCommand::Close,
            ],
        }
    }

    /// Create a trapezoid path.
    pub fn trapezoid(inset: f64) -> Self {
        // Default inset is 0.25 (quarter of width on each side at top)
        let i = inset.clamp(0.0, 0.5);
        Self {
            commands: vec![
                MappedPathCommand::MoveTo { x: i, y: 0.0 },
                MappedPathCommand::LineTo { x: 1.0 - i, y: 0.0 },
                MappedPathCommand::LineTo { x: 1.0, y: 1.0 },
                MappedPathCommand::LineTo { x: 0.0, y: 1.0 },
                MappedPathCommand::Close,
            ],
        }
    }

    /// Create a plus/cross shape.
    pub fn plus(arm_width: f64) -> Self {
        // Default arm width is 1/3 of the shape
        let w = arm_width.clamp(0.1, 0.9);
        let a = (1.0 - w) / 2.0;
        let b = a + w;
        Self {
            commands: vec![
                MappedPathCommand::MoveTo { x: a, y: 0.0 },
                MappedPathCommand::LineTo { x: b, y: 0.0 },
                MappedPathCommand::LineTo { x: b, y: a },
                MappedPathCommand::LineTo { x: 1.0, y: a },
                MappedPathCommand::LineTo { x: 1.0, y: b },
                MappedPathCommand::LineTo { x: b, y: b },
                MappedPathCommand::LineTo { x: b, y: 1.0 },
                MappedPathCommand::LineTo { x: a, y: 1.0 },
                MappedPathCommand::LineTo { x: a, y: b },
                MappedPathCommand::LineTo { x: 0.0, y: b },
                MappedPathCommand::LineTo { x: 0.0, y: a },
                MappedPathCommand::LineTo { x: a, y: a },
                MappedPathCommand::Close,
            ],
        }
    }

    /// Create a right arrow path.
    pub fn right_arrow(head_width: f64, head_length: f64) -> Self {
        // head_width: ratio of head to total height (default 0.5 means head is full height, shaft is 50%)
        // head_length: ratio of head length to total width (default 0.4)
        let hw = head_width.clamp(0.1, 1.0);
        let hl = head_length.clamp(0.1, 0.9);
        let shaft_top = (1.0 - hw) / 2.0;
        let shaft_bottom = shaft_top + hw;
        let shaft_end = 1.0 - hl;

        Self {
            commands: vec![
                MappedPathCommand::MoveTo {
                    x: 0.0,
                    y: shaft_top,
                },
                MappedPathCommand::LineTo {
                    x: shaft_end,
                    y: shaft_top,
                },
                MappedPathCommand::LineTo {
                    x: shaft_end,
                    y: 0.0,
                },
                MappedPathCommand::LineTo { x: 1.0, y: 0.5 },
                MappedPathCommand::LineTo {
                    x: shaft_end,
                    y: 1.0,
                },
                MappedPathCommand::LineTo {
                    x: shaft_end,
                    y: shaft_bottom,
                },
                MappedPathCommand::LineTo {
                    x: 0.0,
                    y: shaft_bottom,
                },
                MappedPathCommand::Close,
            ],
        }
    }

    /// Create a home plate (pentagon) path.
    pub fn home_plate(point_ratio: f64) -> Self {
        // point_ratio: how far the point extends (default 0.2)
        let p = point_ratio.clamp(0.1, 0.5);
        Self {
            commands: vec![
                MappedPathCommand::MoveTo { x: 0.0, y: 0.0 },
                MappedPathCommand::LineTo { x: 1.0 - p, y: 0.0 },
                MappedPathCommand::LineTo { x: 1.0, y: 0.5 },
                MappedPathCommand::LineTo { x: 1.0 - p, y: 1.0 },
                MappedPathCommand::LineTo { x: 0.0, y: 1.0 },
                MappedPathCommand::Close,
            ],
        }
    }

    /// Create a donut/ring shape (approximated as octagon with center hole).
    /// For proper rendering, this would need a compound path.
    pub fn donut(hole_ratio: f64) -> Self {
        // For now, just create an octagon representing the outer shape
        // A proper donut would need compound path support
        Self::regular_polygon(8)
    }

    /// Create a regular polygon path.
    pub fn regular_polygon(sides: u32) -> Self {
        use std::f64::consts::{FRAC_PI_2, TAU};
        let n = sides.max(3) as f64;
        let mut commands = Vec::with_capacity(sides as usize + 2);

        for i in 0..sides {
            let angle = -FRAC_PI_2 + (i as f64) * TAU / n;
            let x = 0.5 + 0.5 * angle.cos();
            let y = 0.5 + 0.5 * angle.sin();

            if i == 0 {
                commands.push(MappedPathCommand::MoveTo { x, y });
            } else {
                commands.push(MappedPathCommand::LineTo { x, y });
            }
        }
        commands.push(MappedPathCommand::Close);

        Self { commands }
    }
}

impl Default for MappedPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Map an Escher shape type to a Repub shape type.
///
/// This function takes the shape type from an FSP record's instance field
/// and maps it to the appropriate Repub shape representation.
///
/// Some MS-ODRAW shape-type constants share the same numeric value (for
/// example HEXAGON and REGULAR_HEXAGON), so a few `|` arms are unreachable by
/// design.
#[allow(unreachable_patterns)]
pub fn map_escher_shape_type(
    shape_type: u32,
    adjust1: Option<u32>,
    adjust2: Option<u32>,
) -> MappedShapeType {
    // Extract adjustment values (typically in fixed-point format)
    // Adjust values are often in a 0-21600 range representing the shape's coordinate system
    let adj1 = adjust1.map(|v| v as f64 / 21600.0).unwrap_or(0.25);
    let adj2 = adjust2.map(|v| v as f64 / 21600.0).unwrap_or(0.25);

    match shape_type {
        SHAPE_TYPE_NOT_PRIMITIVE => MappedShapeType::Unknown,

        // Basic shapes
        SHAPE_TYPE_RECTANGLE | SHAPE_TYPE_TEXT_BOX | SHAPE_TYPE_TEXT_SIMPLE => {
            MappedShapeType::Rectangle { corner_radius: 0.0 }
        }
        SHAPE_TYPE_ROUND_RECT => {
            // Convert adjustment value to corner radius
            // Default adj1 is about 0.167 (1/6) for typical rounded rect
            let radius_ratio = adjust1.map(|v| v as f64 / 21600.0).unwrap_or(0.167);
            MappedShapeType::Rectangle {
                corner_radius: radius_ratio,
            }
        }
        SHAPE_TYPE_ELLIPSE => MappedShapeType::Ellipse,

        // Regular polygons
        SHAPE_TYPE_ISOSCELES_TRIANGLE => MappedShapeType::Polygon { sides: 3 },
        SHAPE_TYPE_DIAMOND => {
            // Diamond is a 4-sided polygon rotated 45 degrees
            // We represent it as a polygon that will be rendered as a diamond
            MappedShapeType::Polygon { sides: 4 }
        }
        SHAPE_TYPE_PENTAGON => MappedShapeType::Polygon { sides: 5 },
        SHAPE_TYPE_HEXAGON | SHAPE_TYPE_REGULAR_HEXAGON => MappedShapeType::Polygon { sides: 6 },
        SHAPE_TYPE_HEPTAGON => MappedShapeType::Polygon { sides: 7 },
        SHAPE_TYPE_OCTAGON => MappedShapeType::Polygon { sides: 8 },
        // Note: SHAPE_TYPE_DECAGON (58) and SHAPE_TYPE_STAR8 have same value in MS-ODRAW
        // Note: SHAPE_TYPE_DODECAGON (59) and SHAPE_TYPE_STAR16 have same value in MS-ODRAW
        // We use the polygon interpretation for these; use Seal for star bursts
        SHAPE_TYPE_DECAGON => MappedShapeType::Polygon { sides: 10 },
        SHAPE_TYPE_DODECAGON => MappedShapeType::Polygon { sides: 12 },

        // Stars
        SHAPE_TYPE_STAR => MappedShapeType::Star {
            points: 5,
            inner_ratio: 0.382,
        },
        SHAPE_TYPE_STAR4 => MappedShapeType::Star {
            points: 4,
            inner_ratio: 0.382,
        },
        SHAPE_TYPE_STAR6 => MappedShapeType::Star {
            points: 6,
            inner_ratio: 0.382,
        },
        SHAPE_TYPE_STAR7 => MappedShapeType::Star {
            points: 7,
            inner_ratio: 0.382,
        },
        // SHAPE_TYPE_STAR8 (58) - same as DECAGON, handled above
        SHAPE_TYPE_STAR10 => MappedShapeType::Star {
            points: 10,
            inner_ratio: 0.382,
        },
        SHAPE_TYPE_STAR12 => MappedShapeType::Star {
            points: 12,
            inner_ratio: 0.382,
        },
        // SHAPE_TYPE_STAR16 (59) - same as DODECAGON, handled above
        SHAPE_TYPE_STAR24 => MappedShapeType::Star {
            points: 24,
            inner_ratio: 0.382,
        },
        SHAPE_TYPE_STAR32 => MappedShapeType::Star {
            points: 32,
            inner_ratio: 0.382,
        },
        SHAPE_TYPE_SEAL => MappedShapeType::Star {
            points: 16,
            inner_ratio: 0.85,
        }, // Seal/burst

        // Custom path shapes
        SHAPE_TYPE_RIGHT_TRIANGLE => MappedShapeType::Path(MappedPath::right_triangle()),
        SHAPE_TYPE_PARALLELOGRAM => MappedShapeType::Path(MappedPath::parallelogram(adj1)),
        SHAPE_TYPE_TRAPEZOID => MappedShapeType::Path(MappedPath::trapezoid(adj1)),
        SHAPE_TYPE_PLUS => MappedShapeType::Path(MappedPath::plus(adj1)),
        SHAPE_TYPE_HOME_PLATE => MappedShapeType::Path(MappedPath::home_plate(adj1)),
        SHAPE_TYPE_DONUT => MappedShapeType::Path(MappedPath::donut(adj1)),

        // Arrows (block arrows - these are shapes, not lines)
        SHAPE_TYPE_ARROW | SHAPE_TYPE_THICK_ARROW => {
            MappedShapeType::Path(MappedPath::right_arrow(adj1, adj2))
        }

        // Lines
        SHAPE_TYPE_LINE => MappedShapeType::Line {
            start_arrow: ArrowStyle::None,
            end_arrow: ArrowStyle::None,
        },

        // Connectors - straight
        SHAPE_TYPE_STRAIGHT_CONNECTOR_1 => MappedShapeType::Connector {
            connector_type: ConnectorRouting::Straight,
            start_arrow: ArrowStyle::None,
            end_arrow: ArrowStyle::None,
        },

        // Connectors - bent (elbow)
        SHAPE_TYPE_BENT_CONNECTOR_2
        | SHAPE_TYPE_BENT_CONNECTOR_3
        | SHAPE_TYPE_BENT_CONNECTOR_4
        | SHAPE_TYPE_BENT_CONNECTOR_5 => MappedShapeType::Connector {
            connector_type: ConnectorRouting::Elbow,
            start_arrow: ArrowStyle::None,
            end_arrow: ArrowStyle::None,
        },

        // Connectors - curved
        SHAPE_TYPE_CURVED_CONNECTOR_2
        | SHAPE_TYPE_CURVED_CONNECTOR_3
        | SHAPE_TYPE_CURVED_CONNECTOR_4
        | SHAPE_TYPE_CURVED_CONNECTOR_5 => MappedShapeType::Connector {
            connector_type: ConnectorRouting::Curved,
            start_arrow: ArrowStyle::None,
            end_arrow: ArrowStyle::None,
        },

        // Pictures
        SHAPE_TYPE_PICTURE_FRAME => MappedShapeType::PictureFrame,

        // Freeform/custom path
        SHAPE_TYPE_FREEFORM => MappedShapeType::Unknown, // Would need path data from FOPT

        // Arc - approximate as ellipse for now
        SHAPE_TYPE_ARC => MappedShapeType::Ellipse,

        // Can/cylinder, cube, balloon, plaque - fall back to rectangle
        SHAPE_TYPE_CAN | SHAPE_TYPE_CUBE | SHAPE_TYPE_BALLOON | SHAPE_TYPE_PLAQUE => {
            MappedShapeType::Rectangle { corner_radius: 0.0 }
        }

        // Host control - special handling needed
        SHAPE_TYPE_HOST_CONTROL => MappedShapeType::Unknown,

        // Default for unknown types
        _ => MappedShapeType::Unknown,
    }
}

/// Get the default adjustment values for a shape type.
pub fn get_default_adjustments(shape_type: u32) -> (Option<u32>, Option<u32>) {
    match shape_type {
        SHAPE_TYPE_ROUND_RECT => (Some(5400), None), // ~25% corner radius
        SHAPE_TYPE_PARALLELOGRAM => (Some(5400), None), // ~25% skew
        SHAPE_TYPE_TRAPEZOID => (Some(5400), None),  // ~25% inset
        SHAPE_TYPE_PLUS => (Some(7200), None),       // ~33% arm width
        SHAPE_TYPE_ARROW | SHAPE_TYPE_THICK_ARROW => (Some(10800), Some(8640)), // 50% head width, 40% head length
        SHAPE_TYPE_HOME_PLATE => (Some(4320), None),                            // ~20% point
        SHAPE_TYPE_DONUT => (Some(5400), None),                                 // ~25% hole
        _ => (None, None),
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_header_parse() {
        // DggContainer header: version=0xF, instance=0, type=0xF000, length=100
        let data = [0x0F, 0x00, 0x00, 0xF0, 0x64, 0x00, 0x00, 0x00];
        let header = EscherRecordHeader::parse(&data).unwrap();

        assert_eq!(header.version, 0x0F);
        assert_eq!(header.instance, 0);
        assert_eq!(header.record_type, 0xF000);
        assert_eq!(header.length, 100);
        assert!(header.is_container());
    }

    #[test]
    fn test_record_header_atom() {
        // FSP atom header: version=0x2, instance=1 (rectangle), type=0xF00A, length=8
        let data = [0x12, 0x00, 0x0A, 0xF0, 0x08, 0x00, 0x00, 0x00];
        let header = EscherRecordHeader::parse(&data).unwrap();

        assert_eq!(header.version, 0x02);
        assert_eq!(header.instance, 1); // Rectangle
        assert_eq!(header.record_type, 0xF00A);
        assert_eq!(header.length, 8);
        assert!(!header.is_container());
    }

    #[test]
    fn test_bgr_to_color() {
        // Red in BGR format: 0x0000FF
        let red = bgr_to_color(0x0000FF);
        assert!((red.r - 1.0).abs() < 0.01);
        assert!(red.g.abs() < 0.01);
        assert!(red.b.abs() < 0.01);

        // Blue in BGR format: 0xFF0000
        let blue = bgr_to_color(0xFF0000);
        assert!(blue.r.abs() < 0.01);
        assert!(blue.g.abs() < 0.01);
        assert!((blue.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_emu_to_points() {
        // 12,700 EMUs = 1 point
        assert!((emu_to_points(12_700) - 1.0).abs() < 0.001);
        assert!((emu_to_points(127_000) - 10.0).abs() < 0.001);
        assert!((emu_to_points(914_400) - 72.0).abs() < 0.01); // 1 inch
    }

    #[test]
    fn test_image_type() {
        assert_eq!(
            ImageType::from_record_type(ESCHER_BLIP_PNG).mime_type(),
            "image/png"
        );
        assert_eq!(
            ImageType::from_record_type(ESCHER_BLIP_JPEG).extension(),
            "jpg"
        );
        assert_eq!(ImageType::from_record_type(0xFFFF), ImageType::Unknown);
    }

    #[test]
    fn test_escher_shape_default() {
        let shape = EscherShape::new();
        assert_eq!(shape.id, 0);
        assert!(!shape.is_group);
        assert!(shape.children.is_empty());
    }

    #[test]
    fn test_fopt_property_parse() {
        // Single property: ID=0x0181 (fill color), value=0x00FF00 (green in BGR)
        let data = [0x81, 0x01, 0x00, 0xFF, 0x00, 0x00];
        let props = FoptProperty::parse_all(&data);

        assert_eq!(props.len(), 1);
        assert_eq!(props[0].id, 0x0181);
        assert_eq!(props[0].value, 0x0000FF00);
        assert!(!props[0].is_blip);
        assert!(!props[0].is_complex);
    }

    #[test]
    fn test_anchor_to_rect() {
        let anchor = EscherAnchor {
            left: 0,
            top: 0,
            right: 914_400, // 1 inch = 72 points
            bottom: 914_400,
        };

        let rect = anchor.to_rect_points();
        assert!((rect.x() - 0.0).abs() < 0.01);
        assert!((rect.y() - 0.0).abs() < 0.01);
        assert!((rect.width() - 72.0).abs() < 0.01);
        assert!((rect.height() - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_parser_new() {
        let parser = EscherParser::new();
        assert!(parser.shapes().is_empty());
        assert!(parser.images().is_empty());
        assert!(parser.warnings().is_empty());
    }

    #[test]
    fn test_shape_type_name() {
        let mut shape = EscherShape::new();
        shape.shape_type = SHAPE_TYPE_RECTANGLE;
        assert_eq!(shape.shape_type_name(), "Rectangle");

        shape.shape_type = SHAPE_TYPE_ELLIPSE;
        assert_eq!(shape.shape_type_name(), "Ellipse");

        shape.shape_type = SHAPE_TYPE_TEXT_BOX;
        assert_eq!(shape.shape_type_name(), "TextBox");
    }

    #[test]
    fn test_shape_flags() {
        let mut shape = EscherShape::new();
        shape.flags = FSP_FLAG_GROUP | FSP_FLAG_FLIP_H;
        shape.is_group = true; // This is typically set by the parser based on the flag

        assert!(shape.is_group);
        assert!(shape.is_flip_h());
        assert!(!shape.is_flip_v());
        assert!(!shape.is_deleted());

        // Test flag detection
        assert!((shape.flags & FSP_FLAG_GROUP) != 0);
        assert!((shape.flags & FSP_FLAG_FLIP_H) != 0);
        assert!((shape.flags & FSP_FLAG_FLIP_V) == 0);
    }

    // Shape Mapping Tests

    #[test]
    fn test_map_rectangle() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_RECTANGLE, None, None);
        match mapped {
            MappedShapeType::Rectangle { corner_radius } => {
                assert!((corner_radius - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Rectangle, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_round_rect() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_ROUND_RECT, Some(5400), None);
        match mapped {
            MappedShapeType::Rectangle { corner_radius } => {
                assert!(corner_radius > 0.0);
                assert!((corner_radius - 0.25).abs() < 0.01);
            }
            _ => panic!("Expected Rectangle with corner_radius, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_ellipse() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_ELLIPSE, None, None);
        assert!(matches!(mapped, MappedShapeType::Ellipse));
    }

    #[test]
    fn test_map_diamond() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_DIAMOND, None, None);
        match mapped {
            MappedShapeType::Polygon { sides } => {
                assert_eq!(sides, 4);
            }
            _ => panic!("Expected Polygon with 4 sides, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_triangle() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_ISOSCELES_TRIANGLE, None, None);
        match mapped {
            MappedShapeType::Polygon { sides } => {
                assert_eq!(sides, 3);
            }
            _ => panic!("Expected Polygon with 3 sides, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_right_triangle() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_RIGHT_TRIANGLE, None, None);
        match mapped {
            MappedShapeType::Path(path) => {
                assert!(!path.commands.is_empty());
                // Should have MoveTo, LineTo, LineTo, Close
                assert_eq!(path.commands.len(), 4);
            }
            _ => panic!("Expected Path, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_parallelogram() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_PARALLELOGRAM, None, None);
        match mapped {
            MappedShapeType::Path(path) => {
                assert!(!path.commands.is_empty());
            }
            _ => panic!("Expected Path, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_hexagon() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_HEXAGON, None, None);
        match mapped {
            MappedShapeType::Polygon { sides } => {
                assert_eq!(sides, 6);
            }
            _ => panic!("Expected Polygon with 6 sides, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_octagon() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_OCTAGON, None, None);
        match mapped {
            MappedShapeType::Polygon { sides } => {
                assert_eq!(sides, 8);
            }
            _ => panic!("Expected Polygon with 8 sides, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_star() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_STAR, None, None);
        match mapped {
            MappedShapeType::Star {
                points,
                inner_ratio,
            } => {
                assert_eq!(points, 5);
                assert!(inner_ratio > 0.0 && inner_ratio < 1.0);
            }
            _ => panic!("Expected Star, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_line() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_LINE, None, None);
        match mapped {
            MappedShapeType::Line {
                start_arrow,
                end_arrow,
            } => {
                assert_eq!(start_arrow, ArrowStyle::None);
                assert_eq!(end_arrow, ArrowStyle::None);
            }
            _ => panic!("Expected Line, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_picture_frame() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_PICTURE_FRAME, None, None);
        assert!(matches!(mapped, MappedShapeType::PictureFrame));
    }

    #[test]
    fn test_map_text_box() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_TEXT_BOX, None, None);
        match mapped {
            MappedShapeType::Rectangle { .. } => {}
            _ => panic!("Expected Rectangle for text box, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_plus() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_PLUS, None, None);
        match mapped {
            MappedShapeType::Path(path) => {
                // Plus shape has 12 line segments + close
                assert!(path.commands.len() >= 12);
            }
            _ => panic!("Expected Path for plus, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_arrow() {
        let mapped = map_escher_shape_type(SHAPE_TYPE_ARROW, None, None);
        match mapped {
            MappedShapeType::Path(path) => {
                // Arrow path should have multiple commands
                assert!(!path.commands.is_empty());
            }
            _ => panic!("Expected Path for arrow, got {:?}", mapped),
        }
    }

    #[test]
    fn test_map_unknown() {
        let mapped = map_escher_shape_type(0xFFFF, None, None);
        assert!(matches!(mapped, MappedShapeType::Unknown));
    }

    #[test]
    fn test_arrow_style_from_escher() {
        assert_eq!(ArrowStyle::from_escher(0), ArrowStyle::None);
        assert_eq!(ArrowStyle::from_escher(1), ArrowStyle::Arrow);
        assert_eq!(ArrowStyle::from_escher(2), ArrowStyle::StealthArrow);
        assert_eq!(ArrowStyle::from_escher(3), ArrowStyle::Diamond);
        assert_eq!(ArrowStyle::from_escher(4), ArrowStyle::Oval);
        assert_eq!(ArrowStyle::from_escher(5), ArrowStyle::OpenArrow);
    }

    #[test]
    fn test_mapped_path_right_triangle() {
        let path = MappedPath::right_triangle();
        assert_eq!(path.commands.len(), 4);

        // First command should be MoveTo
        match path.commands[0] {
            MappedPathCommand::MoveTo { x, y } => {
                assert!((x - 0.0).abs() < 0.001);
                assert!((y - 1.0).abs() < 0.001);
            }
            _ => panic!("First command should be MoveTo"),
        }
    }

    #[test]
    fn test_mapped_path_parallelogram() {
        let path = MappedPath::parallelogram(0.25);
        assert_eq!(path.commands.len(), 5);
    }

    #[test]
    fn test_mapped_path_trapezoid() {
        let path = MappedPath::trapezoid(0.25);
        assert_eq!(path.commands.len(), 5);
    }

    #[test]
    fn test_mapped_path_plus() {
        let path = MappedPath::plus(1.0 / 3.0);
        // Plus has 12 vertices + close
        assert_eq!(path.commands.len(), 13);
    }

    #[test]
    fn test_mapped_path_right_arrow() {
        let path = MappedPath::right_arrow(0.5, 0.4);
        // Arrow has 7 vertices + close
        assert_eq!(path.commands.len(), 8);
    }

    #[test]
    fn test_mapped_path_regular_polygon() {
        // Test triangle
        let tri = MappedPath::regular_polygon(3);
        assert_eq!(tri.commands.len(), 4); // 3 vertices + close

        // Test hexagon
        let hex = MappedPath::regular_polygon(6);
        assert_eq!(hex.commands.len(), 7); // 6 vertices + close
    }

    #[test]
    fn test_get_default_adjustments() {
        let (adj1, adj2) = get_default_adjustments(SHAPE_TYPE_ROUND_RECT);
        assert!(adj1.is_some());

        let (adj1, adj2) = get_default_adjustments(SHAPE_TYPE_PARALLELOGRAM);
        assert!(adj1.is_some());

        let (adj1, adj2) = get_default_adjustments(SHAPE_TYPE_RECTANGLE);
        assert!(adj1.is_none());
    }

    // Rotation and Flip Tests

    #[test]
    fn test_rotation_property_parsing() {
        // Test FOPT property with rotation: ID=0x0004 (FOPT_ROTATION)
        // 90 degrees = 90 * 65536 = 5898240 = 0x005A0000
        let data = [
            0x04, 0x00, // Property ID (0x0004 = rotation)
            0x00, 0x00, 0x5A, 0x00, // Value: 5898240 in little-endian
        ];
        let props = FoptProperty::parse_all(&data);

        assert_eq!(props.len(), 1);
        assert_eq!(props[0].id, FOPT_ROTATION);

        // Convert fixed-point to degrees
        let rotation_deg = props[0].value as i32 as f64 / 65536.0;
        assert!((rotation_deg - 90.0).abs() < 0.001);
    }

    #[test]
    fn test_rotation_negative() {
        // Test negative rotation: -45 degrees = -45 * 65536 = -2949120
        // As u32 this is 0xFFD30000 (two's complement)
        let data = [
            0x04, 0x00, // Property ID (0x0004 = rotation)
            0x00, 0x00, 0xD3, 0xFF, // -2949120 in little-endian (0xFFD30000)
        ];
        let props = FoptProperty::parse_all(&data);

        assert_eq!(props.len(), 1);
        assert_eq!(props[0].id, FOPT_ROTATION);

        // Convert fixed-point to degrees (treating as signed)
        let rotation_deg = props[0].value as i32 as f64 / 65536.0;
        assert!((rotation_deg - (-45.0)).abs() < 0.001);
    }

    #[test]
    fn test_flip_flags_values() {
        // Verify the flip flag constant values match the MS-ODRAW spec
        assert_eq!(FSP_FLAG_FLIP_H, 0x0040);
        assert_eq!(FSP_FLAG_FLIP_V, 0x0080);

        // Test combined flags
        let flags_both = FSP_FLAG_FLIP_H | FSP_FLAG_FLIP_V;
        assert_eq!(flags_both, 0x00C0);
    }

    #[test]
    fn test_escher_shape_flip_methods() {
        let mut shape = EscherShape::new();

        // No flips initially
        assert!(!shape.is_flip_h());
        assert!(!shape.is_flip_v());

        // Set horizontal flip
        shape.flags = FSP_FLAG_FLIP_H;
        assert!(shape.is_flip_h());
        assert!(!shape.is_flip_v());

        // Set vertical flip
        shape.flags = FSP_FLAG_FLIP_V;
        assert!(!shape.is_flip_h());
        assert!(shape.is_flip_v());

        // Set both flips
        shape.flags = FSP_FLAG_FLIP_H | FSP_FLAG_FLIP_V;
        assert!(shape.is_flip_h());
        assert!(shape.is_flip_v());
    }

    #[test]
    fn test_escher_shape_rotation_default() {
        let shape = EscherShape::new();
        assert!((shape.rotation - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_rotation_fixed_point_format() {
        // Test various rotation angles in fixed-point 16.16 format

        // 0 degrees
        let zero_deg = 0u32 as i32 as f64 / 65536.0;
        assert!((zero_deg - 0.0).abs() < 0.001);

        // 45 degrees = 45 * 65536 = 2949120
        let forty_five = 2949120u32 as i32 as f64 / 65536.0;
        assert!((forty_five - 45.0).abs() < 0.001);

        // 90 degrees = 90 * 65536 = 5898240
        let ninety = 5898240u32 as i32 as f64 / 65536.0;
        assert!((ninety - 90.0).abs() < 0.001);

        // 180 degrees = 180 * 65536 = 11796480
        let one_eighty = 11796480u32 as i32 as f64 / 65536.0;
        assert!((one_eighty - 180.0).abs() < 0.001);

        // 270 degrees = 270 * 65536 = 17694720
        let two_seventy = 17694720u32 as i32 as f64 / 65536.0;
        assert!((two_seventy - 270.0).abs() < 0.001);

        // -90 degrees (counter-clockwise)
        let neg_ninety_fixed = (-90i32 * 65536) as u32;
        let neg_ninety = neg_ninety_fixed as i32 as f64 / 65536.0;
        assert!((neg_ninety - (-90.0)).abs() < 0.001);
    }
}
