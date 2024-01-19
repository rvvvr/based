use std::path::PathBuf;

use browser_engine::parser::html::Parser;

fn main() {
    let mut parser = Parser::new();
    parser.load_from_file(PathBuf::from("tests/basic.html"));
    parser.parse().unwrap();
}
