use std::io::Cursor;

use font_kit::font::Font;
use read_fonts::{FontRef, TableProvider, types::GlyphId, tables::hmtx::Hmtx};
use vello::glyph;

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

    pub fn lay_it_out(&self, parent_height: &mut f64) -> LaidoutText {
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

	let mut font_glyphs = Vec::new();
	
	//this chain is kinda just a hack to treat the last word as a word ill need to do something more real at some point.
	for ch in self.contents.chars().chain(std::iter::once(' ')) {
	    let mut glyph = FontGlyph::default();
	    if ch.is_whitespace() {
		glyph.breakable = true;
	    }
	    //check if newline and set broken.
	    let gid = cmap.map_codepoint(ch);
	    if let Some(id) = gid {
		glyph.id = id.to_u16();
		font_glyphs.push(glyph);
	    }
	}
	let hmtx = ot_data.hmtx().unwrap();

	let font_size = self.unwrap_font_size();

	let font_unit_scale_factor = (font_size * self.scale_factor) / head.units_per_em() as f64;

	let mut glyphs = Vec::new();
	let mut x_offset: f64 = self.container.x;
	let mut y_offset: f64 = self.container.y + head.y_max() as f64 * font_unit_scale_factor;
	*parent_height += head.y_max() as f64 * font_unit_scale_factor;
	let mut wordish = Vec::new();
	for glyph in font_glyphs {
	    wordish.push(glyph);
	    if glyph.breakable || glyph.broken {
		let word_length = self.get_word_length(&wordish, &hmtx, font_unit_scale_factor);
		if x_offset + word_length >= self.container.x + self.container.width || glyph.broken {
		    x_offset = self.container.x;
		    y_offset += head.y_max() as f64 * font_unit_scale_factor;
		    *parent_height += head.y_max() as f64 * font_unit_scale_factor;
		}
		for letter in &wordish {
		    glyphs.push(LaidoutGlyph {
			x: x_offset,
			y: y_offset,
			glyph: *letter,
		    });
		    let h_metrics = hmtx.h_metrics().get(letter.id as usize).expect("no metrics");
		    x_offset += h_metrics.advance() as f64 * font_unit_scale_factor + h_metrics.side_bearing() as f64 * font_unit_scale_factor;
		}
		wordish.clear();
	    }
	}
	
	LaidoutText {
	    glyphs,
	    font: font.clone(),
	    font_size,
	}
    }

    fn get_word_length(&self, word: &Vec<FontGlyph>, hmtx: &Hmtx,scale_factor: f64) -> f64 {
	let mut length = 0.;
	for letter in word {
	    let h_metrics = hmtx.h_metrics().get(letter.id as usize).expect("no metrics");
	    length += h_metrics.advance() as f64 * scale_factor + h_metrics.side_bearing() as f64 * scale_factor;
	}
	length
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

#[derive(Debug, Default, Copy, Clone)]
pub struct FontGlyph { //will probably jsut become pub type FontGlyph = usize; but for now keeping it as a struct in case i want to store extra data on a glyph.
    pub id: u16,
    pub broken: bool,
    pub breakable: bool,
}
