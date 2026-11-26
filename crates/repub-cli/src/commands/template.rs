//! Template commands for the CLI.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use console::style;

use repub_templates::{TemplateLibrary, TemplateCategory, PlaceholderValues};
use repub_format_native as native;

use crate::cli::GlobalOptions;

/// Template subcommands.
#[derive(Subcommand, Debug)]
pub enum TemplateCommand {
    /// List available templates
    List(ListArgs),

    /// Show template details
    Info(InfoArgs),

    /// Create a document from a template
    Create(CreateArgs),

    /// Search templates
    Search(SearchArgs),
}

/// Arguments for template list.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter by category
    #[arg(short, long)]
    pub category: Option<String>,
}

/// Arguments for template info.
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Template ID
    pub id: String,
}

/// Arguments for template create.
#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Template ID
    pub template: String,

    /// Output file path
    pub output: PathBuf,

    /// Placeholder values as key=value pairs
    #[arg(short = 'p', long = "placeholder", value_name = "KEY=VALUE")]
    pub placeholders: Vec<String>,

    /// Interactive mode (prompt for placeholders)
    #[arg(short, long)]
    pub interactive: bool,
}

/// Arguments for template search.
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
}

/// Run template command.
pub fn run(cmd: TemplateCommand, global: &GlobalOptions) -> Result<()> {
    match cmd {
        TemplateCommand::List(args) => list(args, global),
        TemplateCommand::Info(args) => info(args, global),
        TemplateCommand::Create(args) => create(args, global),
        TemplateCommand::Search(args) => search(args, global),
    }
}

