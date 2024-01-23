use std::{fs::File, path::PathBuf, io::Read};

use reqwest::Url;

use thiserror::Error;

use super::Char;

#[derive(Debug, Default)]
pub struct CSSParser {
    tokenizer: CSSTokenizer,
    tokens: Vec<CSSToken>,
    sources: Vec<CSSSource>,
}

impl CSSParser {
    pub fn push_url(&mut self, url: &Url) {
        self.sources.push(CSSSource::URL(url.clone()));
    }

    pub fn push_raw_css(&mut self, source: &String) {
        self.sources.push(CSSSource::Raw(source.clone()));
    }

    pub fn push_file(&mut self, file: &PathBuf) {
        self.sources.push(CSSSource::Local(file.clone()));
    }

    pub fn push_many(&mut self, sources: Vec<CSSSource>) {
        self.sources.extend(sources);
    }

    pub fn parse(&mut self) -> Result<Style, CSSError> {
        for source in &self.sources {
            if let CSSSource::Local(file) = source {
                self.tokenizer.load_from_file(file)?;
            } else if let CSSSource::Raw(css) = source {
                self.tokenizer.load_raw(css)?;
            } else if let CSSSource::URL(url) = source {
                self.tokenizer.load_from_url(url)?;
            }
            self.tokenizer.tokenize(&mut self.tokens)?;
        }
        Ok(Style::default())
    }
}

#[derive(Default, Debug)]
pub struct CSSTokenizer {
    source: String,
    source_idx: usize,
}

impl CSSTokenizer {
    pub fn tokenize(&mut self, tokens: &mut Vec<CSSToken>) -> Result<(), CSSError> {
        self.preprocess()?;
        loop {
            match self.consume() {
                Char::Char('\u{0009}' | '\u{000A}' | '\u{0020}') => {
                    tokens.push(self.consume_whitespace_token()?);
                },
                Char::Char(c) if ('A'..='Z').contains(&c) || ('a'..='z').contains(&c) || c as u32 > '\u{0080}' as u32 || c == '_' => {
                    self.reconsume();
                    tokens.push(self.consume_ident_like_token()?);
                },
                a => {
                    do yeet CSSError::UnimplementedCodePoint(a);
                },
            }
        }
    }

    pub fn load_from_file(&mut self, path: &PathBuf) -> Result<(), CSSError> {
        self.source.clear();
        File::open(path).unwrap().read_to_string(&mut self.source)?;
        Ok(())
    }

    pub fn load_raw(&mut self, string: &String) -> Result<(), CSSError> {
        self.source.replace_range(.., &string);
        Ok(())
    }
    
    pub fn load_from_url(&mut self, _url: &Url) -> Result<(), CSSError> {
        todo!();
    }

    fn preprocess(&mut self) -> Result<(), CSSError> {
        self.source = self.source.replace("\u{000D}", "\u{000A}")
                    .replace("\u{000C}", "\u{000A}")
                    .replace("\u{000D}\u{000A}", "\u{000A}")
                    .replace("\u{0000}", "\u{FFFD}");
        //TODO: Filter out surrogates
        Ok(())
    }

    fn reconsume(&mut self) {
        self.source_idx = self.source_idx.saturating_sub(1);
    }

    fn consume(&mut self) -> Char {
        if let Some(char) = self.source.chars().nth(self.source_idx) {
            self.source_idx += 1;
            Char::Char(char)
        } else {
            Char::Eof
        }
    }

    fn consume_escaped(&mut self) -> Result<Option<char>, CSSError> {
        match self.consume() {
            Char::Char('\n') => {
                Ok(None)
            }
        }
    }

    fn consume_ident_sequence(&mut self) -> Result<String, CSSError> {
        let mut result = String::with_capacity(10);
        loop {
            match self.consume() {
                Char::Char(c) if ('A'..='Z').contains(&c) || ('a'..='z').contains(&c) || ('0'..='9').contains(&c) || c as u32 > '\u{0080}' as u32 || c == '_' || c == '-' => {
                    result.push(c);
                },
                Char::Char('\\') => {
                    self.reconsume();
                    if let Some(c) = self.consume_escaped()? {
                        result.push(c);
                    }
                }
                _ => {
                    self.reconsume();
                    break;
                }
            }
        }
        Ok(result)
    }

    fn consume_whitespace_token(&mut self) -> Result<CSSToken, CSSError> {
        while let Char::Char('\u{0009}' | '\u{000A}' | '\u{0020}') = self.consume() {}
        self.reconsume();
        Ok(CSSToken::Whitespace)
    }

    fn consume_ident_like_token(&mut self) -> Result<CSSToken, CSSError> {
        let string = self.consume_ident_sequence()?;
        Ok(CSSToken::Whitespace)
    }
}

#[derive(Debug, Error)]
pub enum CSSError {
    #[error("IO Failed!: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Unimplemented code point {0:?}!")]
    UnimplementedCodePoint(Char),
    #[error("EOF Reached!")]
    EOFReached,
    
}

#[derive(Debug, Default)]
pub struct Style {
    pub rules: Vec<Rule>,
}

impl Style {
    
}

#[derive(Debug)]
struct Rule {
    pub selectors: Vec<Selector>,  
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
enum Selector {

}

#[derive(Debug)]
enum Declaration {

}

#[derive(Debug)]
pub enum CSSToken {
    Whitespace,
    Delim(Char),
}

#[derive(Debug)]
pub enum CSSSource {
    Raw(String),
    URL(Url),
    Local(PathBuf),
}
