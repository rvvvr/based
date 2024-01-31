use colours::Rgba;

use super::{Component, CSSToken, CSSValue, CSSNumber};

pub trait Property {
    fn from_components(components: Vec<Component>) -> CSSValue<Self>
        where Self: Sized;
}

#[derive(Default, Debug, Clone)]
pub struct Colour {
    pub real: Rgba<u8>,
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            real: Rgba { red: r, green: g, blue: b, alpha: a }
        }
    }

    pub const MAROON: Colour =      Colour::new(0x80, 0x00, 0x00, 0xFF);
    pub const RED: Colour =         Colour::new(0xFF, 0x00, 0x00, 0xFF);
    pub const ORANGE: Colour =      Colour::new(0xFF, 0xA5, 0x00, 0xFF);
    pub const YELLOW: Colour =      Colour::new(0xFF, 0xFF, 0x00, 0xFF);
    pub const OLIVE: Colour =       Colour::new(0x80, 0x80, 0x00, 0xFF);
    pub const PURPLE: Colour =      Colour::new(0x80, 0x00, 0x80, 0xFF);
    pub const FUCHIA: Colour =      Colour::new(0xFF, 0x00, 0xFF, 0xFF);
    pub const WHITE: Colour =       Colour::new(0xFF, 0xFF, 0xFF, 0xFF);
    pub const LIME: Colour =        Colour::new(0x00, 0xFF, 0x00, 0xFF);
    pub const GREEN: Colour =       Colour::new(0x00, 0x80, 0x00, 0xFF);
    pub const NAVY: Colour =        Colour::new(0x00, 0x00, 0x80, 0xFF);
    pub const BLUE: Colour =        Colour::new(0x00, 0x00, 0xFF, 0xFF);
    pub const AQUA: Colour =        Colour::new(0x00, 0xFF, 0xFF, 0xFF);
    pub const TEAL: Colour =        Colour::new(0x00, 0x80, 0x80, 0xFF);
    pub const BLACK: Colour =       Colour::new(0x00, 0x00, 0x00, 0xFF);
    pub const SILVER: Colour =      Colour::new(0xc0, 0xc0, 0xc0, 0xFF);
    pub const GRAY: Colour =        Colour::new(0x80, 0x80, 0x80, 0xFF);

}

