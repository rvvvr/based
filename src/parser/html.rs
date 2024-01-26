use std::{str::Chars, path::PathBuf, fs::File, io::Read, collections::VecDeque, string::ParseError};

use thiserror::Error;
use derivative::Derivative;

use crate::{dom::{Document, DOMCoordinate, DOMElement, Node}, function};
use super::{Char, css::CSSParser};
//TODO: Better errors!
#[derive(Derivative, Debug)]
#[derivative(Default(new="true"))]
pub struct HTMLParser {
    nesting_level: usize,
    pause: bool,
    insertion_mode: InsertionMode,
    insertion_mode_origin: InsertionMode,
    using_rules_of: Option<InsertionMode>,
    source: String,
    source_idx: usize,
    open_elements: Vec<OpenElement>,
    scripting_enabled: bool,
    tokenization_state: TokenizationState,
    tokenization_state_origin: TokenizationState,
    current_token: Token,
    emit_buffer: VecDeque<Token>,
    tokens_available: bool,
    document: Document,
    head_pointer: Option<DOMCoordinate>,
    temp_buffer: String,
    last_start_tag: String,
    #[derivative(Default(value = "true"))]
    frameset_ok: bool,
    active_formatting_elements: Vec<DOMCoordinate>,
    done_parsing: bool,
    parsing_errors: Vec<(usize, ParsingError)>,
    css_parser: CSSParser,
}

impl HTMLParser {
    fn normalize_source(&mut self) -> Result<(), ParserError> {
        self.source = self.source.replace("\u{000D}\u{000A}", "\u{000A}")
                    .replace("\u{000D}", "\u{000A}");
        Ok(())
    }

    pub fn parse(&mut self) -> Result<&Vec<(usize, ParsingError)>, ParserError>{
        self.normalize_source()?;
        loop {
            if self.done_parsing {
                self.css_parser.push_many(self.document.find_css_sources());
                self.document.add_styles(self.css_parser.parse_stylesheets().unwrap());
                break Ok(&self.parsing_errors);
            }
            if let Err(e) = self.tokenize() {
                if let ParserError::ParsingError(err) = e {
                    self.parsing_errors.push((self.source_idx, err))
                } else {
                    do yeet e;
                }
            }
            if let Err(e) = self.handle_tokens() {
                if let ParserError::ParsingError(err) = e {
                    self.parsing_errors.push((self.source_idx, err))
                } else {
                    do yeet e;
                }
            }
        }
    }

    fn tokenize(&mut self) -> Result<(), ParserError> {
        loop {
            let result = match &self.tokenization_state {
                TokenizationState::Data => {
                    self.tokenize_data()?
                },
                TokenizationState::TagOpen => {
                    self.tokenize_tag_open()?
                },
                TokenizationState::EndTagOpen => {
                    self.tokenize_end_tag_open()?
                },
                TokenizationState::TagName => {
                    self.tokenize_tag_name()?
                }
                TokenizationState::MarkupDeclarationOpen => {
                    self.tokenize_markup_declaration_open()?
                },
                TokenizationState::DOCTYPE => {
                    self.tokenize_doctype()?
                },
                TokenizationState::BeforeDOCTYPEName => {
                    self.tokenize_before_doctype_name()?
                },
                TokenizationState::DOCTYPEName => {
                    self.tokenize_doctype_name()?
                },
                TokenizationState::AfterDOCTYPEName => {
                    self.tokenize_after_doctype_name()?
                },
                TokenizationState::BogusDOCTYPE => {
                    self.tokenize_bogus_doctype()?
                },
                TokenizationState::CommentStart => {
                    self.tokenize_comment_start()?
                },
                TokenizationState::Comment => { 
                    self.tokenize_comment()?
                },
                TokenizationState::CommentEndDash => {
                    self.tokenize_comment_end_dash()?
                },
                TokenizationState::CommentEnd => {
                    self.tokenize_comment_end()?
                },
                TokenizationState::RAWTEXT => {
                    self.tokenize_rawtext()?
                },
                TokenizationState::RAWTEXTLessThanSign => {
                    self.tokenize_rawtext_less_than_sign()?
                },
                TokenizationState::RAWTEXTEndTagOpen => {
                    self.tokenize_rawtext_end_tag_open()?
                },
                TokenizationState::RAWTEXTEndTagName => {
                    self.tokenize_rawtext_end_tag_name()?
                },
                TokenizationState::RCDATA => {
                    self.tokenize_rcdata()?
                },
                TokenizationState::RCDATALessThanSign => {
                    self.tokenize_rcdata_less_than_sign()?
                },
                TokenizationState::RCDATAEndTagOpen => {
                    self.tokenize_rcdata_end_tag_open()?
                },
                TokenizationState::RCDATAEndTagName => {
                    self.tokenize_rcdata_end_tag_name()?
                },
                TokenizationState::BeforeAttributeName => {
                    self.tokenize_before_attribute_name()?
                },
                TokenizationState::AttributeName => {
                    self.tokenize_attribute_name()?
                },
                TokenizationState::AfterAttributeName => {
                    self.tokenize_after_attribute_name()?
                },
                TokenizationState::BeforeAttributeValue => {
                    self.tokenize_before_attribute_value()?
                },
                TokenizationState::AttributeValueDoubleQuoted => {
                    self.tokenize_attribute_value_double_quoted()?
                },
                TokenizationState::AfterAttributeValueQuoted => {
                    self.tokenize_after_attribute_value_quoted()?
                }
                a => {
                    do yeet ParserError::UnimplementedTokenizationState(*a);
                },
            };
            if self.tokens_available {
                return Ok(());
            }
        }
    }

