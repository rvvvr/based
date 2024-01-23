pub mod html;
pub mod css;

#[derive(Debug)]
pub enum Char {
    Eof,
    Char(char),
}
