use crate::{parser::css::{CSSSource, Style, StyleData, CSSProps, cascader::Cascader}, context::Viewport, layout::{LayoutInfo}};

#[derive(Default, Debug)]
pub struct Document {
    pub style: StyleData,
    pub document_mode: DocumentMode,
    pub children: Vec<Node>,
}

pub trait DOMElement {
    fn insert_element(&mut self, tag_name: String, attributes: Vec<(String, String)>) -> DOMCoordinate;
    fn insert_document_type(&mut self, name: String, system_id: String, public_id: String, quirks: bool);
    fn insert_comment(&mut self, data: String);
    fn get_element_for_coordinate(&mut self, coordinate: DOMCoordinate) -> &mut Element;
}

impl Document {
    pub fn print_tree(&self) {
        
    }

    pub fn find_css_sources(&self) -> Vec<CSSSource> {
        let mut out = Vec::with_capacity(5);
        Self::find_css_sources_recursive(&self.children, &mut out);
        return out;
    }

    fn find_css_sources_recursive(nodes: &Vec<Node>, out: &mut Vec<CSSSource>) {
        for node in nodes {
            if let Node::Element(el) = node {
                if el.tag_name == "style" {
                    out.push(CSSSource::Raw(el.data.clone()));
                    continue;
                }
                Self::find_css_sources_recursive(&el.children, out);
            }
        }
    }

    pub fn add_styles(&mut self, styles: Vec<Style>)  {
        self.style.styles.extend(styles);
    }

    pub fn add_style(&mut self, style: Style) {
        self.style.styles.push(style);
    }

    pub fn cascade(&mut self, viewport: Viewport) {
        Cascader::default().cascade(&mut self.children, &self.style, viewport);
    }

    pub fn layoutify(&mut self, viewport: Viewport) {
        for child in &mut self.children {
            if let Node::Element(el) = child {
                el.layout(Viewport::default().into_layout())
            }
        }
        println!("{:#?}", self);
    }
}

impl DOMElement for Document {
    fn insert_document_type(&mut self, name: String, system_id: String, public_id: String, quirks: bool) {
        self.children.push(Node::DocumentType(DocumentType { name, public_id, system_id }));
        if quirks {
            self.document_mode = DocumentMode::Quirks;
        }
    }

    fn insert_comment(&mut self, data: String) {
        self.children.push(Node::Comment { data });
    }

    fn insert_element(&mut self, tag_name: String, attributes: Vec<(String, String)>) -> DOMCoordinate {
        let coordinate = DOMCoordinate {
            indices: vec![self.children.len()]
        };
        self.children.push(Node::Element(Element { children: vec![], coordinate: coordinate.clone(), css: CSSProps::default(), tag_name, data: String::new(), attributes, layout_info: LayoutInfo::default() }));
        return coordinate;
    }
    
    #[allow(unreachable_code)]
    fn get_element_for_coordinate(&mut self, coordinate: DOMCoordinate) -> &mut Element {
        return if let Some(Node::Element(ref mut element)) = self.children.get_mut(*coordinate.indices.get(0).unwrap()) {
            if coordinate.indices.len() == 1 {
                return element;
            } else {
                return element.get_element_for_coordinate(DOMCoordinate { indices: coordinate.indices[1..].to_vec() });
            }
        } else {
            panic!();
        }
    }
}

#[derive(Default, Debug)]
pub enum DocumentMode {
    #[default]
    NoQuirks,
    Quirks,
    LimitedQuirks,
}

#[derive(Default, Debug)]
pub struct DocumentType {
    pub name: String,
    pub public_id: String,
    pub system_id: String,
}

#[derive(Default, Debug)]
pub struct Element {
    pub tag_name: String,
    pub data: String,
    pub coordinate: DOMCoordinate,
    pub css: CSSProps,
    pub children: Vec<Node>,
    pub attributes: Vec<(String, String)>,
    pub layout_info: LayoutInfo,
}

impl DOMElement for Element {
    fn insert_element(&mut self, tag_name: String, attributes: Vec<(String, String)>) -> DOMCoordinate {
        let mut coordinate = DOMCoordinate {
            indices: self.coordinate.indices.clone()
        };
        coordinate.indices.push(self.children.len());
        self.children.push(Node::Element(Element { children: vec![], coordinate: coordinate.clone(), css: CSSProps::default(), tag_name, data: String::new(), attributes, layout_info: LayoutInfo::default() }));
        return coordinate;
    }
    fn insert_comment(&mut self, data: String) {
        todo!();
    }
    fn insert_document_type(&mut self, name: String, system_id: String, public_id: String, quirks: bool) {
        todo!();
    }

    #[allow(unreachable_code)] // why is this unreachable???
    fn get_element_for_coordinate(&mut self, coordinate: DOMCoordinate) -> &mut Element {
        return if let Some(Node::Element(ref mut element)) = self.children.get_mut(*coordinate.indices.get(0).unwrap()) {
            if coordinate.indices.len() == 1 {
                return element;
            } else {
                return element.get_element_for_coordinate(DOMCoordinate { indices: coordinate.indices[1..].to_vec() });
            }
        } else {
            panic!();
        }
    }
}

#[derive(Debug)]
pub enum Node {
    DocumentType(DocumentType),
    Comment {
        data: String,

    },
    Element(Element),
    Text(String),
    PhantomBox(Element), //only for layoutge
}

#[derive(Default, Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct DOMCoordinate {
    indices: Vec<usize>,
}
