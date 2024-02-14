pub mod css;
pub mod html;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Char {
    Eof,
    Char(char),
}
