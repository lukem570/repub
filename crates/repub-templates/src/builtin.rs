//! Built-in template definitions.

use repub_model::shape::{Shape, ShapeKind};
use repub_model::styles::{Fill, Stroke};
use repub_model::text::StyledText;
use repub_model::{Color, Document, PageOrientation, PageSize, Rect};

use crate::library::TemplateCategory;
use crate::placeholder::{Placeholder, PlaceholderType};
use crate::template::Template;

/// A text-frame shape with initial plain-text content.
fn text_frame(bounds: Rect, text: &str) -> Shape {
    let mut shape = Shape::text_frame(bounds);
    if let ShapeKind::TextFrame(data) = &mut shape.kind {
        data.content = StyledText::plain(text);
    }
    shape
}

// Business Templates

/// Modern business card template (3.5" x 2").
pub fn business_card_modern() -> Template {
    // Business card: 252 x 144 points (3.5" x 2")
    let mut doc = Document::new(
        PageSize::Custom {
            width: 252.0,
            height: 144.0,
        },
        PageOrientation::Landscape,
    );

    let page = &mut doc.pages_mut()[0];

    // Background accent bar
    let mut accent = Shape::rectangle(Rect::new(0.0, 0.0, 252.0, 40.0));
    accent.fill = Fill::Solid(Color::rgb(0.15, 0.4, 0.75)); // Blue
    accent.line = None;
    page.add_shape(accent);

    // Name
    let name_frame = text_frame(Rect::new(16.0, 52.0, 220.0, 28.0), "{{name}}");
    let name_id = name_frame.common.id;
    page.add_shape(name_frame);

    // Title
    let title_frame = text_frame(Rect::new(16.0, 78.0, 220.0, 18.0), "{{title}}");
    let title_id = title_frame.common.id;
    page.add_shape(title_frame);

    // Email
    let email_frame = text_frame(Rect::new(16.0, 100.0, 110.0, 14.0), "{{email}}");
    let email_id = email_frame.common.id;
    page.add_shape(email_frame);

    // Phone
    let phone_frame = text_frame(Rect::new(130.0, 100.0, 110.0, 14.0), "{{phone}}");
    let phone_id = phone_frame.common.id;
    page.add_shape(phone_frame);

    // Website
    let website_frame = text_frame(Rect::new(16.0, 118.0, 220.0, 14.0), "{{website}}");
    let website_id = website_frame.common.id;
    page.add_shape(website_frame);

    // Logo placeholder
    let logo_frame = Shape::picture(Rect::new(190.0, 8.0, 54.0, 24.0));
    let logo_id = logo_frame.common.id;
    page.add_shape(logo_frame);

    let mut template = Template::new(
        "business-card-modern",
        "Modern Business Card",
        TemplateCategory::Business,
        doc,
    );

    template.metadata.description =
        "Clean, modern business card design with blue accent bar".to_string();
    template.metadata.tags = vec![
        "business".to_string(),
        "card".to_string(),
        "modern".to_string(),
        "professional".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("name", "Full Name", name_id)
            .with_default("John Smith")
            .with_example("Jane Doe")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("title", "Job Title", title_id).with_default("Software Engineer"),
    );
    template.add_placeholder(Placeholder {
        id: "email".to_string(),
        name: "Email Address".to_string(),
        description: "Your business email".to_string(),
        object_id: email_id,
        placeholder_type: PlaceholderType::Email,
        default_value: Some("john@company.com".to_string()),
        required: false,
        validation: PlaceholderType::Email
            .default_validation()
            .map(|s| s.to_string()),
        example: Some("jane@company.com".to_string()),
    });
    template.add_placeholder(Placeholder {
        id: "phone".to_string(),
        name: "Phone Number".to_string(),
        description: "Contact phone number".to_string(),
        object_id: phone_id,
        placeholder_type: PlaceholderType::Phone,
        default_value: Some("(555) 123-4567".to_string()),
        required: false,
        validation: None,
        example: None,
    });
    template.add_placeholder(Placeholder {
        id: "website".to_string(),
        name: "Website".to_string(),
        description: "Company website URL".to_string(),
        object_id: website_id,
        placeholder_type: PlaceholderType::Url,
        default_value: Some("www.company.com".to_string()),
        required: false,
        validation: None,
        example: None,
    });
    template.add_placeholder(
        Placeholder::image("logo", "Company Logo", logo_id)
            .with_description("Your company logo (optional)"),
    );

    template
}

