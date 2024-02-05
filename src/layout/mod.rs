use std::process::Command;
use std::io::{prelude::*, self};
use crate::parser::css::properties::{Display, DisplayOutside};
use crate::{dom::{Element, Document, Node, DOMCoordinate}, context::Viewport, parser::css::{CSSProps, CSSValue, properties::Dimensionality, CSSNumber, Numeric, Unit}};

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

impl Element {
    pub fn layout(&mut self, container: LayoutInfo) {
        if self.tag_name == "head" {return;}
        if let CSSValue::Value(display) = self.css.display {
            match display.outside {
                DisplayOutside::Block => self.layout_block(container),
                a => unimplemented!("{:?}", a)
            }
        }
    }

    fn layout_block(&mut self, container: LayoutInfo) {
        self.calculate_width_block(container);
        self.calculate_pos_block(container);
        self.go_children();
        self.calculate_height_block(container);
    }

    fn calculate_width_block(&mut self, container: LayoutInfo) {
        let mut width = self.unwrap_widthwise_dimension(&self.css.width, container);

        let mut margin_left = self.unwrap_widthwise_dimension(&self.css.margin_left, container);
        let mut margin_right = self.unwrap_widthwise_dimension(&self.css.margin_right, container);

        //TODO: let border_left = 
        //TODO: let border_right =

        let padding_left = self.unwrap_widthwise_dimension(&self.css.padding_left, container);
        let padding_right = self.unwrap_widthwise_dimension(&self.css.padding_right, container);

        let total_width = margin_left.v() + /*border_left +*/ padding_left.v() + width.v() + padding_right.v() + /*border_right +*/ margin_right.v();

        if !matches!(width, NearlyExactDimension::Auto) && total_width > container.width {
            margin_left = NearlyExactDimension::Value(0.);
            margin_right = NearlyExactDimension::Value(0.);
        }

        let underflow = container.width - total_width;
        {
            use NearlyExactDimension::*;
            match (width, margin_left, margin_right) {
                (Value(_), Value(_), Value(ref mut v)) => {
                    //TODO: Check direction property.
                    *v += underflow;
                },
                (Value(_), Auto, Value(_)) => {
                    margin_left = NearlyExactDimension::Value(underflow);
                },
                (Value(_), Value(_), Auto) => {
                    margin_right = NearlyExactDimension::Value(underflow);
                }
                (Auto, ..) => {
                    if let Auto = margin_left {
                        margin_left = NearlyExactDimension::Value(0.);
                    }
                    if let Auto = margin_right {
                        margin_right = NearlyExactDimension::Value(0.);
                    }

                    if underflow >= 0. {
                        width = NearlyExactDimension::Value(underflow);
                    } else {
                        width = NearlyExactDimension::Value(0.);
                        margin_right = NearlyExactDimension::Value(margin_right.v() + underflow);
                    }
                }

                (Value(_), Auto, Auto) => {
                    margin_left = NearlyExactDimension::Value(underflow / 2.);
                    margin_right = NearlyExactDimension::Value(underflow / 2.);
                }
            }
        }

        self.layout_info.width = width.v();
        self.layout_info.padding.1 = padding_left.v();
        self.layout_info.padding.2 = padding_right.v();
        self.layout_info.margin.1 = margin_left.v();
        self.layout_info.margin.2 = margin_right.v();
        //TODO: set border
    }

    fn calculate_pos_block(&mut self, container: LayoutInfo) {
        self.layout_info.margin.0 = self.unwrap_heightwise_dimension(&self.css.margin_top, container).v();
        self.layout_info.margin.3 = self.unwrap_heightwise_dimension(&self.css.margin_bottom, container).v();

        //TODO: border
        self.layout_info.padding.0 = self.unwrap_heightwise_dimension(&self.css.padding_top, container).v();
        self.layout_info.padding.3 = self.unwrap_heightwise_dimension(&self.css.padding_bottom, container).v();

        self.layout_info.x = container.x + self.layout_info.margin.1 + /*self.layout_info.border.1 +*/ self.layout_info.padding.1;
        self.layout_info.y = container.content_height + container.y + self.layout_info.margin.0 + /*self.layout_info.border.0 +*/ self.layout_info.padding.0;
    }

    fn go_children(&mut self) {
        for child in &mut self.children {
            if let Node::Element(el) = child {
                el.layout(self.layout_info);
                self.layout_info.content_height += el.layout_info.margin.0 + /*el.layout_info.border.0 +*/ el.layout_info.padding.0 + el.layout_info.height
                    + el.layout_info.padding.3 + /*el.layout_info.border.3 +*/ el.layout_info.padding.3;
            }
        }
    }

