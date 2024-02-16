use std::io::Cursor;

use reqwest::{Client, ClientBuilder};
use url::Url;
use vello::{peniko::Font, SceneBuilder};

use crate::{
    dom::Document,
    layout::LayoutInfo,
    parser::{css::CSSParser, html::HTMLParser},
    renderer::{PageRenderer, RenderInfo},
};

#[derive(Debug)]
pub struct Context {
    html: HTMLParser,
    css: CSSParser,
    document: Document,
    url: Url,
    viewport: Viewport,
    renderer: PageRenderer,
    fonts: Vec<Font>,
    client: Client,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            html: HTMLParser::default(),
            css: CSSParser::default(),
            document: Document::default(),
            viewport: Viewport::default(),
            renderer: PageRenderer::default(),
            url: Url::from_directory_path(std::env::current_dir().unwrap())
                .unwrap()
                .join("real_shit/basic.html")
                .unwrap(),
            fonts: Vec::new(),
	    client: Self::make_request_client(),
        }
    }
}

impl Context {
    pub fn make_request_client() -> Client {
	ClientBuilder::new()
	    .user_agent("Mozilla/5.0 (who cares) based/0.0.0 (https://github.com/rvvvr/based)") // to be made a better static somewhere
	    .cookie_store(true) //to be replaced with my own
	    .build().unwrap()
    }
    
    pub fn new(url: Url) -> Self {
        Self {
            url,
	    client: Self::make_request_client(),
            ..Default::default()
        }
    }

    pub async fn load(&mut self) {
	println!("{:?}", self.url);
        if self.url.scheme() == "file" {
            self.html
                .load_from_file(self.url.to_file_path().unwrap())
                .unwrap();
        } else if let "http" | "https" = self.url.scheme() {
	    let mut shmeep = self.client.get(self.url.clone()).send().await.unwrap().bytes().await.unwrap();
            self.html.load_from_whatever(&mut Cursor::new(shmeep)).unwrap();
        } else {
	    unimplemented!();
	}
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.viewport.resize(width, height);
    }

    pub fn go(&mut self) {
        self.html.parse(&mut self.document).unwrap();
        self.css
            .push_raw_css(&std::include_str!("../../real_shit/default.css").to_string());
        self.css.push_many(self.document.find_css_sources());
        self.document
            .add_styles(self.css.parse_stylesheets().unwrap());
        self.document.cascade(self.viewport);
    }

    pub fn render(&mut self, builder: &mut SceneBuilder, render_info: RenderInfo) {
        self.renderer.render(
            self.viewport,
            &self.document.children,
            builder,
            100.,
            render_info,
        );
    }

    pub fn layoutify(&mut self, scale_factor: f64) {
        self.document.layoutify(self.viewport, scale_factor);
    }
}


//this type is awkward, i'd like to remove it at some point.
#[derive(Debug, Default, Copy, Clone)]
pub struct Viewport {
    pub width: usize,
    pub height: usize,
}

impl Viewport {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    pub fn into_layout(&self) -> LayoutInfo {
        LayoutInfo {
            x: 0.,
            y: 0.,
            width: self.width as f64,
            height: self.height as f64,
            ..Default::default()
        }
    }
}
