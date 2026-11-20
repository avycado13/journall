use gray_matter::engine::YAML;
use gray_matter::Matter;
use jiff::ZonedDateTime;
use jiff::{civil::Date, Span};
use markdown::mdast::{Node, Root};
use markdown::{Constructs, ParseOptions};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use url::Url;

#[derive(Debug, serde::Serialize)]
pub struct Journal {
    pub title: String,
    pub github_repo: String,
    pub description: String,
    pub created_at: Date,

    #[serde(skip_serializing)]
    pub entries: Vec<Entry>,
}

#[derive(Debug)]
pub struct Entry {
    pub date: Date,
    pub title: String,
    pub content: Vec<Node>,
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
        // Frontmatter parsing
        let matter = Matter::<YAML>::new();
        let result = matter.parse(file_content)?;
        let frontmatter: FrontMatter = result.data.ok_or("Missing front matter")?;

        let mut journal = Journal {
            title: frontmatter.title,
            github_repo: frontmatter.github_repo,
            description: frontmatter.description,
            created_at: frontmatter.created_at,
            entries: Vec::new(),
        };

        let options = &ParseOptions {
            constructs: Constructs {
                frontmatter: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        };
        let tree = markdown::to_mdast(file_content, options).map_err(|err| err.to_string())?;

        let mut header = None;
        let mut body = Vec::new();
        if let Some(children) = tree.children() {
            for node in children {
                if matches!(node, Node::Heading(heading) if heading.depth == 1) {
                    if let Some((date, title)) = header.take() {
                        journal.entries.push(Entry {
                            date,
                            title,
                            content: std::mem::take(&mut body),
                            images: Vec::new(),
                            time_spent: Span::new(),
                        });
                    }
                    header = Some(parse_entry_header(
                        &extract_text(node),
                        ZonedDateTime::now().date(),
                    )?);
                } else if header.is_some() {
                    body.push(node.clone());
                }
            }
        }
        if let Some((date, title)) = header {
            journal.entries.push(Entry {
                date,
                title,
                content: body,
                images: Vec::new(),
                time_spent: Span::new(),
            });
        }
        Ok(journal)
    }

    pub fn journal_to_markdown(&self) -> Result<String, Box<dyn std::error::Error>> {
        nodes_to_markdown(&self.journal_to_nodes()?)
    }

    pub fn journal_to_nodes(&self) -> Result<Vec<Node>, Box<dyn std::error::Error>> {
        let mut ast: Vec<Node> = Vec::new();
        ast.push(Node::Yaml(markdown::mdast::Yaml {
            value: yaml_serde::to_string(self)?,
            position: None,
        }));
        for entry in self.entries.iter() {
            ast.push(Node::Heading(markdown::mdast::Heading {
                children: vec![Node::Text(markdown::mdast::Text {
                    value: format_entry_header(&entry.date, &entry.title),
                    position: None,
                })],
                position: None,
                depth: 1,
            }));
            ast.extend(entry.content.iter().cloned());
        }
        Ok(ast)
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

pub fn nodes_to_markdown(nodes: &[Node]) -> Result<String, Box<dyn std::error::Error>> {
    let root = Node::Root(Root {
        children: nodes.to_vec(),
        position: None,
    });

    let md = mdast_util_to_markdown::to_markdown(&root).map_err(|e| e.to_string())?;

    Ok(md)
}
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

fn format_entry_header(date: &Date, title: &str) -> String {
    format!("{date} - {title}")
}
fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    match node {
        Node::Text(t) => text.push_str(&t.value),
        Node::InlineCode(c) => text.push_str(&c.value),
        _ => {
            if let Some(children) = node.children() {
                for child in children {
                    text.push_str(&extract_text(child));
                }
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn parse_entries_preserves_mixed_markdown_bodies() {
        let input = "---\ntitle: Journal\ngithub_repo: example/repo\ndescription: Notes\ncreated_at: 2026-06-01\n---\n\n# 2026-06-08 - First\n\nA **bold** paragraph.\n\n- a list\n- another item\n\n```rust\nlet x = 1;\n```\n\n# 2026-06-09 - Second\n\n![diagram](https://example.com/diagram.png)\n";
        let journal = Journal::parse(input).unwrap();
        assert_eq!(journal.entries.len(), 2);
        assert_eq!(journal.entries[0].title, "First");
        let first = nodes_to_markdown(&journal.entries[0].content).unwrap();
        let second = nodes_to_markdown(&journal.entries[1].content).unwrap();
        assert!(first.contains("**bold**"));
        assert!(first.contains("a list"));
        assert!(first.contains("let x = 1;"));
        assert!(!first.contains("diagram"));
        assert!(second.contains("![diagram](https://example.com/diagram.png)"));
    }

    #[test]
    fn journal_to_nodes_includes_entry_body() {
        let input = "---\ntitle: Journal\ngithub_repo: example/repo\ndescription: Notes\ncreated_at: 2026-06-01\n---\n\n# 2026-06-08 - First\n\nA **bold** paragraph.\n\n- a list\n";
        let journal = Journal::parse(input).unwrap();
        let nodes = journal.journal_to_nodes().unwrap();
        assert!(matches!(nodes[0], Node::Yaml(_)));
        assert!(matches!(nodes[1], Node::Heading(_)));
        assert!(nodes.iter().any(|node| matches!(node, Node::Paragraph(_))));
        assert!(nodes.iter().any(|node| matches!(node, Node::List(_))));
    }

    #[test]
    fn test_parse_entry_header_with_date() {
        let fallback = date(2026, 6, 1);
        let result = parse_entry_header("2026-06-08 - worked on project", fallback).unwrap();

        assert_eq!(result, (date(2026, 6, 8), "worked on project".to_string()));
    }

    #[test]
    fn test_parse_entry_header_fallback() {
        let fallback = date(2026, 6, 1);
        let result = parse_entry_header("worked on project", fallback).unwrap();

        assert_eq!(result, (fallback, "worked on project".to_string()));
    }

    #[test]
    fn test_parse_entry_header_invalid_date() {
        let fallback = date(2026, 6, 1);
        let result = parse_entry_header("not-a-date - worked on project", fallback);

        assert!(result.is_err());
    }
}