/// Classic business card template.
pub fn business_card_classic() -> Template {
    let mut doc = Document::new(
        PageSize::Custom {
            width: 252.0,
            height: 144.0,
        },
        PageOrientation::Landscape,
    );

    let page = &mut doc.pages_mut()[0];

    // Simple border
    let mut border = Shape::rectangle(Rect::new(8.0, 8.0, 236.0, 128.0));
    border.fill = Fill::None;
    border.line = Some(Stroke::new(Color::rgb(0.3, 0.3, 0.3), 1.0));
    page.add_shape(border);

    // Name (centered)
    let name_frame = text_frame(Rect::new(16.0, 40.0, 220.0, 28.0), "{{name}}");
    let name_id = name_frame.common.id;
    page.add_shape(name_frame);

    // Title (centered)
    let title_frame = text_frame(Rect::new(16.0, 68.0, 220.0, 18.0), "{{title}}");
    let title_id = title_frame.common.id;
    page.add_shape(title_frame);

    // Contact info
    let contact_frame = text_frame(Rect::new(16.0, 100.0, 220.0, 28.0), "{{email}} | {{phone}}");
    let contact_id = contact_frame.common.id;
    page.add_shape(contact_frame);

    let mut template = Template::new(
        "business-card-classic",
        "Classic Business Card",
        TemplateCategory::Business,
        doc,
    );

    template.metadata.description =
        "Traditional, elegant business card with simple border".to_string();
    template.metadata.tags = vec![
        "business".to_string(),
        "card".to_string(),
        "classic".to_string(),
        "elegant".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("name", "Full Name", name_id)
            .with_default("John Smith")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("title", "Job Title", title_id).with_default("Managing Director"),
    );
    template.add_placeholder(
        Placeholder::text("contact", "Contact Info", contact_id)
            .with_default("john@company.com | (555) 123-4567"),
    );

    template
}

/// Corporate letterhead template.
pub fn letterhead_corporate() -> Template {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let page = &mut doc.pages_mut()[0];

    // Header bar
    let mut header = Shape::rectangle(Rect::new(0.0, 0.0, 612.0, 72.0));
    header.fill = Fill::Solid(Color::rgb(0.1, 0.3, 0.6));
    header.line = None;
    page.add_shape(header);

    // Company name
    let company_frame = text_frame(Rect::new(72.0, 20.0, 400.0, 32.0), "{{company_name}}");
    let company_id = company_frame.common.id;
    page.add_shape(company_frame);

    // Logo
    let logo_frame = Shape::picture(Rect::new(500.0, 12.0, 96.0, 48.0));
    let logo_id = logo_frame.common.id;
    page.add_shape(logo_frame);

    // Footer line
    let mut footer_line = Shape::rectangle(Rect::new(72.0, 720.0, 468.0, 1.0));
    footer_line.fill = Fill::Solid(Color::rgb(0.6, 0.6, 0.6));
    footer_line.line = None;
    page.add_shape(footer_line);

    // Footer address
    let footer_frame = text_frame(Rect::new(72.0, 730.0, 468.0, 36.0), "{{address}}");
    let address_id = footer_frame.common.id;
    page.add_shape(footer_frame);

    let mut template = Template::new(
        "letterhead-corporate",
        "Corporate Letterhead",
        TemplateCategory::Business,
        doc,
    );

    template.metadata.description =
        "Professional letterhead with header bar and footer".to_string();
    template.metadata.tags = vec![
        "business".to_string(),
        "letterhead".to_string(),
        "corporate".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("company_name", "Company Name", company_id)
            .with_default("Acme Corporation")
            .required(),
    );
    template.add_placeholder(Placeholder::image("logo", "Company Logo", logo_id));
    template.add_placeholder(
        Placeholder::text("address", "Address", address_id).with_default(
            "123 Business Ave, Suite 100, City, State 12345 | (555) 123-4567 | www.company.com",
        ),
    );

    template
}

// Marketing Templates

