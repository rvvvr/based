use std::{fs::File, path::{PathBuf}, io::Read};

use reqwest::Url;

use thiserror::Error;

use super::Char;

#[derive(Debug, Default)]
pub struct CSSParser {
    tokenizer: CSSTokenizer,
    tokens: Vec<CSSToken>,
    tokens_idx: usize,
    sources: Vec<CSSSource>,
    top_level: bool,
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
    
    fn reconsume(&mut self) {
        self.tokens_idx = self.tokens_idx.saturating_sub(1);
    }

    fn consume(&mut self) -> CSSToken {
        let out = self.tokens.get(self.tokens_idx).unwrap_or(&CSSToken::EOF);
        self.tokens_idx += 1;
        out.clone()
    }

    pub fn parse_stylesheets(&mut self) -> Result<Style, CSSError> {
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
        let style = self.consume_list_of_rules()?;
        Ok(style)
    }

    fn consume_list_of_rules(&mut self) -> Result<Style, CSSError> {
        let mut rules = vec![];
        loop {
            match self.consume() {
                CSSToken::Whitespace => {},
                CSSToken::EOF => {
                    break;
                },
                a => {
                    self.reconsume();
                    rules.push(self.consume_qualified_rule()?);
                }
            }
        }
        Ok(Style { rules })
    }

    fn consume_qualified_rule(&mut self) -> Result<Rule, CSSError> {
        let mut rule_builder = RuleBuilder::new(false);
        loop {
            match self.consume() {
                CSSToken::EOF => {
                    break;
                },
                CSSToken::CurlyOpen => {
                    rule_builder.append_to_blocks(self.consume_simple_block(CSSToken::CurlyClose)?);
                    break;
                },
                _ => {
                    self.reconsume();
                    rule_builder.append_to_prelude(self.consume_component_value()?);
                }
            }
        }
        Ok(rule_builder.build())
    }

    fn consume_component_value(&mut self) -> Result<Component, CSSError> {
        match self.consume() {
            CSSToken::CurlyOpen => {
                todo!();
            }
            a => {
                return Ok(Component::Token(a));
            }
        }
    }

    fn consume_simple_block(&mut self, ending: CSSToken) -> Result<SimpleBlock, CSSError> {
        if ending != CSSToken::CurlyClose {
            do yeet CSSError::WrongBlockEndingToken(ending);
        }
        let mut out = SimpleBlock::default();
        loop {
            match self.consume() {
                a if a == ending => {
                    break;
                },
                CSSToken::EOF => {
                    do yeet CSSError::EOFReached;
                }
                a => {
                    self.reconsume();
                    out.push_value(self.consume_component_value()?);
                }
            }
        }
        Ok(out)
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
                Char::Char(c) if c == '>' => {
                    tokens.push(CSSToken::Delim(Char::Char(c)));
                }
                Char::Char('{') => {
                    tokens.push(CSSToken::CurlyOpen);
                },
                Char::Char('}') => {
                    tokens.push(CSSToken::CurlyClose);
                },
                Char::Char(':') => {
                    tokens.push(CSSToken::Colon);
                },
                Char::Eof => {
                    tokens.push(CSSToken::EOF);
                    return Ok(());
                },
                a => {
                    do yeet CSSError::UnimplementedCodePoint(a);
                },
            }
            println!("{:?}", tokens);
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
        todo!();
        /*match self.consume() {
            Char::Char('\n') => {
                Ok(None)
            }
        }*/
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
                },
                _ => {
                    self.reconsume();
                    break;
                }
            }
        }
        Ok(result)
    }

    fn consume_whitespace_token(&mut self) -> Result<CSSToken, CSSError> {
        loop {
            match self.consume() {
                Char::Char('\u{0009}' | '\u{000A}' | '\u{0020}') => {},
                Char::Eof => {
                    break;
                },
                _ => {
                    self.reconsume();
                    break;
                },
            }
        }
        Ok(CSSToken::Whitespace)
    }

    fn consume_ident_like_token(&mut self) -> Result<CSSToken, CSSError> {
        let string = self.consume_ident_sequence()?;
        Ok(CSSToken::Ident(string))
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
    #[error("Unimplemented token {0:?}!")]
    UnimplementedToken(CSSToken),
    #[error("Block ending token was wrong: {0:?}")]
    WrongBlockEndingToken(CSSToken),
    
}

