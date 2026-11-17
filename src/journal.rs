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