impl Property for Colour {
    fn from_components(components: Vec<Component>) -> CSSValue<Self> 
            where Self: Sized{
        CSSValue::Value(if let Some(Component::Token(CSSToken::Ident(t))) = components.get(0) {
            match t.as_str() {
                "maroon" =>     Self::MAROON,
                "red" =>        Self::RED,
                "orange" =>     Self::ORANGE,
                "yellow" =>     Self::YELLOW,
                "olive" =>      Self::OLIVE,
                "purple" =>     Self::PURPLE,
                "fuchia" =>     Self::FUCHIA,
                "white" =>      Self::WHITE,
                "lime" =>       Self::LIME,
                "green" =>      Self::GREEN,
                "navy" =>       Self::NAVY,
                "blue" =>       Self::BLUE,
                "aqua" =>       Self::AQUA,
                "teal" =>       Self::TEAL,
                "black" =>      Self::BLACK,
                "silver" =>     Self::SILVER,
                "gray" =>       Self::GRAY,
                _ => return CSSValue::default(),
            }
        } else {
            return CSSValue::default();
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Display {
    outside: DisplayOutside,
    inside: DisplayInside,
}

impl Display {
    pub const fn new(outside: DisplayOutside, inside: DisplayInside) -> Self {
        Self {
            outside,
            inside,
        }
    }
    
    const NONE: Display =               Self::new(DisplayOutside::None, DisplayInside::Flow);
    const CONTENTS: Display =           Self::new(DisplayOutside::Contents, DisplayInside::Flow);
    const BLOCK: Display =              Self::new(DisplayOutside::Block, DisplayInside::Flow);
    const FLOW_ROOT: Display =          Self::new(DisplayOutside::Block, DisplayInside::FlowRoot);
    const INLINE: Display =             Self::new(DisplayOutside::Inline, DisplayInside::Flow);
    const INLINE_BLOCK: Display =       Self::new(DisplayOutside::Inline, DisplayInside::FlowRoot);
    const RUN_IN: Display =             Self::new(DisplayOutside::RunIn, DisplayInside::Flow);
    const LIST_ITEM: Display =          unimplemented!();
    const INLINE_LIST_ITEM: Display =   unimplemented!();
}

impl Property for Display {
    fn from_components(components: Vec<Component>) -> CSSValue<Self>
            where Self: Sized {
        if components.len() == 1 {
            CSSValue::Value(if let Some(Component::Token(CSSToken::Ident(t))) = components.get(0) {
                match t.as_str() {
                    "none" =>               Self::NONE,
                    "contents" =>           Self::CONTENTS,
                    "block" =>              Self::BLOCK,
                    "flow-root" =>          Self::FLOW_ROOT,
                    "inline" =>             Self::INLINE,
                    "inline-block" =>       Self::INLINE_BLOCK,
                    "run-in" =>             Self::RUN_IN,
                    _ => return CSSValue::default(),
                }
            } else {
                return CSSValue::default();
            })
        } else {
            unimplemented!();
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum DisplayOutside {
    #[default]
    Block,
    Inline,
    RunIn,
    None,
    Contents,
}

#[derive(Debug, Clone, Default)]
pub enum DisplayInside {
    #[default]
    Flow,
    FlowRoot,
    Table,
    Flex,
    Grid,
    Ruby,
}

#[derive(Debug, Clone, Default)]
pub struct FontSize {
    value: CSSNumber,
}

impl FontSize {
    pub const fn new(value: CSSNumber) -> Self {
        Self {
            value
        }
    }
}

impl Property for FontSize {
    fn from_components(components: Vec<Component>) -> CSSValue<Self>
            where Self: Sized {
        if let Some(Component::Token(CSSToken::Number(n))) = components.get(0) {
            CSSValue::Value(Self {
                value: n.clone(),
            })
        } else {
            unimplemented!();
        }
    }
}


#[derive(Debug, Clone, Default)]
pub struct Dimensionality { 
    pub value: CSSNumber,
}

impl Dimensionality {
    pub const fn new(value: CSSNumber) -> Self {
        Self {
            value
        }
    }
}

impl Property for Dimensionality {
    fn from_components(components: Vec<Component>) -> CSSValue<Self>
            where Self: Sized {
        if let Some(Component::Token(CSSToken::Number(n))) = components.get(0) {
            CSSValue::Value(Self {
                value: n.clone()
            })
        } else {
            unimplemented!();
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    #[default]
    Justify
}

impl Property for TextAlign {
    fn from_components(components: Vec<Component>) -> CSSValue<Self>
            where Self: Sized {
        if let Some(Component::Token(CSSToken::Ident(t))) = components.get(0) {
            CSSValue::Value(match t.as_str() {
                "left" => Self::Left,
                "right" => Self::Right,
                "center" => Self::Center,
                "justify" => Self::Justify,
                "inherit" => return CSSValue::Inherit,
                _ => return CSSValue::default(),
            })
        } else {
            CSSValue::default()
        }
    }
}
//these could all be squashed into a macro

#[derive(Debug, Clone)]
pub struct Spacing {
    value: CSSNumber
}

impl Spacing {
    pub const fn new(value: CSSNumber) -> Self {
        Self {
            value
        }
    }
}

impl Property for Spacing {
    fn from_components(components: Vec<Component>) -> CSSValue<Self>
            where Self: Sized {
        if let Some(Component::Token(CSSToken::Number(n))) = components.get(0) {
            CSSValue::Value(Self {
                value: n.clone()
            })
        } else {
            unimplemented!();
        }
    }
}