/// Event flyer template.
pub fn flyer_event() -> Template {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let page = &mut doc.pages_mut()[0];

    // Bold header background
    let mut header_bg = Shape::rectangle(Rect::new(0.0, 0.0, 612.0, 200.0));
    header_bg.fill = Fill::Solid(Color::rgb(0.9, 0.2, 0.2));
    header_bg.line = None;
    page.add_shape(header_bg);

    // Event title
    let title_frame = text_frame(Rect::new(36.0, 60.0, 540.0, 60.0), "{{event_title}}");
    let title_id = title_frame.common.id;
    page.add_shape(title_frame);

    // Date and time
    let date_frame = text_frame(Rect::new(36.0, 130.0, 540.0, 40.0), "{{date}} | {{time}}");
    let date_id = date_frame.common.id;
    page.add_shape(date_frame);

    // Event image
    let image_frame = Shape::picture(Rect::new(36.0, 220.0, 540.0, 280.0));
    let image_id = image_frame.common.id;
    page.add_shape(image_frame);

    // Description
    let desc_frame = text_frame(Rect::new(36.0, 520.0, 540.0, 120.0), "{{description}}");
    let desc_id = desc_frame.common.id;
    page.add_shape(desc_frame);

    // Location
    let location_frame = text_frame(Rect::new(36.0, 660.0, 540.0, 40.0), "📍 {{location}}");
    let location_id = location_frame.common.id;
    page.add_shape(location_frame);

    // Contact
    let contact_frame = text_frame(Rect::new(36.0, 710.0, 540.0, 40.0), "{{contact}}");
    let contact_id = contact_frame.common.id;
    page.add_shape(contact_frame);

    let mut template = Template::new(
        "flyer-event",
        "Event Flyer",
        TemplateCategory::Marketing,
        doc,
    );

    template.metadata.description = "Eye-catching event flyer with bold header".to_string();
    template.metadata.tags = vec![
        "flyer".to_string(),
        "event".to_string(),
        "marketing".to_string(),
        "announcement".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("event_title", "Event Title", title_id)
            .with_default("Annual Conference 2024")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("date", "Date", date_id).with_default("Saturday, March 15, 2024"),
    );
    template.add_placeholder(Placeholder::text("time", "Time", date_id).with_default("7:00 PM")); // Same frame, combined
    template.add_placeholder(
        Placeholder::image("image", "Event Image", image_id)
            .with_description("Main event image or photo"),
    );
    template.add_placeholder(
        Placeholder::text("description", "Description", desc_id).with_default(
            "Join us for an exciting evening of networking, learning, and fun. This year's conference features keynote speakers from industry leaders.",
        ),
    );
    template.add_placeholder(
        Placeholder::text("location", "Location", location_id)
            .with_default("Convention Center, 123 Main St"),
    );
    template.add_placeholder(
        Placeholder::text("contact", "Contact Info", contact_id)
            .with_default("RSVP: events@company.com | (555) 123-4567"),
    );

    template
}

