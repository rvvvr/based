use std::io::Cursor;

use font_kit::font::Font;
use read_fonts::{FontRef, TableProvider, types::GlyphId, tables::hmtx::Hmtx};
use vello::glyph;

use crate::parser::css::{CSSProps, properties::{FontFamily, Colour, TextAlign}, CSSValue};

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
	for ch in self.contents.chars().chain("  ".chars()) {
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
	let mut y_offset = self.container.y;
	let mut line = Vec::new();
	let mut wordish = Vec::new();
	let mut glyphs_peekable = font_glyphs.iter().peekable();
	while let Some(glyph) = glyphs_peekable.next() {
	    wordish.push(*glyph);
	    if glyph.breakable || glyph.broken {
		let line_length = self.get_glyphs_length(&line, &hmtx, font_unit_scale_factor, true, true);
		let word_length = self.get_glyphs_length(&wordish, &hmtx, font_unit_scale_factor, false, false);
		if self.container.x + line_length + word_length >= self.container.x + self.container.width || glyph.broken || glyphs_peekable.peek().is_none() {
		    let mut x_offset = match self.containing_css.text_align.unwrap() {
			TextAlign::Center => {
			    self.container.x + ((self.container.width - line_length) / 2.) 
			},
			TextAlign::Justify | TextAlign::Left => {
			    self.container.x
			},
			TextAlign::Right => {
			    self.container.x + (self.container.width - line_length)
			},
		    };
		    y_offset += head.y_max() as f64 * font_unit_scale_factor;
		    *parent_height += head.y_max() as f64 * font_unit_scale_factor; //need to find out how to make shit with lines that go below fit
		    for letter in &line {
			glyphs.push(LaidoutGlyph {
			    x: x_offset,
			    y: y_offset,
			    glyph: *letter,
			});
			let h_metrics = hmtx.h_metrics().get(letter.id as usize).expect("no metrics");
			x_offset += h_metrics.advance() as f64 * font_unit_scale_factor + h_metrics.side_bearing() as f64 * font_unit_scale_factor;
		    }
		    line.clear();
		}
		line.extend(&wordish);
		wordish.clear();
	    }
	}
	
	LaidoutText {
	    glyphs,
	    font: font.clone(),
	    font_size,
	    colour: self.containing_css.color.unwrap(),
	}
    }

    fn get_glyphs_length(&self, word: &Vec<FontGlyph>, hmtx: &Hmtx,scale_factor: f64, trim_leading: bool, trim_trailing: bool) -> f64 {
	let mut length = 0.;
	let mut first_index: usize = 0;
	let mut last_index: usize = word.len();
	if trim_leading {
	for letter in word {
	    if letter.breakable || letter.broken {
		first_index += 1;
	    } else {
		break;
	    }
	}
	}
	if trim_trailing {
	for (i, letter) in word.iter().enumerate().rev() {
	    if letter.breakable || letter.broken {
		
	    } else {
		last_index = i+1;
		break;
	    }
	}
	}
	for letter in &word[first_index..last_index] {
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
    pub colour: Colour,
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
