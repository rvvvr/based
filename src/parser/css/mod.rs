use std::{
    cmp::Ordering, collections::HashMap, fs::File, io::Read, mem::Discriminant, num::ParseIntError,
    path::PathBuf,
};

use font_types::Tag;
use reqwest::Url;
use thiserror::Error;

use self::properties::{
    Colour, Dimensionality, Display, FontFamily, FontSize, FontWeight, Property, TextAlign,
};
use super::Char;
use crate::util::approx_eq;

pub mod cascader;
pub mod properties;

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

    pub fn push_pretokenized(&mut self, tokens: Vec<CSSToken>) {
        self.tokens.extend(tokens);
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

    fn peek(&self) -> CSSToken {
        let out = self.tokens.get(self.tokens_idx).unwrap_or(&CSSToken::EOF);
        out.clone()
    }

    pub fn parse_stylesheets(&mut self) -> Result<Vec<Style>, CSSError> {
        let mut styles = vec![];
        for source in self.sources.to_vec() {
            self.tokens.clear();
            self.tokens_idx = 0;
            if let CSSSource::Local(file) = source {
                self.tokenizer.load_from_file(&file)?;
            } else if let CSSSource::Raw(css) = source {
                self.tokenizer.load_raw(&css)?;
            } else if let CSSSource::URL(url) = source {
                self.tokenizer.load_from_url(&url)?;
            }
            self.tokenizer.tokenize(&mut self.tokens)?;
            styles.push(self.consume_list_of_rules()?);
        }
        for style in &mut styles {
            style.let_em_know();
        }
        Ok(styles)
    }

    pub fn parse_declaration_list(
        &mut self,
    ) -> Result<Vec<(Discriminant<DeclarationKind>, Declaration)>, CSSError> {
        let mut declarations = vec![];
        loop {
            match self.consume() {
                CSSToken::Whitespace | CSSToken::Semicolon => {}
                a @ CSSToken::Ident(_) => {
                    let mut components = vec![Component::Token(a)];
                    loop {
                        match self.peek() {
                            CSSToken::Semicolon | CSSToken::EOF => {
                                break;
                            }
                            _ => {
                                components.push(self.consume_component_value()?);
                            }
                        }
                    }
                    let declaration = self.consume_declaration(components)?;
                    declarations.push((std::mem::discriminant(&declaration.kind), declaration));
                }
                CSSToken::EOF => {
                    break;
                }
                a => {
                    unimplemented!("{:?}", a);
                }
            }
        }
        Ok(declarations)
    }

    fn consume_declaration(&self, components: Vec<Component>) -> Result<Declaration, CSSError> {
        let mut builder = DeclarationBuilder::default();
        let mut iter = components.iter().peekable();
        if let Some(Component::Token(CSSToken::Ident(t))) = iter.next() {
            builder.set_kind(t.clone());
        }
        while let Some(Component::Token(CSSToken::Whitespace)) = iter.peek() {
            iter.next();
        }
        if let Some(Component::Token(CSSToken::Colon)) = iter.next() {
        } else {
            do yeet CSSError::EOFReached;
        }
        while let Some(Component::Token(CSSToken::Whitespace)) = iter.peek() {
            iter.next();
        }
        if let Some(Component::Token(CSSToken::EOF)) = iter.peek() {
        } else {
            builder.push_value(iter.next().unwrap().clone());
        }
        //TODO: Handle !important

        Ok(builder.build()?)
    }

    fn consume_list_of_rules(&mut self) -> Result<Style, CSSError> {
        let mut rules = vec![];
        loop {
            match self.consume() {
                CSSToken::Whitespace => {}
                CSSToken::EOF => {
                    break;
                }
                a => {
                    self.reconsume();
                    rules.push(self.consume_qualified_rule()?);
                }
            }
        }
        Ok(Style {
            rules,
            level: StyleLevel::UserAgent,
        })
    }

    fn consume_qualified_rule(&mut self) -> Result<Rule, CSSError> {
        let mut rule_builder = RuleBuilder::new(false);
        loop {
            match self.consume() {
                CSSToken::EOF => {
                    break;
                }
                CSSToken::CurlyOpen => {
                    rule_builder.append_to_blocks(self.consume_simple_block(CSSToken::CurlyClose)?);
                    break;
                }
                _ => {
                    self.reconsume();
                    rule_builder.append_to_prelude(self.consume_component_value()?);
                }
            }
        }
        Ok(rule_builder.build()?)
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
                }
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
            self.consume_comments()?;
            match self.consume() {
                Char::Char('\u{0009}' | '\u{000A}' | '\u{0020}') => {
                    tokens.push(self.consume_whitespace_token()?);
                }
                Char::Char('A'..='Z' | 'a'..='z' | '_' | '\u{0080}'..='\u{10FFFF}') => {
                    self.reconsume();
                    tokens.push(self.consume_ident_like_token()?);
                }
                Char::Char(c @ ('>' | '*')) => {
                    tokens.push(CSSToken::Delim(Char::Char(c)));
                }
                Char::Char('{') => {
                    tokens.push(CSSToken::CurlyOpen);
                }
                Char::Char('}') => {
                    tokens.push(CSSToken::CurlyClose);
                }
                Char::Char(':') => {
                    tokens.push(CSSToken::Colon);
                }
                Char::Char(';') => {
                    tokens.push(CSSToken::Semicolon);
                }
                Char::Char('0'..='9') => {
                    self.reconsume();
                    tokens.push(self.consume_numeric()?);
                }
                Char::Char(c @ ('+' | '-' | '.')) => {
                    if let Char::Char('0'..='9') = self.peek() {
                        self.reconsume();
                        tokens.push(self.consume_numeric()?);
                    } else {
                        tokens.push(CSSToken::Delim(Char::Char(c)));
                    }
                }
                Char::Char(',') => {
                    tokens.push(CSSToken::Comma);
                }
                Char::Eof => {
                    tokens.push(CSSToken::EOF);
                    return Ok(());
                }
                a => {
                    do yeet CSSError::UnimplementedCodePoint(a);
                }
            }
        }
    }

    pub fn load_from_file(&mut self, path: &PathBuf) -> Result<(), CSSError> {
        self.source.clear();
        self.source_idx = 0;
        File::open(path).unwrap().read_to_string(&mut self.source)?;
        Ok(())
    }

    pub fn load_raw(&mut self, string: &String) -> Result<(), CSSError> {
        self.source.replace_range(.., &string);
        self.source_idx = 0;
        Ok(())
    }

    pub fn load_from_url(&mut self, _url: &Url) -> Result<(), CSSError> {
        todo!();
    }

    fn preprocess(&mut self) -> Result<(), CSSError> {
        self.source = self
            .source
	    .replace("\u{000D}\u{000A}", "\u{000A}")
            .replace("\u{000D}", "\u{000A}")
            .replace("\u{000C}", "\u{000A}")
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

    fn peek(&self) -> Char {
        if let Some(char) = self.source.chars().nth(self.source_idx) {
            Char::Char(char)
        } else {
            Char::Eof
        }
    }

    fn peek_n(&self, n: usize) -> Char {
        if let Some(char) = self.source.chars().nth(self.source_idx + n - 1) {
            Char::Char(char)
        } else {
            Char::Eof
        }
    }

    fn consume_comments(&mut self) -> Result<(), CSSError> {
        if matches!(self.peek(), Char::Char('/')) && matches!(self.peek_n(2), Char::Char('*')) {
            while !(matches!(self.peek(), Char::Char('*'))
                && matches!(self.peek_n(2), Char::Char('/')))
            {
                self.consume();
            }
            self.consume();
            self.consume();
        }
        Ok(())
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
                Char::Char(
                    c @ ('A'..='Z' | 'a'..='z' | '0'..='9' | '\u{0080}'..='\u{10FFFF}' | '_' | '-'),
                ) => {
                    result.push(c);
                },
                Char::Char('\\') => {
                    self.reconsume();
                    if let Some(c) = self.consume_escaped()? {
                        result.push(c);
                    }
                },
		//caught this edge case while unit testing! turns out they are valuable!!!!
		Char::Eof => {
		    break;
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
                Char::Char('\u{0009}' | '\u{000A}' | '\u{0020}') => {}
                Char::Eof => {
                    break;
                }
                _ => {
                    self.reconsume();
                    break;
                }
            }
        }
        Ok(CSSToken::Whitespace)
    }

    fn consume_ident_like_token(&mut self) -> Result<CSSToken, CSSError> {
        let string = self.consume_ident_sequence()?;
        Ok(CSSToken::Ident(string))
    }

    fn consume_numeric(&mut self) -> Result<CSSToken, CSSError> {
        let number = self.consume_number()?;
        if let Char::Char(c) = self.peek() {
            if ('A'..='Z').contains(&c)
                || ('a'..='z').contains(&c)
                || c as u32 > '\u{0080}' as u32
                || c == '_'
                || c == '\\'
            {
                let unit = self.consume_ident_sequence()?;
                return Ok(CSSToken::Number(CSSNumber::Unit(
                    number,
                    Unit::from_string(unit).unwrap_or_default(),
                )));
            } else if c == '%' {
                self.consume();
                return Ok(CSSToken::Number(CSSNumber::Percentage(number)));
            }
        };
        Ok(CSSToken::Number(CSSNumber::Number(number)))
    }

    fn consume_number(&mut self) -> Result<Numeric, CSSError> {
        let mut rep = NumberRep::default();
        if let Char::Char('+' | '-') = self.peek() {
            rep.set_sign(match self.consume() {
                Char::Char(c) => c,
                Char::Eof => do yeet CSSError::EOFReached,
            });
        };
        while let Char::Char('0'..='9') = self.peek() {
            rep.append_to_integer(match self.consume() {
                Char::Char(c) => c,
                Char::Eof => do yeet CSSError::EOFReached,
            });
        }
        if let Char::Char('.') = self.peek() {
            self.consume();
            while let Char::Char('0'..='9') = self.peek() {
                rep.append_to_decimal(match self.consume() {
                    Char::Char(c) => c,
                    Char::Eof => do yeet CSSError::EOFReached,
                });
            }
        }
        if let Char::Char('E' | 'e') = self.peek() {
            self.consume();
	    if let Char::Char('-' | '+') = self.peek() {
		rep.set_e_sign(match self.consume() {
		    Char::Char(c) => c,
		    Char::Eof => do yeet CSSError::EOFReached,
		});
	    }
            while let Char::Char('0'..='9') = self.peek() {
                rep.append_to_exponent(match self.consume() {
                    Char::Char(c) => c,
                    Char::Eof => do yeet CSSError::EOFReached,
                });
            }
        }
        Ok(rep.into_numeric()?)
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
    #[error("Couldn't parse int! {0:?}")]
    ParseIntError(#[from] ParseIntError),
    #[error("Unexpected token {0:?}! Expected: {1:?}")]
    UnexpectedToken(CSSToken, CSSToken),
}

#[derive(Debug, Default)]
pub struct Style {
    pub rules: Vec<Rule>,
    pub level: StyleLevel,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum StyleLevel {
    #[default]
    Author,
    User,
    UserAgent,
}

impl Style {
    pub fn let_em_know(&mut self) {
        for rule in &mut self.rules {
            if let Block::Declarations(ref mut declarations) = rule.value {
                for (_, declaration) in declarations {
                    declaration.level = self.level;
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct StyleData {
    pub styles: Vec<Style>,
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

    pub fn build(self) -> Result<Rule, CSSError> {
        if self.at {
            todo!("at rule");
        }
        let mut selector = Selector::Placeheld;
        for component in self.preludes {
            selector.append(component);
        }
        let mut declarations: HashMap<Discriminant<DeclarationKind>, Declaration> =
            HashMap::default();
        for ref mut block in self.blocks {
            declarations.extend(block.parse_as_declarations()?)
        }
        Ok(Rule {
            prelude: Prelude::Selector(selector),
            value: Block::Declarations(declarations),
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Rule {
    pub prelude: Prelude,
    pub value: Block,
}

impl Rule {
    pub fn squash(&mut self, other: &Rule) {
        if let Block::Declarations(ref mut self_declarations) = self.value {
            if let Block::Declarations(ref other_declarations) = other.value {
                for (discriminant, declaration) in other_declarations {
                    if self_declarations.contains_key(discriminant) {
                        let mutclaration =
                            unsafe { self_declarations.get_mut(discriminant).unwrap_unchecked() };
                        if declaration >= mutclaration {
                            *mutclaration = declaration.clone();
                        }
                    } else {
                        self_declarations.insert(*discriminant, declaration.clone());
                    }
                }
            }
        }
    }
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
    Declarations(HashMap<Discriminant<DeclarationKind>, Declaration>),
}

#[derive(Debug, Clone, Default)]
pub struct SimpleBlock {
    pub value: Vec<Component>,
}

impl SimpleBlock {
    pub fn push_value(&mut self, component: Component) {
        self.value.push(component);
    }

    pub fn parse_as_declarations(
        &mut self,
    ) -> Result<Vec<(Discriminant<DeclarationKind>, Declaration)>, CSSError> {
        let tokens = self
            .value
            .iter()
            .map(|c| match c {
                Component::Token(t) => t.clone(),
                _ => CSSToken::Whitespace,
            })
            .collect::<Vec<_>>();
        let mut parser = CSSParser::default();
        parser.push_pretokenized(tokens);
        Ok(parser.parse_declaration_list()?)
    }
}

#[derive(Debug, Clone)]
pub enum Selector {
    Placeheld,
    Universal,
    Type(String),
    Child(Box<Selector>, Box<Selector>),
    NextSibling(Box<Selector>, Box<Selector>),
    Both(Box<Selector>, Box<Selector>),
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
                                //TODO:
                                //Id selectors
                                //attribute selectors
                                //pseudo-classes
                                //class selectors
                                new_self = Selector::Type(s);
                            }
                            CSSToken::Delim(Char::Char('*')) => {
                                new_self = Selector::Universal;
                            }
                            _ => panic!(),
                        }
                    }
                    _ => panic!(),
                }
            }
            Selector::Type(_) => match component {
                Component::Token(t) => match t {
                    CSSToken::Whitespace => {
                        new_self = self.clone();
                    }
                    CSSToken::Delim(Char::Char('>')) => {
                        new_self =
                            Selector::Child(Box::new(self.clone()), Box::new(Selector::Placeheld));
                    }
                    CSSToken::Delim(Char::Char('+')) => {
                        new_self = Selector::NextSibling(
                            Box::new(self.clone()),
                            Box::new(Selector::Placeheld),
                        );
                    }
                    CSSToken::Comma => {
                        new_self =
                            Selector::Both(Box::new(self.clone()), Box::new(Selector::Placeheld));
                    }
                    _ => panic!("{:?}", t),
                },
                _ => panic!(),
            },
            Selector::Child(l, r) => {
                let new_l = l.clone();
                let mut new_r = r.clone();
                new_r.append(component);
                new_self = Selector::Child(new_l, new_r);
            }
            Selector::NextSibling(l, r) => {
                let new_l = l.clone();
                let mut new_r = r.clone();
                new_r.append(component);
                new_self = Selector::NextSibling(new_l, new_r);
            }
            Selector::Both(l, r) => {
                let new_l = l.clone();
                let mut new_r = r.clone();
                new_r.append(component);
                new_self = Selector::Both(new_l, new_r);
            }
            Selector::Universal => {
                new_self = self.clone();
            } //probably just the wind...
            _ => panic!(),
        }
        *self = new_self;
    }
}

#[derive(Default, Debug, Clone)]
pub struct DeclarationBuilder {
    kind: String,
    value: Vec<Component>,
    level: StyleLevel,
}

impl DeclarationBuilder {
    pub fn from_kind(kind: String) -> Self {
        Self {
            kind,
            value: vec![],
            level: StyleLevel::default(),
        }
    }

    pub fn set_kind(&mut self, kind: String) {
        self.kind = kind;
    }

    pub fn push_value(&mut self, shmeep: Component) {
        self.value.push(shmeep);
    }

    pub fn set_level(&mut self, level: StyleLevel) {
        self.level = level;
    }

    pub fn build(self) -> Result<Declaration, CSSError> {
        //TODO: So much
        let kind = match self.kind.as_str() {
            "color" => DeclarationKind::Color(Colour::from_components(self.value)),
            "display" => DeclarationKind::Display(Display::from_components(self.value)),
            "font-size" => DeclarationKind::FontSize(FontSize::from_components(self.value)),
            "font-weight" => DeclarationKind::FontWeight(FontWeight::from_components(self.value)),
            "text-align" => DeclarationKind::TextAlign(TextAlign::from_components(self.value)),
            "background-color" => {
                DeclarationKind::BackgroundColor(Colour::from_components(self.value))
            }
            "width" => DeclarationKind::Width(Dimensionality::from_components(self.value)),
            "height" => DeclarationKind::Height(Dimensionality::from_components(self.value)),
            "padding-top" => {
                DeclarationKind::PaddingTop(Dimensionality::from_components(self.value))
            }
            "padding-bottom" => {
                DeclarationKind::PaddingBottom(Dimensionality::from_components(self.value))
            }
            "padding-left" => {
                DeclarationKind::PaddingLeft(Dimensionality::from_components(self.value))
            }
            "padding-right" => {
                DeclarationKind::PaddingRight(Dimensionality::from_components(self.value))
            }
            "margin-top" => DeclarationKind::MarginTop(Dimensionality::from_components(self.value)),
            "margin-bottom" => {
                DeclarationKind::MarginBottom(Dimensionality::from_components(self.value))
            }
            "margin-left" => {
                DeclarationKind::MarginLeft(Dimensionality::from_components(self.value))
            }
            "margin-right" => {
                DeclarationKind::MarginRight(Dimensionality::from_components(self.value))
            }
            "font-family" => DeclarationKind::FontFamily(FontFamily::from_components(self.value)),
            _ => DeclarationKind::Unknown(self.kind, self.value),
        };
        Ok(Declaration {
            important: false,
            kind,
            level: self.level,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Declaration {
    important: bool,
    kind: DeclarationKind,
    level: StyleLevel,
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        return self.important == other.important
            && approx_eq::<DeclarationKind>(&self.kind, &other.kind)
            && approx_eq::<StyleLevel>(&self.level, &other.level);
    }
}

impl PartialOrd for Declaration {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            match (self.important, other.important, self.level, other.level) {
                (_, _, StyleLevel::UserAgent, StyleLevel::UserAgent) => Ordering::Equal,
                (_, _, StyleLevel::User, StyleLevel::User) => Ordering::Equal,
                (_, _, StyleLevel::Author, StyleLevel::Author) => Ordering::Equal,
                (true, false, _, _) => Ordering::Greater,
                (false, true, _, _) => Ordering::Less,
                (true, true, StyleLevel::UserAgent, StyleLevel::User) => Ordering::Greater,
                (true, true, StyleLevel::User, StyleLevel::UserAgent) => Ordering::Less,
                (true, true, StyleLevel::UserAgent, StyleLevel::Author) => Ordering::Greater,
                (true, true, StyleLevel::Author, StyleLevel::UserAgent) => Ordering::Less,
                (true, true, StyleLevel::User, StyleLevel::Author) => Ordering::Greater,
                (true, true, StyleLevel::Author, StyleLevel::User) => Ordering::Less,
                (false, false, StyleLevel::Author, StyleLevel::User) => Ordering::Greater,
                (false, false, StyleLevel::User, StyleLevel::Author) => Ordering::Greater,
                (false, false, StyleLevel::Author, StyleLevel::UserAgent) => Ordering::Greater,
                (false, false, StyleLevel::UserAgent, StyleLevel::Author) => Ordering::Less,
                (false, false, StyleLevel::User, StyleLevel::UserAgent) => Ordering::Greater,
                (false, false, StyleLevel::UserAgent, StyleLevel::User) => Ordering::Less,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub enum DeclarationKind {
    Unknown(String, Vec<Component>),
    Color(CSSValue<Colour>), // as much as i'd like to use the right spelling of colour here, it
    // should be this way to be idiomatic.
    Display(CSSValue<Display>),
    FontSize(CSSValue<FontSize>),
    FontWeight(CSSValue<FontWeight>),
    TextAlign(CSSValue<TextAlign>),
    BackgroundColor(CSSValue<Colour>),
    Width(CSSValue<Dimensionality>),
    Height(CSSValue<Dimensionality>),
    PaddingTop(CSSValue<Dimensionality>),
    PaddingBottom(CSSValue<Dimensionality>),
    PaddingLeft(CSSValue<Dimensionality>),
    PaddingRight(CSSValue<Dimensionality>),
    MarginTop(CSSValue<Dimensionality>),
    MarginBottom(CSSValue<Dimensionality>),
    MarginLeft(CSSValue<Dimensionality>),
    MarginRight(CSSValue<Dimensionality>),
    FontFamily(CSSValue<FontFamily>),
}

#[derive(Default, Debug, Clone)]
pub struct CSSProps {
    pub color: CSSValue<Colour>,
    pub display: CSSValue<Display>,
    pub font_size: CSSValue<FontSize>,
    pub font_weight: CSSValue<FontWeight>,
    pub text_align: CSSValue<TextAlign>,
    pub background_color: CSSValue<Colour>,
    pub width: CSSValue<Dimensionality>,
    pub height: CSSValue<Dimensionality>,
    pub padding_top: CSSValue<Dimensionality>,
    pub padding_bottom: CSSValue<Dimensionality>,
    pub padding_left: CSSValue<Dimensionality>,
    pub padding_right: CSSValue<Dimensionality>,
    pub margin_top: CSSValue<Dimensionality>,
    pub margin_bottom: CSSValue<Dimensionality>,
    pub margin_left: CSSValue<Dimensionality>,
    pub margin_right: CSSValue<Dimensionality>,
    pub font_family: CSSValue<FontFamily>,
}

impl CSSProps {
    pub fn val_for_variable_tag(&self, tag: Tag) -> Option<f64> {
        if Tag::new_checked(&[b'w', b'g', b'h', b't']).unwrap() == tag {
            return if let FontWeight::Absolute(f) = self.font_weight.unwrap() {
                Some(f)
            } else {
                Some(400.)
            };
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum Component {
    Block(Block),
    Function(),
    Token(CSSToken),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CSSToken {
    Whitespace,
    Delim(Char),
    Ident(String),
    Colon,
    Semicolon,
    CurlyOpen,
    CurlyClose,
    Comma,
    Number(CSSNumber),
    EOF,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
pub enum CSSNumber {
    Number(Numeric),
    Percentage(Numeric),
    Unit(Numeric, Unit),
}

impl CSSNumber {
    pub fn unwrap(&self) -> Numeric {
        match self {
            CSSNumber::Number(n) | CSSNumber::Unit(n, _) | CSSNumber::Percentage(n) => n.clone(),
        }
    }
}

impl Default for CSSNumber {
    fn default() -> Self {
        Self::Number(Numeric::Integer(0))
    }
}

#[derive(Debug, Default, Clone, PartialEq, PartialOrd, Copy)]
pub enum Unit {
    #[default]
    Px,
    Cm,
    Mm,
    In,
    Pt,
    Pc,
    Em,
    Ex,
    Ch,
    Rem,
    Vw,
    Vh,
    Vmin,
    Vmax,
}

impl Unit {
    pub fn from_string(string: String) -> Option<Self> {
        Some(match string.to_ascii_lowercase().as_str() {
            "px" => Unit::Px,
            "cm" => Unit::Cm,
            "mm" => Unit::Mm,
            "in" => Unit::In,
            "pt" => Unit::Pt,
            "pc" => Unit::Pc,
            "em" => Unit::Em,
            "ex" => Unit::Ex,
            "ch" => Unit::Ch,
            "rem" => Unit::Rem,
            "vw" => Unit::Vw,
            "vh" => Unit::Vh,
            "vmin" => Unit::Vmin,
            "vmax" => Unit::Vmax,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub enum CSSSource {
    Raw(String),
    URL(Url),
    Local(PathBuf),
    Pretokenized(Vec<CSSToken>),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
pub enum Numeric {
    Integer(i32),
    Number(f32),
}

impl Numeric {
    pub fn unwrap_f64(&self) -> f64 {
        // will continue implementing these as i need them.
        match self {
            Self::Integer(i) => *i as f64,
            Self::Number(f) => *f as f64,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct NumberRep {
    sign: Sign,
    e_sign: Sign,
    integer_part: String,
    decimal_part: String,
    exponent_part: String,
}

impl NumberRep {
    pub fn set_sign(&mut self, char: char) {
        self.sign = Sign::from_char(char);
    }

    pub fn set_e_sign(&mut self, char: char) {
	self.sign = Sign::from_char(char);
    }

    pub fn append_to_integer(&mut self, char: char) {
        self.integer_part.push(char);
    }

    pub fn append_to_decimal(&mut self, char: char) {
        self.decimal_part.push(char);
    }

    pub fn append_to_exponent(&mut self, char: char) {
        self.exponent_part.push(char);
    }

    pub fn into_numeric(&self) -> Result<Numeric, CSSError> {
        let sign = match self.sign {
            Sign::Plus => 1,
            Sign::Minus => -1,
        };
        if self.decimal_part.is_empty() && self.exponent_part.is_empty() {
            Ok(Numeric::Integer(
                sign * str::parse::<i32>(&self.integer_part).unwrap_or(0),
            ))
        } else {
	    let int = str::parse::<f32>(&self.integer_part).unwrap_or(0.);
	    let frac = str::parse::<f32>(&self.decimal_part).unwrap_or(0.);
	    let e_sign = match self.e_sign {
		Sign::Plus => 1.,
		Sign::Minus => -1.,
	    };
	    let exp = str::parse::<f32>(&self.exponent_part).unwrap_or(0.);
            Ok(Numeric::Number(
		sign as f32 * (int + (frac * (10_f32).powi(-1 * self.decimal_part.len() as i32))) * (10_f32).powf(e_sign * exp)))
        }
    }
}

#[derive(Default, Debug, Clone)]
pub enum Sign {
    #[default]
    Plus,
    Minus,
}

impl Sign {
    pub fn from_char(c: char) -> Self {
        if c == '-' {
            return Self::Minus;
        }
        return Self::Plus;
    }
}

#[derive(Debug, Default, Clone)]
pub enum CSSValue<T: Property + Default + Clone> {
    #[default]
    Inherit,
    Initial,
    Value(T),
}

impl<T: Property + Default + Clone> CSSValue<T> {
    pub fn unwrap(&self) -> T {
        if let Self::Value(t) = self {
            t.clone()
        } else {
            T::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess() {
	let input = "\u{000D}shmrmep\u{000C}shmrospo\u{000D}\u{000A}\u{0000}";
	let mut tokenizer = CSSTokenizer::default();
	tokenizer.load_raw(&String::from(input));
	tokenizer.preprocess();
	assert_eq!(tokenizer.source, String::from("\u{000A}shmrmep\u{000A}shmrospo\u{000A}\u{FFFD}"));
    }

    #[test]
    fn test_tokenize_ident() {
	let input = "_shmeep_shmOp_SHMORP";
	let mut tokenizer = CSSTokenizer::default();
	tokenizer.load_raw(&String::from(input));
	let mut result = Vec::new();
	tokenizer.tokenize(&mut result).unwrap();
	assert_eq!(result.as_slice(), &[CSSToken::Ident(String::from("_shmeep_shmOp_SHMORP")), CSSToken::EOF]);
    }

    #[test]
    fn test_tokenize_integers() {
	let input = "543 -543 +543 543% 543px";
	let mut tokenizer = CSSTokenizer::default();
	tokenizer.load_raw(&String::from(input));
	let mut result = Vec::new();
	tokenizer.tokenize(&mut result).unwrap();
	assert_eq!(result.as_slice(), &[CSSToken::Number(CSSNumber::Number(Numeric::Integer(543))), CSSToken::Whitespace,
				     CSSToken::Number(CSSNumber::Number(Numeric::Integer(-543))), CSSToken::Whitespace,
				     CSSToken::Number(CSSNumber::Number(Numeric::Integer(543))), CSSToken:: Whitespace,
				     CSSToken::Number(CSSNumber::Percentage(Numeric::Integer(543))), CSSToken::Whitespace,
				     CSSToken::Number(CSSNumber::Unit(Numeric::Integer(543), Unit::Px)),
				     CSSToken::EOF]);
    }

    #[test]
    fn test_tokenize_floats() {
	let input = "3.2 +3.2 -3.2 3e2 +3e2 -3e2 .3";
	let mut tokenizer = CSSTokenizer::default();
	tokenizer.load_raw(&String::from(input));
	let mut result = Vec::new();
	tokenizer.tokenize(&mut result).unwrap();
	assert_eq!(result.as_slice(),
		  &[CSSToken::Number(CSSNumber::Number(Numeric::Number(3.2))), CSSToken::Whitespace,
		    CSSToken::Number(CSSNumber::Number(Numeric::Number(3.2))), CSSToken::Whitespace,
		    CSSToken::Number(CSSNumber::Number(Numeric::Number(-3.2))), CSSToken::Whitespace,
		    CSSToken::Number(CSSNumber::Number(Numeric::Number(300.))), CSSToken::Whitespace,
		    CSSToken::Number(CSSNumber::Number(Numeric::Number(300.))), CSSToken::Whitespace,
		    CSSToken::Number(CSSNumber::Number(Numeric::Number(-300.))), CSSToken::Whitespace,
		    CSSToken::Number(CSSNumber::Number(Numeric::Number(0.3))), CSSToken::EOF,]);
    }
}