    fn calculate_height_block(&mut self, container: LayoutInfo) {
        let height = match self.css.height {
            CSSValue::Value(width) => {
                match width {
                    Dimensionality::Auto => {
                        NearlyExactDimension::Auto
                    },
                    Dimensionality::Real(v) => {
                        match v {
                            CSSNumber::Unit(v, u) => {
                                match u {
                                    Unit::Px => {
                                        NearlyExactDimension::Value(v.unwrap_f64())
                                    },
                                    a => unimplemented!("{:?}", a),
                                }
                            }
                            CSSNumber::Number(v) => {
                                NearlyExactDimension::Value(v.unwrap_f64())
                            }
                            CSSNumber::Percentage(v) => {
                                NearlyExactDimension::Auto
                            }
                        }
                    }
                }
            },
            CSSValue::Inherit => {
                unreachable!()
            },
            CSSValue::Initial => {
                NearlyExactDimension::Auto
            }
        };
        if let NearlyExactDimension::Value(v) = height {
            self.layout_info.height = v;
        } else {
            self.layout_info.height = self.layout_info.content_height;
        }
    }

    fn unwrap_widthwise_dimension(&self, dimension: &CSSValue<Dimensionality>, container: LayoutInfo) -> NearlyExactDimension {
        match dimension {
            CSSValue::Value(width) => {
                match width {
                    Dimensionality::Auto => {
                        NearlyExactDimension::Auto
                    },
                    Dimensionality::Real(v) => {
                        match v {
                            CSSNumber::Unit(v, u) => {
                                match u {
                                    Unit::Px => {
                                        NearlyExactDimension::Value(v.unwrap_f64())
                                    },
                                    a => unimplemented!("{:?}", a),
                                }
                            }
                            CSSNumber::Number(v) => {
                                NearlyExactDimension::Value(v.unwrap_f64())
                            }
                            CSSNumber::Percentage(v) => {
                                NearlyExactDimension::Value((container.width) * (v.unwrap_f64() / 100.))
                            }
                        }
                    }
                }
            },
            CSSValue::Inherit => {
                unreachable!()
            },
            CSSValue::Initial => {
                NearlyExactDimension::Auto
            }
        }
    }

    fn unwrap_heightwise_dimension(&self, dimension: &CSSValue<Dimensionality>, container: LayoutInfo) -> NearlyExactDimension {
        match dimension {
            CSSValue::Value(height) => {
                match height {
                    Dimensionality::Auto => {
                        NearlyExactDimension::Auto
                    },
                    Dimensionality::Real(v) => {
                        match v {
                            CSSNumber::Unit(v, u) => {
                                match u {
                                    Unit::Px => {
                                        NearlyExactDimension::Value(v.unwrap_f64())
                                    },
                                    a => unimplemented!("{:?}", a),
                                }
                            }
                            CSSNumber::Number(v) => {
                                NearlyExactDimension::Value(v.unwrap_f64())
                            }
                            CSSNumber::Percentage(v) => {
                                NearlyExactDimension::Value((container.height) * (v.unwrap_f64() / 100.))
                            }
                        }
                    }
                }
            },
            CSSValue::Inherit => {
                unreachable!()
            },
            CSSValue::Initial => {
                NearlyExactDimension::Auto
            }
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub enum NearlyExactDimension {
    #[default]
    Auto,
    Value(f64),
}

impl NearlyExactDimension {
    pub fn v(&self) -> f64 {
        match self {
            Self::Value(v) => *v,
            Self::Auto => 0.,
        }
    }
}

pub enum BuildingHeight {
    WaitingForContents,
    Value(f64),
}


#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutInfo { // renderer's job should be to render, not figure out the place on the
                        // page. my bad on the last impl.
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub content_height: f64,
    pub margin: (f64, f64, f64, f64), //top, left, right, bottom
    pub padding: (f64, f64, f64, f64),
}

impl LayoutInfo {
    pub fn expand(&self, expand: (f64, f64, f64, f64)) -> LayoutInfo {
        LayoutInfo { x: self.x - expand.1, y: self.y - expand.0, width: self.width + expand.1 + expand.2, height: self.height + expand.0 + expand.3, content_height: self.content_height, margin: self.margin, padding: self.padding }
    }
}