#[derive(Debug, Default)]
pub struct Style {
    pub rules: Vec<Rule>,
}

impl Style {
    
}

#[derive(Debug)]
pub struct RuleBuilder {
    pub preludes: Vec<Component>,
    pub blocks: Vec<SimpleBlock>,
    pub at: bool,
}

impl RuleBuilder {
    pub fn new(at: bool) -> Self {
        Self {
            preludes: vec![],
            blocks: vec![],
            at,
        }
    }

    pub fn append_to_prelude(&mut self, component: Component) {
        self.preludes.push(component);
    }

    pub fn append_to_blocks(&mut self, block: SimpleBlock) {
        self.blocks.push(block);
    }

    pub fn build(self) -> Rule {
        if self.at {
            todo!("at rule");
        }
        let mut selector = Selector::Placeheld;
        for component in self.preludes {
            selector.append(component);
        }
        println!("{:?}", selector);
        Rule { prelude: Prelude::Selector(selector), value: Block::Empty }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Rule {
    pub prelude: Prelude,  
    pub value: Block,
}

impl Rule {
}

#[derive(Debug, Clone, Default)]
pub enum Prelude {
    #[default]
    None,
    Selector(Selector),
}

#[derive(Debug, Clone, Default)]
pub enum Block {
    #[default]
    Empty,
    Declarations(Vec<Declaration>),
}

#[derive(Debug, Clone, Default)]
pub struct SimpleBlock {
    pub value: Vec<Component>
}

impl SimpleBlock {
    pub fn push_value(&mut self, component: Component) {
        self.value.push(component);
    }
}

#[derive(Debug, Clone)]
pub enum Selector {
    Placeheld,
    Universal,
    Type(String),
    Child(Box<Selector>, Box<Selector>)

}

impl Selector {
    pub fn append(&mut self, component: Component) {
        let new_self: Selector;
        match self {
            Selector::Placeheld => {
                match component {
                    Component::Token(t) => {
                        match t {
                            CSSToken::Whitespace => {
                                new_self = self.clone();
                            }
                            CSSToken::Ident(s) => {
                                if s == "*" {
                                    new_self = Selector::Universal
                                } else {
                                    new_self = Selector::Type(s);
                                }
                            },
                            _ => panic!(),
                        }
                    },
                    _ => panic!(),
                }
            },
            ref s if let Selector::Type(_) = s => {
                match component {
                    Component::Token(t) => {
                        match t {
                            CSSToken::Whitespace => {
                                new_self = self.clone();
                            },
                            CSSToken::Delim(Char::Char('>')) => {
                                new_self = Selector::Child(Box::new(self.clone()), Box::new(Selector::Placeheld));
                            },
                            _ => panic!("{:?}", t),
                        }
                    }
                    _ => panic!(),
                }
            },
            ref s if let Selector::Child(l, r) = s => {
                let new_l = l.clone();
                let mut new_r = r.clone();
                new_r.append(component);
                new_self = Selector::Child(new_l, new_r);
            }
            _ => panic!(),
        }
        *self = new_self;
    }
}

#[derive(Debug, Clone)]
pub enum Declaration {

}

#[derive(Debug, Clone)]
pub enum Component {
    Block(Block),
    Function(),
    Token(CSSToken),
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum CSSToken {
    Whitespace,
    Delim(Char),
    Ident(String),
    Colon,
    CurlyOpen,
    CurlyClose,
    EOF,
}

#[derive(Debug)]
pub enum CSSSource {
    Raw(String),
    URL(Url),
    Local(PathBuf),
}
