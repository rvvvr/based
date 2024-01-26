use std::path::PathBuf;

use based::{parser::html::HTMLParser, context::Context};

fn main() {
    let mut context = Context::default();
    context.load();
    context.go();
}