/// Tri-fold brochure template.
pub fn brochure_trifold() -> Template {
    // Tri-fold: 11" x 8.5" landscape, folded into thirds
    let mut doc = Document::new(
        PageSize::Custom {
            width: 792.0,
            height: 612.0,
        },
        PageOrientation::Landscape,
    );

    let page = &mut doc.pages_mut()[0];
    let panel_width = 264.0; // 792 / 3

    // Panel divider lines (guides)
    let mut line1 = Shape::rectangle(Rect::new(panel_width, 0.0, 0.5, 612.0));
    line1.fill = Fill::Solid(Color::rgb(0.8, 0.8, 0.8));
    line1.line = None;
    page.add_shape(line1);

    let mut line2 = Shape::rectangle(Rect::new(panel_width * 2.0, 0.0, 0.5, 612.0));
    line2.fill = Fill::Solid(Color::rgb(0.8, 0.8, 0.8));
    line2.line = None;
    page.add_shape(line2);

    // Panel 1 - Cover (right panel when folded)
    let cover_title = text_frame(
        Rect::new(panel_width * 2.0 + 20.0, 200.0, 224.0, 80.0),
        "{{title}}",
    );
    let title_id = cover_title.common.id;
    page.add_shape(cover_title);

    let cover_subtitle = text_frame(
        Rect::new(panel_width * 2.0 + 20.0, 290.0, 224.0, 40.0),
        "{{subtitle}}",
    );
    let subtitle_id = cover_subtitle.common.id;
    page.add_shape(cover_subtitle);

    // Panel 2 - Inside left
    let section1 = text_frame(Rect::new(20.0, 40.0, 224.0, 530.0), "{{section1}}");
    let section1_id = section1.common.id;
    page.add_shape(section1);

    // Panel 3 - Inside center
    let section2 = text_frame(
        Rect::new(panel_width + 20.0, 40.0, 224.0, 530.0),
        "{{section2}}",
    );
    let section2_id = section2.common.id;
    page.add_shape(section2);

    let mut template = Template::new(
        "brochure-trifold",
        "Tri-Fold Brochure",
        TemplateCategory::Marketing,
        doc,
    );

    template.metadata.description = "Standard tri-fold brochure layout (11\" x 8.5\")".to_string();
    template.metadata.tags = vec![
        "brochure".to_string(),
        "trifold".to_string(),
        "marketing".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("title", "Cover Title", title_id)
            .with_default("Company Name")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("subtitle", "Cover Subtitle", subtitle_id)
            .with_default("Your tagline here"),
    );
    template.add_placeholder(
        Placeholder::text("section1", "Section 1 Content", section1_id)
            .with_default("About Us\n\nLorem ipsum dolor sit amet, consectetur adipiscing elit."),
    );
    template.add_placeholder(
        Placeholder::text("section2", "Section 2 Content", section2_id).with_default(
            "Our Services\n\nLorem ipsum dolor sit amet, consectetur adipiscing elit.",
        ),
    );

    template
}

// Personal Templates

/// Party invitation template.
pub fn invitation_party() -> Template {
    // 5" x 7" invitation
    let mut doc = Document::new(
        PageSize::Custom {
            width: 360.0,
            height: 504.0,
        },
        PageOrientation::Portrait,
    );

    let page = &mut doc.pages_mut()[0];

    // Decorative border
    let mut border = Shape::rounded_rectangle(Rect::new(16.0, 16.0, 328.0, 472.0), 12.0);
    border.fill = Fill::None;
    border.line = Some(Stroke::new(Color::rgb(0.8, 0.6, 0.2), 3.0));
    page.add_shape(border);

    // "You're Invited"
    let header = text_frame(Rect::new(36.0, 50.0, 288.0, 40.0), "You're Invited!");
    page.add_shape(header);

    // Event name
    let event_frame = text_frame(Rect::new(36.0, 110.0, 288.0, 50.0), "{{event_name}}");
    let event_id = event_frame.common.id;
    page.add_shape(event_frame);

    // Date
    let date_frame = text_frame(Rect::new(36.0, 180.0, 288.0, 30.0), "{{date}}");
    let date_id = date_frame.common.id;
    page.add_shape(date_frame);

    // Time
    let time_frame = text_frame(Rect::new(36.0, 220.0, 288.0, 30.0), "{{time}}");
    let time_id = time_frame.common.id;
    page.add_shape(time_frame);

    // Location
    let location_frame = text_frame(Rect::new(36.0, 270.0, 288.0, 60.0), "{{location}}");
    let location_id = location_frame.common.id;
    page.add_shape(location_frame);

    // RSVP
    let rsvp_frame = text_frame(Rect::new(36.0, 380.0, 288.0, 60.0), "RSVP: {{rsvp}}");
    let rsvp_id = rsvp_frame.common.id;
    page.add_shape(rsvp_frame);

    let mut template = Template::new(
        "invitation-party",
        "Party Invitation",
        TemplateCategory::Personal,
        doc,
    );

    template.metadata.description = "Elegant party invitation card (5\" x 7\")".to_string();
    template.metadata.tags = vec![
        "invitation".to_string(),
        "party".to_string(),
        "celebration".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("event_name", "Event Name", event_id)
            .with_default("Birthday Celebration")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("date", "Date", date_id).with_default("Saturday, January 15, 2024"),
    );
    template.add_placeholder(
        Placeholder::text("time", "Time", time_id).with_default("7:00 PM - 11:00 PM"),
    );
    template.add_placeholder(
        Placeholder::text("location", "Location", location_id)
            .with_default("123 Celebration Lane\nAnytown, USA 12345"),
    );
    template.add_placeholder(
        Placeholder::text("rsvp", "RSVP Contact", rsvp_id)
            .with_default("Jane at (555) 123-4567\nby January 10th"),
    );

    template
}

