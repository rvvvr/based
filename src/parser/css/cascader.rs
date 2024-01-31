use crate::{dom::{Node, Element}, parser::css::{Selector, Rule, properties::Dimensionality}, context::Viewport};

use super::{StyleData, Prelude, Declaration, DeclarationKind, Block, CSSValue, properties::{Colour, TextAlign, FontSize, Display, Spacing}, CSSProps, RuleBuilder, CSSNumber, Unit, Numeric};

#[derive(Debug, Default)]
pub struct Cascader {
    parent_prop_stack: Vec<CSSProps>,
    parent_name_stack: Vec<String>,
    last_sibling: String,
}

impl<'a> Cascader {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn cascade(&mut self, input: &mut Vec<Node>, style: &StyleData, viewport: Viewport) {
        self.parent_prop_stack.push(CSSProps {
            width: CSSValue::Value(Dimensionality::new(CSSNumber::Unit(Numeric::Integer(viewport.width as i32), Unit::Px))),
            height: CSSValue::Value(Dimensionality::new(CSSNumber::Unit(Numeric::Integer(viewport.height as i32), Unit::Px))),
            ..Default::default()
        })
    }

    fn cascade_internal(&'a mut self, input: &'a mut Vec<Node>, style: &StyleData) {
        println!("shmop");
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
                let mut real_rule = RuleBuilder::new(false).build().unwrap();
                for ref mut rule in applicable_rules {
                    real_rule.squash(rule);
                }
                self.defaulterizeificate(&mut real_rule);
                if let Block::Declarations(declarations) = real_rule.value {
                    for declaration in declarations.values() {
                        self.apply(el, declaration.clone());
                    }
                }
                self.parent_prop_stack.push(el.css.clone());
                self.last_sibling = el.tag_name.clone();
                self.parent_name_stack.push(el.tag_name.clone());
                self.cascade_internal(&mut el.children, style);
                self.parent_prop_stack.pop();
                self.parent_name_stack.pop();
            }
        }
    }

    //me when no function overloading.....
    pub fn defaulterizeificate(&mut self, rule: &mut Rule) {
        if let Block::Declarations(ref mut declarations) = rule.value {
            for declaration in declarations.values_mut() {
                match declaration.kind {
                    DeclarationKind::Unknown(..) => {},
                    DeclarationKind::Color(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().color.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Colour>::default();
                        }
                    }
                    DeclarationKind::TextAlign(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().text_align.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<TextAlign>::default();
                        }
                    }
                    DeclarationKind::Display(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().display.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Display>::default();
                        }
                    }
                    DeclarationKind::FontSize(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().font_size.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<FontSize>::default();
                        }
                    }
                    DeclarationKind::BackgroundColor(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().background_color.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Colour>::default();
                        }
                    }
                    DeclarationKind::Width(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().width.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Dimensionality>::default();
                        }
                    }
                    DeclarationKind::Height(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().height.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Dimensionality>::default();
                        }
                    }
                    DeclarationKind::MarginTop(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().margin_top.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::MarginBottom(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().margin_bottom.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::MarginLeft(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().margin_left.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::MarginRight(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().margin_right.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::PaddingTop(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().padding_top.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::PaddingBottom(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().padding_bottom.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::PaddingLeft(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().padding_left.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                    DeclarationKind::PaddingRight(ref mut v) => {
                        if let CSSValue::Inherit = v {
                            *v = self.parent_prop_stack.last().unwrap().padding_right.clone();
                        } else if let CSSValue::Initial = v {
                            *v = CSSValue::<Spacing>::default();
                        }
                    }
                }
            }
        }
    }
    
    pub fn applicable(&self, selector: &Selector, tag_name: &String) -> bool {
        match selector {
            Selector::Both(l, r) => {
                return self.applicable(l, tag_name) || self.applicable(r, tag_name);
            },
            Selector::Universal => {
                return true;
            },
            Selector::Placeheld => {
                return false;
            },
            Selector::Type(t) => {
                return t == tag_name;
            },
            Selector::NextSibling(l, r) => {
                return self.applicable(l, &self.last_sibling) && self.applicable(r, tag_name);
            },
            Selector::Child(l, r) => {
                return self.applicable(l, &self.parent_name_stack.last().unwrap_or(&String::new())) && self.applicable(r, tag_name);
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
            DeclarationKind::BackgroundColor(v) => element.css.background_color = v,
            DeclarationKind::Width(v) => element.css.width = v,
            DeclarationKind::Height(v) => element.css.height = v,
            DeclarationKind::MarginTop(v) => element.css.margin_top = v,
            DeclarationKind::MarginBottom(v) => element.css.margin_bottom = v,
            DeclarationKind::MarginLeft(v) => element.css.margin_left = v,
            DeclarationKind::MarginRight(v) => element.css.margin_right = v,
            DeclarationKind::PaddingTop(v) => element.css.padding_top = v,
            DeclarationKind::PaddingBottom(v) => element.css.padding_bottom = v,
            DeclarationKind::PaddingLeft(v) => element.css.padding_left = v,
            DeclarationKind::PaddingRight(v) => element.css.padding_right = v,
            DeclarationKind::Unknown(_, _) => {},
        }
    }
}
