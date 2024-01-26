use url::Url;

use crate::{parser::{html::HTMLParser, css::CSSParser}, dom::Document};

#[derive(Debug)]
pub struct Context {
    html: HTMLParser,
    css: CSSParser,
    document: Document,
    url: Url,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            html: HTMLParser::default(),
            css: CSSParser::default(),
            document: Document::default(),
            url: Url::from_directory_path(std::env::current_dir().unwrap()).unwrap().join("tests/basic.html").unwrap()
        }
    }
}

impl Context {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            ..Default::default()
        }
    }

    pub fn load(&mut self) {
        if self.url.scheme() == "file" {
            self.html.load_from_file(self.url.to_file_path().unwrap()).unwrap();
        } else {
            unimplemented!();
        }
    }

    pub fn go(&mut self) {
        self.html.parse(&mut self.document).unwrap();
        self.css.push_raw_css(&std::include_str!("../../real_shit/default.css").to_string());
        self.css.push_many(self.document.find_css_sources());
        self.document.add_styles(self.css.parse_stylesheets().unwrap());
        self.document.cascade();
        println!("{:#?}", self);
    }
}