/// Greeting card template.
pub fn greeting_card() -> Template {
    // 5" x 7" folded card (10" x 7" flat)
    let mut doc = Document::new(
        PageSize::Custom {
            width: 360.0,
            height: 504.0,
        },
        PageOrientation::Portrait,
    );

    let page = &mut doc.pages_mut()[0];

    // Image placeholder (top half)
    let image_frame = Shape::picture(Rect::new(20.0, 20.0, 320.0, 220.0));
    let image_id = image_frame.common.id;
    page.add_shape(image_frame);

    // Message
    let message_frame = text_frame(Rect::new(36.0, 270.0, 288.0, 180.0), "{{message}}");
    let message_id = message_frame.common.id;
    page.add_shape(message_frame);

    // Signature
    let signature_frame = text_frame(Rect::new(180.0, 460.0, 144.0, 30.0), "{{signature}}");
    let signature_id = signature_frame.common.id;
    page.add_shape(signature_frame);

    let mut template = Template::new(
        "greeting-card",
        "Greeting Card",
        TemplateCategory::Personal,
        doc,
    );

    template.metadata.description = "Simple greeting card with image and message".to_string();
    template.metadata.tags = vec![
        "card".to_string(),
        "greeting".to_string(),
        "thank you".to_string(),
        "birthday".to_string(),
    ];

    template.add_placeholder(
        Placeholder::image("image", "Card Image", image_id)
            .with_description("Photo or illustration for the card"),
    );
    template.add_placeholder(
        Placeholder::text("message", "Message", message_id).with_default(
            "Wishing you all the best on your special day!\n\nMay this year bring you joy, happiness, and wonderful memories.",
        ),
    );
    template.add_placeholder(
        Placeholder::text("signature", "Signature", signature_id)
            .with_default("With love,\nYour Name"),
    );

    template
}

// Office Templates

/// Achievement certificate template.
pub fn certificate_achievement() -> Template {
    // 11" x 8.5" landscape certificate
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Landscape);
    let page = &mut doc.pages_mut()[0];

    // Decorative border
    let mut outer_border = Shape::rectangle(Rect::new(20.0, 20.0, 752.0, 572.0));
    outer_border.fill = Fill::None;
    outer_border.line = Some(Stroke::new(Color::rgb(0.6, 0.5, 0.2), 4.0));
    page.add_shape(outer_border);

    let mut inner_border = Shape::rectangle(Rect::new(30.0, 30.0, 732.0, 552.0));
    inner_border.fill = Fill::None;
    inner_border.line = Some(Stroke::new(Color::rgb(0.6, 0.5, 0.2), 1.0));
    page.add_shape(inner_border);

    // Title
    let title_frame = text_frame(
        Rect::new(100.0, 60.0, 592.0, 50.0),
        "Certificate of Achievement",
    );
    page.add_shape(title_frame);

    // "This certifies that"
    let certify_frame = text_frame(Rect::new(100.0, 140.0, 592.0, 30.0), "This certifies that");
    page.add_shape(certify_frame);

    // Recipient name
    let name_frame = text_frame(Rect::new(100.0, 180.0, 592.0, 60.0), "{{recipient_name}}");
    let name_id = name_frame.common.id;
    page.add_shape(name_frame);

    // Achievement description
    let desc_frame = text_frame(Rect::new(100.0, 260.0, 592.0, 100.0), "{{achievement}}");
    let desc_id = desc_frame.common.id;
    page.add_shape(desc_frame);

    // Date
    let date_frame = text_frame(Rect::new(100.0, 400.0, 250.0, 30.0), "Date: {{date}}");
    let date_id = date_frame.common.id;
    page.add_shape(date_frame);

    // Signature line
    let mut sig_line = Shape::rectangle(Rect::new(442.0, 420.0, 200.0, 1.0));
    sig_line.fill = Fill::Solid(Color::BLACK);
    sig_line.line = None;
    page.add_shape(sig_line);

    // Signature label
    let sig_label = text_frame(Rect::new(442.0, 430.0, 200.0, 20.0), "{{signatory_title}}");
    let sig_id = sig_label.common.id;
    page.add_shape(sig_label);

    let mut template = Template::new(
        "certificate-achievement",
        "Achievement Certificate",
        TemplateCategory::Office,
        doc,
    );

    template.metadata.description =
        "Formal certificate of achievement with decorative border".to_string();
    template.metadata.tags = vec![
        "certificate".to_string(),
        "achievement".to_string(),
        "award".to_string(),
        "recognition".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("recipient_name", "Recipient Name", name_id)
            .with_default("John Smith")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("achievement", "Achievement", desc_id)
            .with_default("has successfully completed the requirements for\nExcellence in Professional Development"),
    );
    template.add_placeholder(
        Placeholder::text("date", "Date", date_id).with_default("January 15, 2024"),
    );
    template.add_placeholder(
        Placeholder::text("signatory_title", "Signatory Title", sig_id)
            .with_default("Program Director"),
    );

    template
}

