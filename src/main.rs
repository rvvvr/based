use std::path::PathBuf;

use based::parser::html::HTMLParser;

fn main() {
    let mut parser = HTMLParser::new();
    parser.load_from_file(PathBuf::from("tests/basic.html")).unwrap();
    parser.parse().unwrap();
    println!("{:#?}", parser);
}
