use std::io::Cursor;

use font_kit::font::Font;
use font_types::F2Dot14;
use read_fonts::{
    tables::hmtx::Hmtx,
    types::{GlyphId, Tag},
    FontRef, TableProvider,
};
use vello::glyph;

use super::LayoutInfo;
use crate::{
    layout::text::font_data::FontData,
    parser::css::{
        properties::{Colour, FontFamily, TextAlign},
        CSSProps, CSSValue,
    },
};

pub mod font_data;

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
//TODO: Presume nothing about a font
impl<'a> TextLayoutifier<'a> {
    pub fn new(
        containing_css: &'a CSSProps,
        container: &'a LayoutInfo,
        contents: &'a str,
        scale_factor: f64,
    ) -> Self {
        Self {
            containing_css,
            container,
            contents,
            scale_factor,
        }
    }

    pub fn lay_it_out(&self, parent_height: &mut f64) -> LaidoutText {
        let font =
            if let CSSValue::Value(FontFamily::Resolved(font)) = &self.containing_css.font_family {
                font
            } else {
                unreachable!();
            };

        let font_raw = font.copy_font_data().unwrap();

        let ot_data = FontRef::new(font_raw.as_slice()).unwrap();
        println!("{:?}", self.contents.chars().collect::<Vec<_>>());

        let mut font_glyphs = Vec::new();

        let hmtx = ot_data.hmtx().unwrap();
        let head = ot_data.head().unwrap();
        let cmap = ot_data.cmap().unwrap();
        let axes = if let Ok(fvar) = ot_data.fvar() {
            let axes = fvar.axes().unwrap();
            let mut out = Vec::with_capacity(axes.len());
            for axis in axes {
                if let Some(val) = self
                    .containing_css
                    .val_for_variable_tag(axis.axis_tag.get())
                {
                    let clamped =
                        val.clamp(axis.min_value.get().to_f64(), axis.max_value.get().to_f64());
                    let normalized = clamped / axis.max_value.get().to_f64();
                    let twodot14 = F2Dot14::from_f32(normalized as f32);
                    out.push(twodot14);
                } else {
                    //assuming that default value in a font has been clamped already... would love to be proven wrong kappa
                    let normalized = axis.default_value.get() / axis.max_value.get();
                    let twodot14 = normalized.to_f2dot14();
                    out.push(twodot14);
                }
            }
            Some(out)
        } else {
            None
        };
        //this chain is kinda just a hack to treat the last word as a word ill need to do something more real at some point.
        for ch in self.contents.chars().chain("  ".chars()) {
            let mut glyph = FontGlyph::default();
            if ch.is_whitespace() {
                glyph.breakable = true;
            }
            //check if newline and set broken.
            let gid = cmap.map_codepoint(ch as u32);
            if let Some(id) = gid {
                glyph.id = id.to_u16();
                font_glyphs.push(glyph);
            }
        }
        let font_size = self.unwrap_font_size();

        let font_unit_scale_factor = (font_size * self.scale_factor) / head.units_per_em() as f64;

        let mut glyphs = Vec::new();
        let mut y_offset = self.container.y + self.container.content_height;
        let mut line = Vec::new();
        let mut wordish = Vec::new();
        let mut glyphs_peekable = font_glyphs.iter().peekable();
        while let Some(glyph) = glyphs_peekable.next() {
            wordish.push(*glyph);
            if glyph.breakable || glyph.broken {
                let line_length =
                    self.get_glyphs_length(&line, &hmtx, font_unit_scale_factor, true, true);
                let word_length =
                    self.get_glyphs_length(&wordish, &hmtx, font_unit_scale_factor, false, false);
                if self.container.x + self.container.padding.1 - self.container.padding.2
                    + line_length
                    + word_length
                    >= self.container.x + self.container.width + self.container.padding.1
                        - self.container.padding.2
                    || glyph.broken
                    || glyphs_peekable.peek().is_none()
                {
                    let mut x_offset = match self.containing_css.text_align.unwrap() {
                        TextAlign::Center => {
                            self.container.x
                                + ((self.container.width + self.container.padding.1 - line_length)
                                    / 2.)
                        }
                        TextAlign::Left => self.container.x,
                        TextAlign::Right => {
                            self.container.x
                                + (self.container.width + self.container.padding.1 - line_length)
                        }
                        TextAlign::Justify => unimplemented!("text-align: justify"),
                    };
                    y_offset += head.y_max() as f64 * font_unit_scale_factor;
                    *parent_height += head.y_max() as f64 * font_unit_scale_factor; //need to find out how to make shit with lines that go below fit
                    for letter in &line {
                        glyphs.push(LaidoutGlyph {
                            x: x_offset,
                            y: y_offset,
                            glyph: *letter,
                        });
                        let h_metrics = hmtx
                            .h_metrics()
                            .get(letter.id as usize)
                            .expect("no metrics");
                        x_offset += h_metrics.advance() as f64 * font_unit_scale_factor
                            + h_metrics.side_bearing() as f64 * font_unit_scale_factor;
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
            axes,
        }
    }

    fn get_glyphs_length(
        &self,
        word: &Vec<FontGlyph>,
        hmtx: &Hmtx,
        scale_factor: f64,
        trim_leading: bool,
        trim_trailing: bool,
    ) -> f64 {
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
                    last_index = i + 1;
                    break;
                }
            }
        }
        for letter in &word[first_index..last_index] {
            let h_metrics = hmtx.h_metrics().get(letter.id as usize).unwrap();
            length += h_metrics.advance() as f64 * scale_factor
                + h_metrics.side_bearing() as f64 * scale_factor;
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
    pub axes: Option<Vec<F2Dot14>>,
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
pub struct FontGlyph {
    //will probably jsut become pub type FontGlyph = usize; but for now keeping it as a struct in case i want to store extra data on a glyph.
    pub id: u16,
    pub broken: bool,
    pub breakable: bool,
}