/// Address label template (Avery 5160 compatible).
pub fn label_address() -> Template {
    // Single label: 2.625" x 1" (189 x 72 points)
    let mut doc = Document::new(
        PageSize::Custom {
            width: 189.0,
            height: 72.0,
        },
        PageOrientation::Landscape,
    );

    let page = &mut doc.pages_mut()[0];

    // Name
    let name_frame = text_frame(Rect::new(8.0, 8.0, 173.0, 18.0), "{{name}}");
    let name_id = name_frame.common.id;
    page.add_shape(name_frame);

    // Address line 1
    let addr1_frame = text_frame(Rect::new(8.0, 26.0, 173.0, 14.0), "{{address1}}");
    let addr1_id = addr1_frame.common.id;
    page.add_shape(addr1_frame);

    // Address line 2
    let addr2_frame = text_frame(Rect::new(8.0, 40.0, 173.0, 14.0), "{{address2}}");
    let addr2_id = addr2_frame.common.id;
    page.add_shape(addr2_frame);

    // City, State ZIP
    let csz_frame = text_frame(Rect::new(8.0, 54.0, 173.0, 14.0), "{{city_state_zip}}");
    let csz_id = csz_frame.common.id;
    page.add_shape(csz_frame);

    let mut template = Template::new(
        "label-address",
        "Address Label",
        TemplateCategory::Office,
        doc,
    );

    template.metadata.description = "Standard address label (Avery 5160 compatible)".to_string();
    template.metadata.tags = vec![
        "label".to_string(),
        "address".to_string(),
        "mailing".to_string(),
    ];

    template.add_placeholder(
        Placeholder::text("name", "Name", name_id)
            .with_default("John Smith")
            .required(),
    );
    template.add_placeholder(
        Placeholder::text("address1", "Address Line 1", addr1_id).with_default("123 Main Street"),
    );
    template.add_placeholder(
        Placeholder::text("address2", "Address Line 2", addr2_id).with_default("Apt 4B"),
    );
    template.add_placeholder(
        Placeholder::text("city_state_zip", "City, State ZIP", csz_id)
            .with_default("Anytown, ST 12345"),
    );

    template
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_builtins_valid() {
        // Ensure all built-in templates can be created without panicking
        let templates = vec![
            business_card_modern(),
            business_card_classic(),
            letterhead_corporate(),
            flyer_event(),
            brochure_trifold(),
            invitation_party(),
            greeting_card(),
            certificate_achievement(),
            label_address(),
        ];

        for template in templates {
            assert!(!template.metadata.id.is_empty());
            assert!(!template.metadata.name.is_empty());
            assert!(!template.placeholders.is_empty() || template.metadata.id == "greeting-card");
        }
    }

    #[test]
    fn test_business_card_apply() {
        let template = business_card_modern();

        let mut values = crate::PlaceholderValues::new();
        values.set("name", "Jane Doe");
        values.set("title", "CEO");
        values.set("email", "jane@company.com");

        let doc = template.apply(&values).unwrap();
        assert_eq!(doc.pages().len(), 1);
    }
}
