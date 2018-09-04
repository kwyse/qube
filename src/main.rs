extern crate pulldown_cmark;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use pulldown_cmark::{html, Parser};

mod server;

const ARTICLES_PATH: &str = "./articles";

fn main() {
    let address = "localhost:12333";
    println!("Serving knowledge at {}!", address);

    server::serve(address, find_article).expect("stuff");
}

fn find_article(path: &str) -> String {
    enrich_files().get(path.trim_matches('/'))
        .map(parse_markdown)
        .unwrap_or("Nothing here! Best write something!".to_string())
}

fn parse_markdown(markdown: &String) -> String {
    let parser = Parser::new(markdown);
    let mut buf = String::new();
    html::push_html(&mut buf, parser);

    buf
}

fn enrich_files() -> HashMap<String, String> {
    let mut enriched_files = HashMap::new();

    let filenames = determine_links();
    for filename in &filenames {
        if let Ok(mut file) = File::open(format!("{}/{}.md", ARTICLES_PATH, filename)) {
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();

            let links_excluding_current = filenames.iter()
                .map(ToString::to_string)
                .filter(|name| name != filename)
                .map(|name| name.replace("_", " "))
                .collect::<Vec<String>>();

            let enriched_file = add_hyperlinks(&contents, &links_excluding_current);
            enriched_files.insert(filename.to_string(), enriched_file);
        }
    }

    enriched_files
}

fn add_hyperlinks(contents: &str, links: &[String]) -> String {
    let mut hyperlinked_contents = contents.to_string();
    for link in links {
        let hyperlink = format!("[{}]({})", link, link.replace(" ", "_"));
        hyperlinked_contents = hyperlinked_contents.replacen(link, &hyperlink, 1);
    }

    hyperlinked_contents
}

fn determine_links() -> Vec<String> {
    get_file_names(ARTICLES_PATH).unwrap_or(Vec::new())
}

fn get_file_names<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    let mut names = Vec::new();

    for entry in fs::read_dir(path)? {
        let file = entry?;

        let filename = file.path()
            .file_stem()
            .and_then(|name| name.to_str())
            .map(ToString::to_string);

        if let Some(filename) = filename {
            names.push(filename);
        }
    }

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_contents_returns_empty_string() {
        assert_eq!(add_hyperlinks("", &[]), "".to_string());
        assert_eq!(add_hyperlinks("", &["link".to_string()]), "".to_string());
    }

    #[test]
    fn populated_contents_with_empty_links_returns_unaltered_contents() {
        assert_eq!(add_hyperlinks("contents", &[]), "contents".to_string());
    }

    #[test]
    fn populated_contents_with_different_link_returns_unaltered_contents() {
        assert_eq!(add_hyperlinks("contents", &["link".to_string()]), "contents".to_string());
    }

    #[test]
    fn populated_contents_with_matching_link_returns_hyperlinked_contents() {
        assert_eq!(add_hyperlinks("link", &["link".to_string()]), "[link](link)".to_string());
    }
}