    fn current_element(&self) -> Option<OpenElement> {
        self.open_elements.last().cloned()
    }

    fn emit(&mut self, token: Token) -> Result<(), ParserError> {
        self.tokens_available = true;
        if let Token::StartTag { ref name, ..} = token {
            self.last_start_tag = name.to_string();
        } 
        self.emit_buffer.push_back(token);
        Ok(())
    }

    fn emit_current(&mut self) -> Result<(), ParserError> {
        self.tokens_available = true;
        if let Token::StartTag { ref name, .. } = self.current_token {
            self.last_start_tag = name.to_string();
        }
        self.emit_buffer.push_back(self.current_token.clone());
        Ok(())
    }

    fn emit_temp_buffer(&mut self) -> Result<(), ParserError> {
        let mut tokens = vec![];
        for char in self.temp_buffer.chars() {
            tokens.push(Token::Character { char });
        }
        self.emit_buffer.extend(tokens);
        self.tokens_available = true;
        Ok(())
    }

    fn reprocess_token(&mut self, token: Token, new_mode: InsertionMode) -> Result<(), ParserError> {
        self.emit_buffer.push_front(token);
        self.insertion_mode = new_mode;
        self.tokens_available = true;
        Ok(())
    }

    fn handle_tokens(&mut self) -> Result<(), ParserError> {
        self.tokens_available = false;
        println!("{:#?}", self);
        while let Some(token) = self.emit_buffer.pop_front() {
            match &self.insertion_mode {
                InsertionMode::Initial => {
                    self.handle_token_for_initial(token)?
                },
                InsertionMode::BeforeHtml => {
                    self.handle_token_for_before_html(token)?
                },
                InsertionMode::BeforeHead => {
                    self.handle_token_for_before_head(token)?
                },
                InsertionMode::InHead => {
                    self.handle_token_for_in_head(token)?
                },
                InsertionMode::AfterHead => {
                    self.handle_token_for_after_head(token)?
                },
                InsertionMode::InBody => {
                    self.handle_token_for_in_body(token)?
                },
                InsertionMode::Text => {
                    self.handle_token_for_text(token)?
                },
                InsertionMode::AfterBody => {
                    self.handle_token_for_after_body(token)?
                },
                InsertionMode::AfterAfterBody => {
                    self.handle_token_for_after_after_body(token)?
                },
                a => {
                    do yeet ParserError::UnimplementedInsertionMode(*a);
                },
            }
        };
        Ok(())
    }

