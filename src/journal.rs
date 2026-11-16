use gray_matter::engine::YAML;
use gray_matter::Matter;
use jiff::{civil::Date, Span};
use markdown::mdast::Node;
use markdown::{to_mdast, ParseOptions};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use url::Url;

#[derive(Debug)]
pub struct Journal {
    pub title: String,
    pub github_repo: String,
    pub description: String,
    pub created_at: Date,
    pub entries: Vec<Entry>,
}

#[derive(Debug)]
pub struct Entry {
    pub date: Date,
    pub title: String,
    pub content: String,
    pub images: Vec<Image>,
    pub time_spent: Span,
}

#[derive(Debug)]
pub struct Image {
    pub url: Url,
    pub alt_text: String,
}

#[derive(Deserialize)]
struct FrontMatter {
    title: String,
    github_repo: String,
    description: String,
    created_at: Date,
}

impl Journal {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file_content = fs::read_to_string(path)?;
        Self::parse(&file_content)
    }

    pub fn parse(file_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Parse YAML frontmatter for Journal metadata
        let matter = Matter::<YAML>::new();
        let parsed = matter.parse::<FrontMatter>(file_content)?;
        let fm = parsed.data.ok_or("Missing YAML frontmatter")?;

        let mut journal = Journal {
            title: fm.title,
            github_repo: fm.github_repo,
            description: fm.description,
            created_at: fm.created_at,
            entries: Vec::new(),
        };

        // 2. Parse Markdown body into an AST
        let ast = to_mdast(&parsed.content, &ParseOptions::default())
            .map_err(|err| format!("{:?}", err))?;

        let mut current_entry: Option<Entry> = None;

        if let Node::Root(root) = ast {
            for node in root.children {
                match node {
                    // Assume H2 (`## 2026-11-16 - Entry Title`) starts a new Entry
                    Node::Heading(heading) if heading.depth == 2 => {
                        if let Some(entry) = current_entry.take() {
                            journal.add_entry(entry);
                        }

                        let header_text = extract_text(&Node::Heading(heading));
                        let (date, title) = parse_entry_header(&header_text, fm.created_at)?;

                        current_entry = Some(Entry {
                            date,
                            title,
                            content: String::new(),
                            images: Vec::new(),
                            time_spent: Span::new(),
                        });
                    }
                    // Extract images and text from paragraphs inside the current entry
                    Node::Paragraph(para) => {
                        if let Some(entry) = current_entry.as_mut() {
                            collect_images(&Node::Paragraph(para.clone()), entry);
                            let text = extract_text(&Node::Paragraph(para));

                            // Example: detect "Time spent: 1h 30m" using jiff's ISO/friendly Span parser
                            if let Some(span_str) = text.strip_prefix("Time spent:") {
                                if let Ok(span) = span_str.trim().parse::<Span>() {
                                    entry.time_spent = span;
                                    continue;
                                }
                            }

                            if !text.is_empty() {
                                if !entry.content.is_empty() {
                                    entry.content.push_str("\n\n");
                                }
                                entry.content.push_str(&text);
                            }
                        }
                    }
                    // Append other blocks (lists, code blocks, blockquotes) as text
                    other => {
                        if let Some(entry) = current_entry.as_mut() {
                            let text = extract_text(&other);
                            if !text.is_empty() {
                                if !entry.content.is_empty() {
                                    entry.content.push_str("\n\n");
                                }
                                entry.content.push_str(&text);
                            }
                        }
                    }
                }
            }
        }

        if let Some(entry) = current_entry {
            journal.add_entry(entry);
        }

        Ok(journal)
    }

    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
    }
}

impl Entry {
    pub fn add_image(&mut self, image: Image) {
        self.images.push(image);
    }
}

/// Splits `"2026-11-16 - My Title"` into `(Date, String)`
fn parse_entry_header(
    header: &str,
    fallback: Date,
) -> Result<(Date, String), Box<dyn std::error::Error>> {
    if let Some((date_part, title_part)) = header.split_once(" - ") {
        let date = date_part.trim().parse::<Date>()?;
        Ok((date, title_part.trim().to_string()))
    } else {
        Ok((fallback, header.trim().to_string()))
    }
}

/// Recursively collects all `Node::Image` items into `Entry::images`
fn collect_images(node: &Node, entry: &mut Entry) {
    if let Node::Image(img) = node {
        if let Ok(url) = Url::parse(&img.url) {
            entry.add_image(Image {
                url,
                alt_text: img.alt.clone(),
            });
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_images(child, entry);
        }
    }
}

/// Recursively extracts readable text from an AST node
fn extract_text(node: &Node) -> String {
    match node {
        Node::Text(t) => t.value.clone(),
        Node::InlineCode(c) => c.value.clone(),
        Node::Code(c) => c.value.clone(),
        _ => node
            .children()
            .map(|children| children.iter().map(extract_text).collect::<String>())
            .unwrap_or_default(),
    }
}
