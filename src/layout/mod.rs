use std::process::Command;
use std::io::{prelude::*, self};
use crate::{dom::{Element, Document, Node, DOMCoordinate}, context::Viewport, parser::css::{CSSProps, CSSValue, properties::Dimensionality, CSSNumber, Numeric, Unit}};

#[derive(Default, Debug)]
pub struct Layoutifier {
    parent_stack: Vec<LayoutNode> //probably worth making a `LayoutInfo` type that holds everything i
                                //need while building the layout but this is fine for now...
}
//three billion million trees to represent the same document is a little redundantge maybe...
fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

impl Layoutifier {
    pub fn layoutify(&mut self, viewport: Viewport, docment: &Document) -> LayoutNode {
        let mut root_node = LayoutNode {
            element: None,
            absolute_width: viewport.width,
            absolute_height: viewport.height,
            children: Vec::new(),
        };
        self.parent_stack.push(root_node.clone());
        for node in &docment.children {
            if let Node::Element(el) = node {
                if el.tag_name == "head" { continue; } // each element should have it's own type
                                                       // with some sort of defaulted "Layoutible"
                                                       // field but for now this is finege..
                root_node.children.extend(self.layoutify_recursive(&el.children));
            }
        }
        root_node
    }

    fn layoutify_recursive(&mut self, nodes: &Vec<Node>) -> Vec<LayoutNode> {
        let mut out = Vec::with_capacity(nodes.len());
        for node in nodes {
            if let Node::Element(el) = node {
                if el.tag_name == "head" {continue;}
                let mut n = LayoutNode { 
                    element: Some(ChildlessElement::from(el)), 
                    absolute_width: self.compute_dimension(true, &el.css.width),
                    absolute_height: self.compute_dimension(false, &el.css.height), 
                    children: Vec::new(),
                };
                self.parent_stack.push(n.clone());
                n.children.extend(self.layoutify_recursive(&el.children));
                out.push(n);
                self.parent_stack.pop();
            }
        }
        out
    }

    fn compute_dimension(&self, width: bool, dimension: &CSSValue<Dimensionality>) -> usize {
        if matches!(dimension, CSSValue::Inherit | CSSValue::Initial) {
            if width {
                self.parent_stack.last().unwrap().absolute_width
            } else {
                self.parent_stack.last().unwrap().absolute_height
            }
        } else if let CSSValue::Value(v) = dimension {
            match v.value {
                CSSNumber::Number(n) => {
                    match n {
                        Numeric::Number(f) => {
                            f as usize
                        },
                        Numeric::Integer(i) => {
                            i as usize
                        },
                    }
                },
                CSSNumber::Percentage(n) => {
                    let percent = match n {
                        Numeric::Number(f) => {
                            f / 100.
                        },
                        Numeric::Integer(i) => {
                            (i as f32) / 100.
                        },
                    };
                    if width {
                        ((self.parent_stack.last().unwrap().absolute_width as f32) * percent) as usize
                    } else {
                        ((self.parent_stack.last().unwrap().absolute_height as f32) * percent) as usize
                    }
                },
                CSSNumber::Unit(n, u) => {
                    match u {
                        Unit::Px => {
                            match n {
                                Numeric::Number(f) => {
                                    f as usize
                                },
                                Numeric::Integer(i) => {
                                    i as usize
                                },
                            }
                        },
                        a => unimplemented!("{:?}", a),
                    }
                },
            }
        } else {
            unreachable!()
        }
    }
}

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub element: Option<ChildlessElement>, //could be a "phantom" box for combining display modes
                                           //or to act as the root node
    pub absolute_width: usize,
    pub absolute_height: usize,
    pub absolute_padding: (usize, usize, usize, usize),
    pub absolute_margin: (usize, usize, usize, usize), //top, bottom, left, right
    pub children: Vec<LayoutNode>,
}

#[derive(Clone, Debug)]
pub struct ChildlessElement {
    pub tag_name: String,
    pub data: String,
    pub coordinate: DOMCoordinate,
    pub css: CSSProps,
    pub attributes: Vec<(String, String)>,
}

impl From<&Element> for ChildlessElement {
    fn from(value: &Element) -> Self {
        Self {
            tag_name: value.tag_name.clone(),
            css: value.css.clone(),
            coordinate: value.coordinate.clone(),
            data: value.data.clone(),
            attributes: value.attributes.clone(),
        }
    }
}
