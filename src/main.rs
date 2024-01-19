use std::path::PathBuf;

use based::parser::html::Parser;

fn main() {
    let mut parser = Parser::new();
    parser.load_from_file(PathBuf::from("tests/basic.html"));
    let err = parser.parse();
    println!("{:#?}", parser);
    err.unwrap();
}
