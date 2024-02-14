use core::slice::SlicePattern;

use font_types::{F2Dot14, GlyphId};
use read_fonts::{
    tables::{head::Head, vmtx::LongMetric},
    FontRef, TableProvider,
};

use super::TextLayoutifier;

pub struct FontData<'a> {
    font: FontRef<'a>,
    variable: bool,
    axes: Vec<F2Dot14>,
}

impl<'a> FontData<'a> {
    pub fn new(font: FontRef<'a>) -> Self {
        let mut axes = Vec::new();
        let mut variable = false;
        if let Ok(fvar) = font.fvar() {
            variable = true;
            for axis in fvar.axes().unwrap() {
                println!("{:?}", axis);
                axes.push((axis.default_value.get() / axis.max_value.get()).to_f2dot14());
            }
        }

        Self {
            font,
            variable,
            axes,
        }
    }

    pub fn head(&self) -> Head<'a> {
        self.font.head().unwrap()
    }

    pub fn set_axes(&mut self, axes: impl Iterator<Item = F2Dot14>) {
        self.axes = axes.collect::<Vec<_>>();
    }

    //TODO: my own, more complete implementation
    pub fn lookup_glyph(&self, codepoint: char) -> Option<u16> {
        self.font
            .cmap()
            .unwrap()
            .map_codepoint(codepoint)
            .map(|g| g.to_u16())
    }

    pub fn h_metrics(&self, gid: u16) -> HorizontalMetrics {
        let glyph_id = GlyphId::new(gid);
        if self.variable {
            let hvar = self.font.hvar().unwrap();
            HorizontalMetrics {
                advance: hvar
                    .advance_width_delta(glyph_id, self.axes.as_slice())
                    .unwrap()
                    .to_f64(),
                lsb: hvar
                    .advance_width_delta(glyph_id, self.axes.as_slice())
                    .unwrap()
                    .to_f64(),
            }
        } else {
            //can probably be made unsafe given all glyphs have metrics
            HorizontalMetrics::from(
                self.font
                    .hmtx()
                    .unwrap()
                    .h_metrics()
                    .get(glyph_id.to_u16() as usize)
                    .unwrap(),
            )
        }
    }
}

impl From<&LongMetric> for HorizontalMetrics {
    fn from(value: &LongMetric) -> Self {
        Self {
            advance: value.advance() as f64,
            lsb: value.side_bearing() as f64,
        }
    }
}

#[derive(Debug)]
pub struct HorizontalMetrics {
    pub advance: f64,
    pub lsb: f64,
}