    fn handle_token_for_initial(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Doctype { name, public_id, system_id, force_quirks } => {
                self.document.insert_document_type(name, system_id, public_id, force_quirks);
                self.insertion_mode = InsertionMode::BeforeHtml;
            },
            Token::Comment { data } => {
                self.document.insert_comment(data);
            }
            a => {
                self.reprocess_token(a, InsertionMode::BeforeHtml)?;
            }
        }
        Ok(())
    }

    fn handle_token_for_before_html(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {},
            Token::Comment { data } => {
                self.document.insert_comment(data);
            },
            Token::StartTag { name, attributes } if name == "html" => {
                let coordinate = self.document.insert_element(name, attributes);
                self.open_elements.push(OpenElement { coordinate });
                self.insertion_mode = InsertionMode::BeforeHead;
            }
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode);
            }
        }
        Ok(())
    }

    fn handle_token_for_before_head(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {},
            Token::StartTag { name, attributes } if name == "head" => {
                let coordinate = self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate).insert_element(name, attributes);
                self.open_elements.push(OpenElement { coordinate: coordinate.clone() });
                self.head_pointer = Some(coordinate);
                self.insertion_mode = InsertionMode::InHead;
            }
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode);
            } 
        }
        Ok(())
    }

    fn handle_token_for_in_head(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {},
            Token::StartTag { ref name, .. } if name == "title" => {
                self.generic_parsing_algorithm(token, false)?;
            },
            Token::StartTag { ref name, .. } if name == "style" || name == "noframes" => {
                self.generic_parsing_algorithm(token, true)?;
            },
            Token::EndTag { ref name } if name == "head" => {
                let _ = self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
            }
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode);
            }
        }
        Ok(())
    }

    fn handle_token_for_after_head(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {
                self.insert_character(char)?;
            },
            Token::StartTag { name, attributes } if name == "body" => {
                let coordinate = self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate).insert_element(name, attributes);
                self.open_elements.push(OpenElement { coordinate });
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
            },
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode)
            }
        }
        Ok(())
    }

    fn handle_token_for_in_body(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if char == '\u{0000}' => {},
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {
                if !self.active_formatting_elements.is_empty() {
                    todo!("Reconstruct active formatting elements!");
                }
                self.insert_character(char)?;
            },
            Token::Character { char } => {
                if !self.active_formatting_elements.is_empty() {
                    todo!("Reconstruct active formatting elements!");
                }
                self.insert_character(char)?;
            },
            Token::StartTag { name, attributes } if name == "p" => {
                let coordinate = self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate).insert_element(name, attributes);
                self.open_elements.push(OpenElement { coordinate });
            },
            Token::StartTag { name, attributes } if name == "h1" || name == "h2" || name == "h3" || name == "h4" || name == "h5" || name == "h6" => {
                //TODO: check for p in button scope
                //TODO: also check if current element is h1..=6
                let coordinate = self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate).insert_element(name, attributes);
                self.open_elements.push(OpenElement { coordinate });
            },
            Token::EndTag { name } if name == "h1" || name == "h2" || name == "h3" || name == "h4" || name == "h5" || name == "h6" => {
                //TODO: Generate implied end tags
                //TODO: Check for element in scope
                while let Some(element) = self.open_elements.pop() {
                    let element_name = &self.document.get_element_for_coordinate(element.coordinate).tag_name;
                    if element_name == &name {
                        break;
                    }
                }
            },
            Token::EndTag { name } if name == "p" => {
                while let Some(element) = self.open_elements.pop() {
                    let element_name = &self.document.get_element_for_coordinate(element.coordinate).tag_name;
                    if element_name == &name {
                        break;
                    }
                }
            },
            Token::EndTag { name } if name == "body" => {
                //TODO: Check for body tag in scope
                //TODO: Check if one of those other trillion elements are in scope
                self.insertion_mode = InsertionMode::AfterBody;
            },
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode)
            },
        }
        Ok(())
    }

    fn handle_token_for_text(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } => {
                self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate).data.push(char);
            },
            Token::EndTag { name } if name == "script" => {
                todo!();
            },
            Token::EndTag { name } => {
                self.open_elements.pop();
                self.insertion_mode = self.insertion_mode_origin;
            }
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode);
            }
        }
        Ok(())
    }

    fn handle_token_for_after_body(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {
                self.handle_token_for_in_body(token)?
            },
            Token::EndTag { name } if name == "html" => {
                //TODO: Something to do with fragment parsing.
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode);
            },
        }
        Ok(())
    }

    fn handle_token_for_after_after_body(&mut self, token: Token) -> Result<(), ParserError> {
        match token {
            Token::Character { char } if (char == '\u{0009}') || (char == '\u{000A}') || (char == '\u{000C}') || (char == '\u{000D}') || (char == '\u{0020}') => {
                self.handle_token_for_in_body(token)?
            },
            Token::EOF => {
                self.done_parsing = true;
            },
            a => {
                do yeet ParserError::UnhandledTokenForInsertionMode(a, self.insertion_mode);
            },
        }
        Ok(())
    }

    fn generic_parsing_algorithm(&mut self, token: Token, raw_text: bool) -> Result<(), ParserError> {
        if let Token::StartTag { name, attributes } = token {
            let coordinate = self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate).insert_element(name, attributes);
            self.open_elements.push(OpenElement { coordinate: coordinate.clone() });
            self.tokenization_state = if raw_text {
                TokenizationState::RAWTEXT
            } else {
                TokenizationState::RCDATA
            };
            self.insertion_mode_origin = self.insertion_mode;
            self.insertion_mode = InsertionMode::Text;
            Ok(())
        } else {
            do yeet ParserError::CurrentTokenWrongType(function!());
        }
    }

    fn consume(&mut self) -> Char {
        if let Some(char) = self.source.chars().nth(self.source_idx) {
            self.source_idx += 1;
            Char::Char(char)
        } else {
            Char::Eof
        }
    }

    fn reconsume(&mut self, next_state: TokenizationState) {
        self.source_idx = self.source_idx.saturating_sub(1);
        self.tokenization_state = next_state;
    }

    fn insert_character(&mut self, c: char) -> Result<(), ParserError> {
        let current_node = self.document.get_element_for_coordinate(self.current_element().unwrap().coordinate);
        if ["table", "tbody", "tfoot", "thead", "tr"].contains(&current_node.tag_name.as_str()) {
            todo!("Foster parenting")
        }
        let last_child = current_node.children.last_mut();
        if !last_child.is_some_and(|node| {
            if let Node::Text(ref mut internal) = node {
               internal.push(c); 
               return true;
            }
            return false;
        }) {
            current_node.children.push(Node::Text(String::from(c)));
        }
        Ok(())
    }

    pub fn load_from_file(&mut self, path: PathBuf) -> Result<(), ParserError> {
        File::open(path).unwrap().read_to_string(&mut self.source);
        Ok(())
    }

    fn tokenize_data(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('<') => {
                self.tokenization_state = TokenizationState::TagOpen;
            },
            Char::Char(c) => {
                self.emit(Token::Character { char: c })?;
            },
            Char::Eof => {
                self.emit(Token::EOF)?;
            },
        };
        Ok(())
    }

    fn tokenize_tag_open(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('!') => {
                self.tokenization_state = TokenizationState::MarkupDeclarationOpen;
            },
            Char::Char('/') => {
                self.tokenization_state = TokenizationState::EndTagOpen;
            },
            Char::Char('a'..='z' | 'A'..='Z') => {
                self.current_token = Token::StartTag { name: String::new(), attributes: vec![] };
                self.reconsume(TokenizationState::TagName);
            },
            Char::Char(c) => {
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Char(c), self.tokenization_state);
            },
            Char::Eof => {
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Eof, self.tokenization_state);
            },
        };
        Ok(())
    }

    fn tokenize_end_tag_open(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char(c) if ('a'..='z').contains(&c) || ('A'..='Z').contains(&c) => {
                self.current_token = Token::EndTag { name: String::new() };
                self.reconsume(TokenizationState::TagName);
            },
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                do yeet ParsingError::MissingEndTagName;
            },
            Char::Char(_) => {
                self.current_token = Token::Comment { data: String::new() };
                self.reconsume(TokenizationState::BogusComment);
                do yeet ParsingError::EofBeforeTagName;
            },
            Char::Eof => {
                self.emit(Token::Character { char: '<' })?;
                self.emit(Token::Character { char: '/' })?;
                self.emit(Token::EOF)?;
            },
        }
        Ok(())
    }

    fn tokenize_tag_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
                self.tokenization_state = TokenizationState::BeforeAttributeName;
            },
            Char::Char('/') => {
                self.tokenization_state = TokenizationState::SelfClosingStartTag;
            },
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                self.emit_current()?;
            },
            Char::Char(c) if ('A'..='Z').contains(&c) => {
                if let Token::StartTag { ref mut name, .. } = self.current_token {
                    name.push(char::from_u32(c as u32 + 0x20).unwrap());
                } else if let Token::EndTag { ref mut name } = self.current_token {
                    name.push(char::from_u32(c as u32 + 0x20).unwrap());
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char('\u{0000}') => {
                if let Token::StartTag { ref mut name, .. } = self.current_token {
                    name.push('\u{FFFD}');
                } else if let Token::EndTag { ref mut name } = self.current_token {
                    name.push('\u{FFFD}');
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char(c) => {
                if let Token::StartTag { ref mut name, .. } = self.current_token {
                    name.push(c);
                } else if let Token::EndTag { ref mut name } = self.current_token {
                    name.push(c);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            }
            Char::Eof => {
                self.emit(Token::EOF)?;
                do yeet ParserError::ParsingError(ParsingError::EofInTag);
            },
        }
        Ok(())
    }

    fn tokenize_markup_declaration_open(&mut self) -> Result<(), ParserError> {
        if &self.source[self.source_idx..self.source_idx + 2] == "--" {
            self.source_idx += 2;
            self.tokenization_state = TokenizationState::CommentStart;
            self.current_token = Token::Comment { data: String::new() };
        } else if &self.source[self.source_idx..self.source_idx + 7] == "DOCTYPE" {
            self.source_idx += 7;
            self.tokenization_state = TokenizationState::DOCTYPE;
        } else { 
            self.current_token = Token::Comment { data: String::new() };
            self.tokenization_state = TokenizationState::BogusComment;
            do yeet ParserError::ParsingError(ParsingError::IncorrectlyOpenedComment);
        };
        Ok(())
    }

    fn tokenize_doctype(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
                self.tokenization_state = TokenizationState::BeforeDOCTYPEName;
            },
            Char::Char(c) => {
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Char(c), self.tokenization_state);
            },
            Char::Eof => {
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Eof, self.tokenization_state);
            },
        }
        Ok(())
    }

    fn tokenize_before_doctype_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
            },
            Char::Char(c) if ('\u{0041}'..='\u{005A}').contains(&c) => {
                self.current_token = Token::Doctype { name: String::from(char::from_u32(c as u32 + 0x20).unwrap()), public_id: String::new(), system_id: String::new(), force_quirks: false };
                self.tokenization_state = TokenizationState::DOCTYPEName;
            },
            Char::Char('\u{0000}') => {
                self.current_token = Token::Doctype { name: String::from("\u{FFFD}"), public_id: String::new(), system_id: String::new(), force_quirks: false };
                self.tokenization_state = TokenizationState::DOCTYPEName;
            },
            Char::Char('>') => {
                self.current_token = Token::Doctype { name: String::new(), public_id: String::new(), system_id: String::new(), force_quirks: true };
                self.tokenization_state = TokenizationState::Data;
                self.emit_current();
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Char('>'), self.tokenization_state);
            }
            Char::Char(c) => {
                self.current_token = Token::Doctype { name: String::from(c), public_id: String::new(), system_id: String::new(), force_quirks: false };
                self.tokenization_state = TokenizationState::DOCTYPEName;
            },
            Char::Eof => {
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Eof, self.tokenization_state);
            },
        };
        Ok(())
    }

    fn tokenize_doctype_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
                self.tokenization_state = TokenizationState::AfterDOCTYPEName;
            },
            Char::Char(c) if ('\u{0041}'..='\u{005A}').contains(&c) => {
                if let Token::Doctype { ref mut name, .. } = self.current_token { 
                    name.push(char::from_u32(c as u32 + 0x20).unwrap());
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                self.emit_current()?;
            }
            Char::Char('\u{0000}') => {
                if let Token::Doctype { ref mut name, .. } = self.current_token {
                    name.push('\u{FFFD}');
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char(c) => {
                if let Token::Doctype { ref mut name, ..} = self.current_token {
                    name.push(c);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Eof => {
                do yeet ParserError::UnhandledCharForTokenizationState(Char::Eof, self.tokenization_state);
            },
        };
        Ok(())
    }

    fn tokenize_after_doctype_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {},
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                self.emit_current()?;
            },
            Char::Char(_) => {
                //TODO: Public and system identifiers
                if let Token::Doctype { ref mut force_quirks, .. } = &mut self.current_token {
                    *force_quirks = true;
                    self.reconsume(TokenizationState::BogusDOCTYPE);
                    do yeet ParsingError::InvalidCharacterSequenceAfterDoctypeName;
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Eof => {
                if let Token::Doctype { ref mut force_quirks, .. } = &mut self.current_token {
                    *force_quirks = true;
                    self.emit_current()?;
                    self.emit(Token::EOF)?;
                    do yeet ParsingError::EofInDoctype;
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
                
            }
        }
        Ok(())
    }

    fn tokenize_bogus_doctype(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                self.emit_current()?;
            },
            Char::Char('\u{0000}') => {
                do yeet ParsingError::UnexpectedNullCharacter;
            },
            Char::Eof => {
                self.emit_current()?;
                self.emit(Token::EOF)?;
            },
            Char::Char(_) => {},
        }
        Ok(()) 
    }

    fn tokenize_comment_start(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('-') => {
                self.tokenization_state = TokenizationState::CommentStartDash;
            },
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                self.emit_current()?;
                do yeet ParserError::ParsingError(ParsingError::AbruptClosingOfEmptyComment);
            },
            _ => {
                self.reconsume(TokenizationState::Comment);
            }
        }
        Ok(())
    }

    fn tokenize_comment(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('<') => {
                if let Token::Comment { ref mut data } = self.current_token {
                    data.push('<');
                    self.tokenization_state = TokenizationState::CommentLessThanSign;
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char('-') => {
                self.tokenization_state = TokenizationState::CommentEndDash;
            },
            Char::Char('\u{0000}') => {
                if let Token::Comment { ref mut data } = self.current_token {
                    data.push('\u{FFFD}');
                    do yeet ParserError::ParsingError(ParsingError::UnexpectedNullCharacter);
                }
            },
            Char::Char(c) => {
                if let Token::Comment { ref mut data } = self.current_token {
                    data.push(c);
                }
            },
            Char::Eof => {
                self.emit_current()?;
                self.emit(Token::EOF)?;
                do yeet ParserError::ParsingError(ParsingError::EofInComment);
            },
        }
        Ok(())
    }

    fn tokenize_comment_end_dash(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('-') => {
                self.tokenization_state = TokenizationState::CommentEnd;
            },
            Char::Char(_) => {
                if let Token::Comment { ref mut data } = self.current_token {
                    data.push('-');
                    self.reconsume(TokenizationState::Comment);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Eof => {
                self.emit_current()?;
                self.emit(Token::EOF)?;
                do yeet ParserError::ParsingError(ParsingError::EofInComment);
            },
        }
        Ok(())
    }

    fn tokenize_comment_end(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('>') => {
                self.emit_current()?;
                self.tokenization_state = TokenizationState::Data;
            },
            Char::Char('!') => {
                self.tokenization_state = TokenizationState::CommentEndBang;
            },
            Char::Char('-') => {
                if let Token::Comment { ref mut data } = self.current_token {
                    data.push('-');
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char(_) => {
                if let Token::Comment { ref mut data } = self.current_token {
                    data.push('-');
                    data.push('-');
                    self.reconsume(TokenizationState::Comment);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Eof => {
                self.emit_current()?;
                self.emit(Token::EOF)?;
                do yeet ParserError::ParsingError(ParsingError::EofInComment);
            },
        }
        Ok(())
    }

    fn tokenize_rawtext(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('<') => {
                self.tokenization_state = TokenizationState::RAWTEXTLessThanSign;
            },
            Char::Char('\u{0000}') => {
                self.emit(Token::Character { char: '\u{FFFD}' })?;
                do yeet ParsingError::UnexpectedNullCharacter;
            },
            Char::Char(c) => {
                self.emit(Token::Character { char: c })?;
            },
            Char::Eof => {
                self.emit(Token::EOF)?;
            }
        }
        Ok(())
    }

    fn tokenize_rawtext_less_than_sign(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('/') => {
                self.temp_buffer.clear();
                self.tokenization_state = TokenizationState::RAWTEXTEndTagOpen;
            },
            _ => {
                self.emit(Token::Character { char: '<' })?;
                self.reconsume(TokenizationState::RAWTEXT);
            }
        }
        Ok(())
    }

    fn tokenize_rawtext_end_tag_open(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('A'..='Z' | 'a'..='z') => {
                self.current_token = Token::EndTag { name: String::new() };
                self.reconsume(TokenizationState::RAWTEXTEndTagName);
            },
            _ => {
                self.emit(Token::Character { char: '<' })?;
                self.emit(Token::Character { char: '/' })?;
                self.reconsume(TokenizationState::RAWTEXT);
            },
        }
        Ok(())
    }

    fn tokenize_rawtext_end_tag_name(&mut self) -> Result<(), ParserError> {
        match self.consume() { 
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
                if self.current_tag_is_appropriate()? {
                    self.tokenization_state = TokenizationState::BeforeAttributeName;
                    return Ok(());
                }
            },
            Char::Char('/') => {
                if self.current_tag_is_appropriate()? { 
                    self.tokenization_state = TokenizationState::SelfClosingStartTag;
                    return Ok(());
                }
            },
            Char::Char('>') => {
                if self.current_tag_is_appropriate()? {
                    self.tokenization_state = TokenizationState::Data;
                    self.emit_current()?;
                    return Ok(())
                }
            },
            Char::Char(c) if ('A'..='Z').contains(&c) => {
                if let Token::EndTag { ref mut name } = self.current_token {
                    name.push(char::from_u32(c as u32 + 0x20).unwrap());
                    self.temp_buffer.push(c);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
                return Ok(());
            },
            Char::Char(c) if ('a'..='z').contains(&c) => {
                if let Token::EndTag { ref mut name } = self.current_token {
                    name.push(c);
                    self.temp_buffer.push(c);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
                return Ok(())
            },
            _ => {},
        };
        self.emit(Token::Character { char: '<' })?;
        self.emit(Token::Character { char: '/' })?;
        self.emit_temp_buffer()?;
        self.reconsume(TokenizationState::RAWTEXT);
        Ok(())
    }

    fn tokenize_rcdata(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('&') => {
                self.tokenization_state_origin = self.tokenization_state;
                self.tokenization_state = TokenizationState::CharacterReference;
            },
            Char::Char('<') => {
                self.tokenization_state = TokenizationState::RCDATALessThanSign;
            },
            Char::Char('\u{0000}') => {
                self.emit(Token::Character { char: '\u{FFFD}'})?;
                do yeet ParsingError::UnexpectedNullCharacter;
            }
            Char::Char(c) => { 
                self.emit(Token::Character { char: c })?;
            }
            Char::Eof => {
                self.emit(Token::EOF)?;
            }
        }
        Ok(())
    }

    fn tokenize_rcdata_less_than_sign(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('/') => {
                self.temp_buffer.clear();
                self.tokenization_state = TokenizationState::RCDATAEndTagOpen;
            },
            _ => {
                self.emit(Token::Character { char: '<' })?;
                self.reconsume(TokenizationState::RCDATA);
            }
        }
        Ok(())
    }
    
    fn tokenize_rcdata_end_tag_open(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('a'..='z' | 'A'..='Z') => {
                self.reconsume(TokenizationState::RCDATAEndTagName);
                self.current_token = Token::EndTag { name: String::new() };
            },
            _ => {
                self.emit(Token::Character { char: '<' })?;
                self.emit(Token::Character { char: '/' })?;
                self.reconsume(TokenizationState::RCDATA);
            }
        }
        Ok(())
    }
    
    fn tokenize_rcdata_end_tag_name(&mut self) -> Result<(), ParserError> {
        match self.consume() { 
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
                if self.current_tag_is_appropriate()? {
                    self.tokenization_state = TokenizationState::BeforeAttributeName;
                    return Ok(());
                }
            },
            Char::Char('/') => {
                if self.current_tag_is_appropriate()? { 
                    self.tokenization_state = TokenizationState::SelfClosingStartTag;
                    return Ok(());
                }
            },
            Char::Char('>') => {
                if self.current_tag_is_appropriate()? {
                    self.tokenization_state = TokenizationState::Data;
                    self.emit_current()?;
                    return Ok(())
                }
            },
            Char::Char(c) if ('A'..='Z').contains(&c) => {
                if let Token::EndTag { ref mut name } = self.current_token {
                    name.push(char::from_u32(c as u32 + 0x20).unwrap());
                    self.temp_buffer.push(c);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
                return Ok(());
            },
            Char::Char(c) if ('a'..='z').contains(&c) => {
                if let Token::EndTag { ref mut name } = self.current_token {
                    name.push(c);
                    self.temp_buffer.push(c);
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
                return Ok(())
            },
            _ => {},
        };
        self.emit(Token::Character { char: '<' })?;
        self.emit(Token::Character { char: '/' })?;
        self.emit_temp_buffer()?;
        self.reconsume(TokenizationState::RCDATA);
        Ok(())
    }

    fn tokenize_before_attribute_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {},
            Char::Eof | Char::Char('>' | '/') => {
                self.reconsume(TokenizationState::AfterAttributeName);
            },
            Char::Char(c) => {
                if let Token::StartTag { ref mut attributes, .. } = &mut self.current_token {
                    attributes.push((String::new(), String::new()));
                    self.reconsume(TokenizationState::AttributeName);
                }
            }
        }
        Ok(())
    }

    fn tokenize_attribute_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}' | '>' | '/') | Char::Eof => {
                self.reconsume(TokenizationState::AfterAttributeName);
            },
            Char::Char('=') => {
                self.tokenization_state = TokenizationState::BeforeAttributeValue;
            },
            Char::Char(c) if ('A'..='Z').contains(&c) => {
                if let Token::StartTag { name, ref mut attributes } = &mut self.current_token {
                    if let Some((ref mut name, ref value)) = attributes.last_mut() {
                        name.push(char::from_u32(c as u32 + 0x20).unwrap());
                    } else {
                        do yeet ParserError::CurrentTokenWrongType(function!());
                    }
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char('\u{0000}') => {
                if let Token::StartTag { name, ref mut attributes } = &mut self.current_token {
                    if let Some((ref mut name, ref value)) = attributes.last_mut() {
                        name.push('\u{FFFD}');
                    } else {
                        do yeet ParserError::CurrentTokenWrongType(function!());
                    }
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Char(c) => {
                if let Token::StartTag { name, ref mut attributes } = &mut self.current_token {
                    if let Some((ref mut name, ref value)) = attributes.last_mut() {
                        name.push(c);
                    } else {
                        do yeet ParserError::CurrentTokenWrongType(function!());
                    }
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            }
        }
        Ok(())
    }

    fn tokenize_after_attribute_name(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {},
            Char::Char('/') => {
                self.tokenization_state = TokenizationState::SelfClosingStartTag;
            },
            Char::Char('=') => {
                self.tokenization_state = TokenizationState::BeforeAttributeValue;
            },
            Char::Char('>') => {
                self.emit_current()?;
                self.tokenization_state = TokenizationState::Data;
            },
            Char::Char(c) => {
                if let Token::StartTag { name, ref mut attributes } = &mut self.current_token {
                    attributes.push((String::new(), String::new()));
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Eof => {
                self.emit(Token::EOF)?;
                do yeet ParsingError::EofInTag;
            }
        }
        Ok(())
    }

    fn tokenize_before_attribute_value(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {},
            Char::Char('"') => {
                self.tokenization_state = TokenizationState::AttributeValueDoubleQuoted;
            },
            Char::Char('\'') => {
                self.tokenization_state = TokenizationState::AttributeValueSingleQuoted;
            },
            Char::Char('>') => {
                self.emit_current()?;
                self.tokenization_state = TokenizationState::Data;
                do yeet ParsingError::MissingAttributeValue;
            },
            _ => {
                self.reconsume(TokenizationState::AttributeValueUnquoted);
            }
        }
        Ok(())
    }

    fn tokenize_attribute_value_double_quoted(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('"') => {
                self.tokenization_state = TokenizationState::AfterAttributeValueQuoted;
            },
            Char::Char('&') => {
                self.tokenization_state_origin = self.tokenization_state;
                self.tokenization_state = TokenizationState::CharacterReference;
            },
            Char::Char('\u{0000}') => {
                if let Token::StartTag { name, ref mut attributes } = &mut self.current_token {
                    if let Some((ref name, ref mut value)) = attributes.last_mut() {
                        value.push('\u{FFFD}');
                    } else {
                        do yeet ParserError::CurrentTokenWrongType(function!());
                    }
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            }
            Char::Char(c) => {
                if let Token::StartTag { name, ref mut attributes } = &mut self.current_token {
                    if let Some((ref name, ref mut value)) = attributes.last_mut() {
                        value.push(c);
                    } else {
                        do yeet ParserError::CurrentTokenWrongType(function!());
                    }
                } else {
                    do yeet ParserError::CurrentTokenWrongType(function!());
                }
            },
            Char::Eof => {
                self.emit(Token::EOF);
                do yeet ParsingError::EofInTag;
            },
        }
        Ok(())
    }

    fn tokenize_after_attribute_value_quoted(&mut self) -> Result<(), ParserError> {
        match self.consume() {
            Char::Char('\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{0020}') => {
                self.tokenization_state = TokenizationState::BeforeAttributeName;
            },
            Char::Char('/') => {
                self.tokenization_state = TokenizationState::SelfClosingStartTag;
            },
            Char::Char('>') => {
                self.tokenization_state = TokenizationState::Data;
                self.emit_current()?;
            },
            Char::Char(_) => {
                self.reconsume(TokenizationState::BeforeAttributeName);
                do yeet ParsingError::MissingWhitespaceBetweenAttributes;
            },
            Char::Eof => {
                self.emit(Token::EOF)?;
                do yeet ParsingError::EofInTag;
            }
        }
        Ok(())
    }

    fn current_tag_is_appropriate(&self) -> Result<bool, ParserError> {
        Ok(if let Token::EndTag { ref name } = self.current_token { name } else { do yeet ParserError::CurrentTokenWrongType(function!()) } == &self.last_start_tag)
    }
    
}

#[derive(Debug, Default, Clone)]
pub enum Token {
    #[default]
    EOF,
    Comment {
        data: String,
    },
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
        force_quirks: bool,
    },
    Character {
        char: char,
    },
    StartTag {
        name: String,
        attributes: Vec<(String, String)>
    },
    EndTag {
        name: String
    },
}


#[derive(Debug, Default, Clone, Copy)]
pub enum InsertionMode {
    #[default]
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum TokenizationState {
    #[default]
    Data,
    RCDATA,
    RAWTEXT,
    ScriptData,
    PLAINTEXT,
    TagOpen,
    EndTagOpen,
    TagName,
    RCDATALessThanSign,
    RCDATAEndTagOpen,
    RCDATAEndTagName,
    RAWTEXTLessThanSign,
    RAWTEXTEndTagOpen,
    RAWTEXTEndTagName,
    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    DOCTYPE,
    BeforeDOCTYPEName,
    DOCTYPEName,
    AfterDOCTYPEName,
    AfterDOCTYPEPublicKeyword,
    BeforeDOCTYPEPublicIdentifier,
    DOCTYPEPublicIdentifierDoubleQuoted,
    DOCTYPEPublicIdentifierSingleQuoted,
    AfterDOCTYPEPublicIdentifier,
    BetweenDOCTYPEPublicAndSystemIdentifiers,
    AfterDOCTYPESystemKeyword,
    BeforeDOCTYPESystemIdentifier,
    DOCTYPESystemIdentifierDoubleQuoted,
    DOCTYPESystemIdentifierSingleQuoted,
    AfterDOCTYPESystemIdentifier,
    BogusDOCTYPE,
    CDATASection,
    CDATASectionBracket,
    CDATASectionEnd,
    CharacterReference,
    NamedCharacterReference,
    AmbiguousAmpersand,
    NumericCharacterReference,
    HexadecimalCharacterReferenceStart,
    DecimalCharacterReferenceStart,
    HexadecimalCharacterReference,
    DecimalCharacterReference,
    NumericCharacterReferenceEnd,
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Normalizing newlines in the source failed!")]
    NormalizationFailed,
    #[error("Tokenization state {0:?} not implemented!")]
    UnimplementedTokenizationState(TokenizationState),
    #[error("Unhandled char {0:?} for tokenization state {1:?}")]
    UnhandledCharForTokenizationState(Char, TokenizationState),
    #[error("Internal parsing error!: {0}")]
    ParsingError(#[from] ParsingError),
    #[error("Current token is wrong type! {0}")]
    CurrentTokenWrongType(String),
    #[error("Insertion mode {0:?} not implemented!")]
    UnimplementedInsertionMode(InsertionMode),
    #[error("Unhandled token {0:?} for insertion mode {1:?}")]
    UnhandledTokenForInsertionMode(Token, InsertionMode),
}

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("AbruptClosingOfEmptyComment")]
    AbruptClosingOfEmptyComment,
    #[error("AbruptDoctypePublicIdentifier")]
    AbruptDoctypePublicIdentifier,
    #[error("AbruptDoctypeSystemIdentifier")]
    AbruptDoctypeSystemIdentifier,
    #[error("AbsenceOfDigitsInNumericCharacterReference")]
    AbsenceOfDigitsInNumericCharacterReference,
    #[error("CDATAInHtmlContent")]
    CDATAInHtmlContent,
    #[error("CharacterReferenceOutsideUnicodeRange")]
    CharacterReferenceOutsideUnicodeRange,
    #[error("ControlCharacterInInputStream")]
    ControlCharacterInInputStream,
    #[error("ControlCharacterReference")]
    ControlCharacterReference,
    #[error("DuplicateAttribute")]
    DuplicateAttribute,
    #[error("EndTagWithAttributes")]
    EndTagWithAttributes,
    #[error("EndTagWithTrailingSolidus")]
    EndTagWithTrailingSolidus,
    #[error("EofBeforeTagName")]
    EofBeforeTagName,
    #[error("EofInCDATA")]
    EofInCDATA,
    #[error("EofInComment")]
    EofInComment,
    #[error("EofInDoctype")]
    EofInDoctype,
    #[error("EofInScriptHtmlCommentLikeText")]
    EofInScriptHtmlCommentLikeText,
    #[error("EofInTag")]
    EofInTag,
    #[error("IncorrectlyClosedComment")]
    IncorrectlyClosedComment,
    #[error("IncorrectlyOpenedComment")]
    IncorrectlyOpenedComment,
    #[error("InvalidCharacterSequenceAfterDoctypeName")]
    InvalidCharacterSequenceAfterDoctypeName,
    #[error("InvalidFirstCharacterOfTagName")]
    InvalidFirstCharacterOfTagName,
    #[error("MissingAttributeValue")]
    MissingAttributeValue,
    #[error("MissingDoctypeName")]
    MissingDoctypeName,
    #[error("MissingDoctypePublicIdentifier")]
    MissingDoctypePublicIdentifier,
    #[error("MissingDoctypeSystemIdentifier")]
    MissingDoctypeSystemIdentifier,
    #[error("MissingEndTagName")]
    MissingEndTagName,
    #[error("MissingQuoteBeforeDoctypePublicIdentifier")]
    MissingQuoteBeforeDoctypePublicIdentifier,
    #[error("MissingQuoteBeforeDoctypeSystemIdentifier")]
    MissingQuoteBeforeDoctypeSystemIdentifier,
    #[error("MissingSemicolonAfterCharacterReference")]
    MissingSemicolonAfterCharacterReference,
    #[error("MissingWhitespaceAfterDoctypePublicKeyword")]
    MissingWhitespaceAfterDoctypePublicKeyword,
    #[error("MissingWhitespaceAfterDoctypeSystemKeyword")]
    MissingWhitespaceAfterDoctypeSystemKeyword,
    #[error("MissingWhitespaceBeforeDoctypeName")]
    MissingWhitespaceBeforeDoctypeName,
    #[error("MissingWhitespaceBetweenAttributes")]
    MissingWhitespaceBetweenAttributes,
    #[error("MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers")]
    MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
    #[error("NestedComment")]
    NestedComment,
    #[error("NonCharacterCharacterReference")]
    NonCharacterCharacterReference,
    #[error("NonCharacterInInputStream")]
    NonCharacterInInputStream,
    #[error("NonVoidHtmlElementStartTagWithTrailingSolidus")]
    NonVoidHtmlElementStartTagWithTrailingSolidus,
    #[error("NullCharacterReference")]
    NullCharacterReference,
    #[error("SurrogateCharacterReference")]
    SurrogateCharacterReference,
    #[error("SurrogateInInputStream")]
    SurrogateInInputStream,
    #[error("UnexpectedCharacterAfterDoctypeSystemIdentifier")]
    UnexpectedCharacterAfterDoctypeSystemIdentifier,
    #[error("UnexpectedCharacterInAttributeName")]
    UnexpectedCharacterInAttributeName,
    #[error("UnexpectedCharacterInUnquotedAttributeValue")]
    UnexpectedCharacterInUnquotedAttributeValue,
    #[error("UnexpectedEqualsSignBeforeAttributeName")]
    UnexpectedEqualsSignBeforeAttributeName,
    #[error("UnexpectedNullCharacter")]
    UnexpectedNullCharacter,
    #[error("UnexpectedQuestionMarkInsteadOfTagName")]
    UnexpectedQuestionMarkInsteadOfTagName,
    #[error("UnexpectedSoidusInTag")]
    UnexpectedSoidusInTag,
    #[error("UnknownNamedCharacterReference")]
    UnknownNamedCharacterReference,
}

#[derive(Default, Debug, Clone)]
struct OpenElement {
    coordinate: DOMCoordinate,
}
