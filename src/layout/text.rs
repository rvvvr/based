use std::io::Cursor;

use font_kit::font::Font;
use read_fonts::{FontRef, TableProvider};

use crate::parser::css::{CSSProps, properties::FontFamily, CSSValue};

use super::LayoutInfo;

#[derive(Clone, Debug)]
pub struct TextLayoutifier<'a> {
    containing_css: &'a CSSProps,
    container: &'a LayoutInfo,
    contents: &'a str,
    scale_factor: f64,
}
//my own glyph shaping and text layout engine.
//why am i writing this myself rather than just bringing in harfbuzz or pango or whatever and calling it a day?
//same reason i'm writing a browser engine. i dont know how fonts work, but i'd like to. therefore, this module.
//there's also the added bonus of no complete glyph shaping engine exists in pure rust, and it'd be nice to have one. not that things need to be written in rust to be usable- but it can't hurt.
//this will need to be massively expanded upon and eventually lifted out into its own crate.
impl<'a> TextLayoutifier<'a> {
    pub fn new(containing_css: &'a CSSProps, container: &'a LayoutInfo, contents: &'a str, scale_factor: f64) -> Self {
	Self {
	    containing_css,
	    container,
	    contents,
	    scale_factor,
	}
    }

    pub fn lay_it_out(&self) -> LaidoutText {
	let font = if let CSSValue::Value(FontFamily::Resolved(font)) = &self.containing_css.font_family {
	    font
	} else {
	    unreachable!();
	};

	let font_data = font.copy_font_data().unwrap();

	let ot_data = FontRef::new(font_data.as_slice()).unwrap();
	let head = ot_data.head().unwrap();
	let cmap = ot_data.cmap().expect("error reading");

	println!("{:?}", self.contents.chars().collect::<Vec<_>>());

	let glyph_ids = self.contents.chars().map(|v| cmap.map_codepoint(v)).filter_map(|v| v);

	let htmx = ot_data.hmtx().unwrap();

	let font_size = self.unwrap_font_size();

	let font_unit_scale_factor = (font_size * self.scale_factor) / head.units_per_em() as f64;

	let mut glyphs = Vec::new();
	let mut x_offset: f64 = self.container.x;
	for id in glyph_ids {
	    glyphs.push(LaidoutGlyph {
		x: x_offset,
		y: self.container.y + head.y_max() as f64 * font_unit_scale_factor,
		glyph: FontGlyph {
		    id: id.to_u16(),
		},
	    });
	    let horizontal_metrics = htmx.h_metrics().get(id.to_u16() as usize).unwrap();
	    x_offset += (horizontal_metrics.advance.get() as f64 * font_unit_scale_factor) + (horizontal_metrics.side_bearing.get() as f64 * font_unit_scale_factor);
	}
	
	LaidoutText {
	    glyphs,
	    font: font.clone(),
	    font_size,
	}
    }

    fn unwrap_font_size(&self) -> f64 {
	if let CSSValue::Value(font_size) = self.containing_css.font_size {
	    font_size.value.unwrap().unwrap_f64()
	} else {
	    unreachable!()
	}
    }
}

#[derive(Debug)]
pub struct LaidoutText {
    pub glyphs: Vec<LaidoutGlyph>,
    pub font: Font,
    pub font_size: f64,
}

impl Iterator for LaidoutText {
    type Item = LaidoutGlyph;

    fn next(&mut self) -> Option<Self::Item> {
	None
    }
}

#[derive(Debug)]
pub struct LaidoutGlyph {
    pub x: f64,
    pub y: f64,
    pub glyph: FontGlyph,
}

#[derive(Debug)]
pub struct FontGlyph { //will probably jsut become pub type FontGlyph = usize; but for now keeping it as a struct in case i want to store extra data on a glyph.
    pub id: u16,
}