fn list(args: ListArgs, global: &GlobalOptions) -> Result<()> {
    let library = TemplateLibrary::with_builtins();

    // Filter by category if specified
    let category = args.category.as_ref().and_then(|c| {
        match c.to_lowercase().as_str() {
            "business" => Some(TemplateCategory::Business),
            "marketing" => Some(TemplateCategory::Marketing),
            "publications" => Some(TemplateCategory::Publications),
            "personal" => Some(TemplateCategory::Personal),
            "office" => Some(TemplateCategory::Office),
            "custom" => Some(TemplateCategory::Custom),
            _ => None,
        }
    });

    let templates: Vec<_> = if let Some(cat) = category {
        library.by_category(cat)
    } else {
        library.all().collect()
    };

    if global.output == crate::cli::OutputFormat::Json {
        let items: Vec<_> = templates.iter().map(|t| {
            serde_json::json!({
                "id": t.metadata.id,
                "name": t.metadata.name,
                "category": format!("{:?}", t.metadata.category),
                "description": t.metadata.description,
                "tags": t.metadata.tags,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        if templates.is_empty() {
            println!("No templates found.");
            return Ok(());
        }

        // Group by category
        let mut by_category: std::collections::HashMap<TemplateCategory, Vec<_>> = std::collections::HashMap::new();
        for template in templates {
            by_category.entry(template.metadata.category).or_default().push(template);
        }

        for category in TemplateCategory::all() {
            if let Some(templates) = by_category.get(category) {
                println!("\n{}", style(category.name()).bold().underlined());
                for template in templates {
                    println!("  {} - {}", 
                        style(&template.metadata.id).cyan(),
                        template.metadata.name
                    );
                    if !template.metadata.description.is_empty() {
                        println!("    {}", style(&template.metadata.description).dim());
                    }
                }
            }
        }
        println!();
    }

    Ok(())
}

fn info(args: InfoArgs, global: &GlobalOptions) -> Result<()> {
    let library = TemplateLibrary::with_builtins();
    
    let template = library.get(&args.id)
        .context(format!("Template not found: {}", args.id))?;

    if global.output == crate::cli::OutputFormat::Json {
        let placeholders: Vec<_> = template.placeholders.iter().map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "type": format!("{:?}", p.placeholder_type),
                "required": p.required,
                "default": p.default_value,
                "description": p.description,
            })
        }).collect();

        let info = serde_json::json!({
            "id": template.metadata.id,
            "name": template.metadata.name,
            "category": format!("{:?}", template.metadata.category),
            "description": template.metadata.description,
            "tags": template.metadata.tags,
            "version": template.metadata.version,
            "placeholders": placeholders,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!();
        println!("{}", style(&template.metadata.name).bold().cyan());
        println!("ID: {}", template.metadata.id);
        println!("Category: {}", template.metadata.category.name());
        println!("Version: {}", template.metadata.version);
        
        if !template.metadata.description.is_empty() {
            println!("\n{}", template.metadata.description);
        }

        if !template.metadata.tags.is_empty() {
            println!("\nTags: {}", template.metadata.tags.join(", "));
        }

        if !template.placeholders.is_empty() {
            println!("\n{}", style("Placeholders:").bold());
            for p in &template.placeholders {
                let required = if p.required { 
                    style("*").red().to_string() 
                } else { 
                    " ".to_string() 
                };
                println!("  {}{} ({:?})", required, style(&p.id).yellow(), p.placeholder_type);
                println!("    Name: {}", p.name);
                if !p.description.is_empty() {
                    println!("    {}", style(&p.description).dim());
                }
                if let Some(ref default) = p.default_value {
                    println!("    Default: {}", default);
                }
            }
        }
        println!();
    }

    Ok(())
}

fn create(args: CreateArgs, global: &GlobalOptions) -> Result<()> {
    let library = TemplateLibrary::with_builtins();
    
    let template = library.get(&args.template)
        .context(format!("Template not found: {}", args.template))?;

    let mut values = PlaceholderValues::new();

    // Parse placeholder values from command line
    for pair in &args.placeholders {
        if let Some((key, value)) = pair.split_once('=') {
            values.set(key, value);
        } else {
            anyhow::bail!("Invalid placeholder format: {}. Expected KEY=VALUE", pair);
        }
    }

    // Interactive mode
    if args.interactive {
        use dialoguer::{Input, theme::ColorfulTheme};

        for placeholder in &template.placeholders {
            if values.has(&placeholder.id) {
                continue; // Already provided
            }

            if !placeholder.description.is_empty() {
                // Show description as help text
                println!("  {}", style(&placeholder.description).dim());
            }

            let value: String = if let Some(ref default) = placeholder.default_value {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(&placeholder.name)
                    .default(default.clone())
                    .interact_text()?
            } else {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(&placeholder.name)
                    .interact_text()?
            };
            values.set(&placeholder.id, &value);
        }
    }

    // Apply template
    let document = template.apply(&values)
        .context("Failed to apply template")?;

    // Save document
    native::save(&document, &args.output, &native::SaveOptions::default())
        .context("Failed to save document")?;

    if !global.quiet {
        println!("{} Created {} from template '{}'", 
            style("✓").green().bold(),
            args.output.display(),
            template.metadata.name
        );
    }

    Ok(())
}

fn search(args: SearchArgs, global: &GlobalOptions) -> Result<()> {
    let library = TemplateLibrary::with_builtins();
    let results = library.search(&args.query);

    if global.output == crate::cli::OutputFormat::Json {
        let items: Vec<_> = results.iter().map(|t| {
            serde_json::json!({
                "id": t.metadata.id,
                "name": t.metadata.name,
                "category": format!("{:?}", t.metadata.category),
                "description": t.metadata.description,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        if results.is_empty() {
            println!("No templates found matching '{}'", args.query);
            return Ok(());
        }

        println!("\nFound {} template(s) matching '{}':\n", results.len(), args.query);
        for template in results {
            println!("  {} - {}", 
                style(&template.metadata.id).cyan(),
                template.metadata.name
            );
            if !template.metadata.description.is_empty() {
                println!("    {}", style(&template.metadata.description).dim());
            }
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        let library = TemplateLibrary::with_builtins();
        assert!(!library.is_empty());
    }

    #[test]
    fn test_search_templates() {
        let library = TemplateLibrary::with_builtins();
        let results = library.search("business");
        assert!(!results.is_empty());
    }
}
