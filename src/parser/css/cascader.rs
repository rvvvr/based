use crate::{dom::{Node, Element}, parser::css::{Selector, Rule}};

use super::{StyleData, Prelude, Declaration, DeclarationKind, Block};

#[derive(Debug, Default)]
pub struct Cascader<'a> {
    parent_stack: Vec<&'a mut Element>,
}

impl Cascader<'_> {
    pub fn cascade(&mut self, input: &mut Vec<Node>, style: &StyleData) {
        for node in input {
            if let Node::Element(ref mut el) = node {
                let mut applicable_rules: Vec<Rule> = vec![];
                for rules in &style.styles {
                    for rule in &rules.rules {
                        if let Prelude::Selector(selector) = &rule.prelude {
                            if self.applicable(selector, &el.tag_name) {
                                applicable_rules.push(rule.clone());
                            }
                        }
                    }
                }
                for rule in applicable_rules {
                    if let Block::Declarations(declarations) = &rule.value {
                        for declaration in declarations {
                            self.apply(el, declaration.clone());
                        }
                    }
                }
            }
        }
    }
    
    pub fn applicable(&self, selector: &Selector, tag_name: &String) -> bool {
        match selector {
            Selector::Both(l, r) => {
                if let Selector::Type(t) = *l.clone() {
                    return t == *tag_name || self.applicable(r, tag_name);
                } else if let Selector::Universal = *l.clone() {
                    return true;
                } else {
                    return false;
                }
            },
            Selector::Universal => {
                return true;
            },
            Selector::Placeheld => {
                return false;
            },
            Selector::Type(t) => {
                return t == tag_name;
            }
            a => { return false; },
        }
    }

    pub fn apply(&self, element: &mut Element, declaration: Declaration) {
        match declaration.kind {
            DeclarationKind::Color(v) => element.css.color = v,
            DeclarationKind::Display(v) => element.css.display = v,
            DeclarationKind::FontSize(v) => element.css.font_size = v,
            DeclarationKind::TextAlign(v) => element.css.text_align = v,
            DeclarationKind::Unknown(_, _) => {},
        }
    }
}
