pub mod html;
pub mod css;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Char {
    Eof,
    Char(char),
}
