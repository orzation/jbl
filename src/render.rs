use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Shaping, SwashCache};
use image::{ImageBuffer, Rgb};
use regex::Regex;

pub trait FontRenderHandler {
    fn render(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>>;
}

const ANSI_COLORS: [(u8, u8, u8); 8] = [
    (0, 0, 0),       // 30 black
    (205, 49, 49),   // 31 red
    (13, 188, 121),  // 32 green
    (229, 229, 16),  // 33 yellow
    (36, 114, 200),  // 34 blue
    (188, 63, 188),  // 35 magenta
    (17, 168, 205),  // 36 cyan
    (229, 229, 229), // 37 white
];

const ANSI_BRIGHT_COLORS: [(u8, u8, u8); 8] = [
    (85, 85, 85),    // 90 bright black
    (255, 85, 85),   // 91 bright red
    (85, 255, 85),   // 92 bright green
    (255, 255, 85),  // 93 bright yellow
    (85, 85, 255),   // 94 bright blue
    (255, 85, 255),  // 95 bright magenta
    (85, 255, 255),  // 96 bright cyan
    (255, 255, 255), // 97 bright white
];

fn ansi_256_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=7 => ANSI_COLORS[index as usize],
        8..=15 => ANSI_BRIGHT_COLORS[(index - 8) as usize],
        16..=231 => {
            let i = index - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let vals = [0u8, 95, 135, 175, 215, 255];
            (vals[r as usize], vals[g as usize], vals[b as usize])
        }
        232..=255 => {
            let gray = index - 232;
            let v = 8 + gray * 10;
            (v, v, v)
        }
    }
}

fn parse_sgr(params: &str, default_fg: (u8, u8, u8)) -> (Color, Option<Color>) {
    let mut fg = Color::rgb(default_fg.0, default_fg.1, default_fg.2);
    let mut bg: Option<Color> = None;
    let parts: Vec<&str> = params.split(';').collect();
    let mut i = 0;
    while i < parts.len() {
        let code: u8 = if parts[i].is_empty() {
            0
        } else {
            parts[i].parse().unwrap_or(0)
        };
        match code {
            0 => {
                fg = Color::rgb(default_fg.0, default_fg.1, default_fg.2);
                bg = None;
            }
            30..=37 => {
                let c = ANSI_COLORS[(code - 30) as usize];
                fg = Color::rgb(c.0, c.1, c.2);
            }
            38 if i + 2 < parts.len() && parts[i + 1] == "5" => {
                if let Ok(idx) = parts[i + 2].parse::<u8>() {
                    let c = ansi_256_to_rgb(idx);
                    fg = Color::rgb(c.0, c.1, c.2);
                }
                i += 2;
            }
            38 if i + 4 < parts.len() && parts[i + 1] == "2" => {
                if let (Ok(r), Ok(g), Ok(b)) =
                    (parts[i + 2].parse(), parts[i + 3].parse(), parts[i + 4].parse())
                {
                    fg = Color::rgb(r, g, b);
                }
                i += 4;
            }
            39 => fg = Color::rgb(default_fg.0, default_fg.1, default_fg.2),
            40..=47 => {
                let c = ANSI_COLORS[(code - 40) as usize];
                bg = Some(Color::rgb(c.0, c.1, c.2));
            }
            48 if i + 2 < parts.len() && parts[i + 1] == "5" => {
                if let Ok(idx) = parts[i + 2].parse::<u8>() {
                    let c = ansi_256_to_rgb(idx);
                    bg = Some(Color::rgb(c.0, c.1, c.2));
                }
                i += 2;
            }
            48 if i + 4 < parts.len() && parts[i + 1] == "2" => {
                if let (Ok(r), Ok(g), Ok(b)) =
                    (parts[i + 2].parse(), parts[i + 3].parse(), parts[i + 4].parse())
                {
                    bg = Some(Color::rgb(r, g, b));
                }
                i += 4;
            }
            49 => bg = None,
            90..=97 => {
                let c = ANSI_BRIGHT_COLORS[(code - 90) as usize];
                fg = Color::rgb(c.0, c.1, c.2);
            }
            100..=107 => {
                let c = ANSI_BRIGHT_COLORS[(code - 100) as usize];
                bg = Some(Color::rgb(c.0, c.1, c.2));
            }
            _ => {}
        }
        i += 1;
    }
    (fg, bg)
}

#[derive(Debug, Clone, Copy)]
struct TextSpan<'a> {
    text: &'a str,
    fg: Color,
    bg: Option<Color>,
    end: usize,
}

fn parse_ansi<'a>(text: &'a str, default_fg: (u8, u8, u8)) -> Vec<TextSpan<'a>> {
    let mut spans = Vec::new();
    let mut current_fg = Color::rgb(default_fg.0, default_fg.1, default_fg.2);
    let mut current_bg: Option<Color> = None;
    let mut remaining = text;
    let mut offset = 0usize;

    while !remaining.is_empty() {
        if let Some(esc_pos) = remaining.find('\x1b') {
            if esc_pos > 0 {
                spans.push(TextSpan {
                    text: &remaining[..esc_pos],
                    fg: current_fg,
                    bg: current_bg,
                    end: offset + esc_pos,
                });
                offset += esc_pos;
            }

            remaining = &remaining[esc_pos..];

            if remaining.len() >= 2 && remaining.as_bytes()[1] == b'[' {
                remaining = &remaining[2..];

                let mut params_end = 0;
                while params_end < remaining.len() {
                    let b = remaining.as_bytes()[params_end];
                    if b.is_ascii_digit() || b == b';' {
                        params_end += 1;
                    } else {
                        break;
                    }
                }

                if params_end < remaining.len() {
                    let cmd = remaining.as_bytes()[params_end];
                    if cmd == b'm' {
                        let params = &remaining[..params_end];
                        let (fg, bg) = parse_sgr(params, default_fg);
                        current_fg = fg;
                        current_bg = bg;
                    }
                    remaining = &remaining[params_end + 1..];
                } else {
                    spans.push(TextSpan {
                        text: remaining,
                        fg: current_fg,
                        bg: current_bg,
                        end: offset + remaining.len(),
                    });
                    break;
                }
            } else if remaining.len() >= 2 {
                remaining = &remaining[2..];
            } else {
                spans.push(TextSpan {
                    text: remaining,
                    fg: current_fg,
                    bg: current_bg,
                    end: offset + remaining.len(),
                });
                break;
            }
        } else {
            spans.push(TextSpan {
                text: remaining,
                fg: current_fg,
                bg: current_bg,
                end: offset + remaining.len(),
            });
            break;
        }
    }

    spans
}

#[derive(Debug, PartialEq)]
pub struct Metrics<'a> {
    pub text: String,
    pub font: Family<'a>,
    pub size: f32,
    pub color: (u8, u8, u8),
    pub bg_color: (u8, u8, u8),
    pub padding: u8,
    pub width: Option<f32>,
}

impl FontRenderHandler for Metrics<'_> {
    fn render(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        let font_size: f32 = self.size;
        let line_height: f32 = self.size * 1.2;

        let carrige_pos: Vec<(usize, char)> = self
            .text
            .chars()
            .enumerate()
            .filter(|(_, c)| *c == '\n')
            .collect();
        let raw_line_number = carrige_pos.len() + 1;
        let max_line_length = match raw_line_number {
            1 => self.text.len(),
            _ => {
                carrige_pos
                    .iter()
                    .fold((0, 0), |(len, pre), (idx, _)| {
                        (idx.saturating_sub(pre).saturating_sub(1).max(len), *idx)
                    })
                    .0
            }
        };

        let render_width = self
            .width
            .unwrap_or(font_size * max_line_length as f32)
            .max(1.0);

        let metrics = cosmic_text::Metrics::new(font_size, line_height);

        // stage1: layout text with unbounded height to compute wrapped lines
        let mut buffer = Buffer::new(&mut font_system, metrics);
        let mut buffer = buffer.borrow_with(&mut font_system);
        buffer.set_size(render_width, 1e6);

        let default_attrs =
            Attrs::new().family(self.font).color(Color::rgb(self.color.0, self.color.1, self.color.2));
        let spans = parse_ansi(&self.text, self.color);
        let rich_spans: Vec<(&str, Attrs)> = spans
            .iter()
            .map(|span| (span.text, default_attrs.color(span.fg)))
            .collect();
        buffer.set_rich_text(rich_spans, default_attrs, Shaping::Advanced);

        let (max_width, actual_line_count) = {
            let mut max_w = 0.0f32;
            let mut count = 0;
            for run in buffer.layout_runs() {
                max_w = max_w.max(run.line_w);
                count += 1;
            }
            (max_w, count.max(1))
        };
        let render_height = line_height * actual_line_count as f32;

        // stage2: set precise size for final rendering
        // Add 1.0px to render_height to counteract floating point precision loss
        // in cosmic-text's visible_lines() = (height / line_height) as i32,
        // which can truncate (e.g. 6.999999 → 6) and cause the last line to be cropped.
        buffer.set_size(render_width, render_height + 1.0);

        // stage3: draw the image
        let img_width = if self.width.is_some() {
            render_width
        } else {
            max_width
        } as u32;
        let img_height = render_height as u32;
        let mut img_buf: ImageBuffer<Rgb<u8>, Vec<_>> = ImageBuffer::new(
            img_width + self.padding as u32 * 2,
            img_height + self.padding as u32 * 2,
        );

        // a. draw the background
        for pixel in img_buf.pixels_mut() {
            *pixel = image::Rgb([self.bg_color.0, self.bg_color.1, self.bg_color.2]);
        }

        // b. draw per-glyph background colors from ANSI spans
        let mut pure_line_start_offsets = vec![0usize];
        let mut pure_pos = 0;
        for span in &spans {
            for c in span.text.bytes() {
                if c == b'\n' {
                    pure_line_start_offsets.push(pure_pos + 1);
                }
                pure_pos += 1;
            }
        }

        let mut span_idx = 0;
        for run in buffer.layout_runs() {
            let line_start = pure_line_start_offsets.get(run.line_i).copied().unwrap_or(0);
            for glyph in run.glyphs {
                let glyph_start_global = line_start + glyph.start;
                while span_idx < spans.len() && spans[span_idx].end <= glyph_start_global {
                    span_idx += 1;
                }
                if let Some(span) = spans.get(span_idx) {
                    if let Some(bg_color) = span.bg {
                        let x = glyph.x.floor() as i32;
                        let y = run.line_top.floor() as i32;
                        let w = glyph.w.ceil() as u32;
                        let h = line_height.ceil() as u32;

                        let x0 = x.max(0) as u32;
                        let y0 = y.max(0) as u32;
                        let x1 = (x + w as i32).min(img_width as i32) as u32;
                        let y1 = (y + h as i32).min(img_height as i32) as u32;

                        if x0 < x1 && y0 < y1 {
                            let pad = self.padding as u32;
                            let stride = img_buf.width() as usize * 3;
                            let r = bg_color.r();
                            let g = bg_color.g();
                            let b = bg_color.b();
                            let raw = img_buf.as_mut();
                            for row in (y0 + pad)..(y1 + pad) {
                                let row_start = row as usize * stride + (x0 + pad) as usize * 3;
                                let row_len = (x1 - x0) as usize * 3;
                                let row_slice = &mut raw[row_start..row_start + row_len];
                                for chunk in row_slice.chunks_exact_mut(3) {
                                    chunk[0] = r;
                                    chunk[1] = g;
                                    chunk[2] = b;
                                }
                            }
                        }
                    }
                }
            }
        }

        // c. draw the text
        let fallback_color = Color::rgb(self.color.0, self.color.1, self.color.2);
        buffer.draw(&mut swash_cache, fallback_color, |x, y, w, h, color| {
            let a = color.a();
            if a == 0
                || x < 0
                || x >= img_width as i32
                || y < 0
                || y >= img_height as i32
                || w != 1
                || h != 1
            {
                return;
            }

            let px_x = x as u32 + self.padding as u32;
            let px_y = y as u32 + self.padding as u32;
            let existing = img_buf.get_pixel(px_x, px_y);
            let alpha = a as i32;
            let inv = 255 - alpha;
            let blend = |fg: u8, bg: u8| ((fg as i32 * alpha + bg as i32 * inv) / 255).clamp(0, 255) as u8;
            img_buf.put_pixel(
                px_x,
                px_y,
                Rgb([blend(color.r(), existing[0]), blend(color.g(), existing[1]), blend(color.b(), existing[2])]),
            );
        });
        img_buf
    }
}

impl Metrics<'_> {
    pub fn new<'a>(
        text: String,
        font: &'a String,
        size: f32,
        color: &'a String,
        bg_color: &'a String,
        padding: u8,
        width: Option<f32>,
    ) -> Result<Metrics<'a>, String> {
        let font = match font.as_str() {
            "Serif" => Family::Serif,
            "SansSerif" => Family::SansSerif,
            "Cursive" => Family::Cursive,
            "Fantasy" => Family::Fantasy,
            "Monospace" => Family::Monospace,
            str => Family::Name(str),
        };

        let hex_color_regex = Regex::new(r#"^#([a-fA-F0-9]{6}|[a-fA-F0-9]{3})$"#).unwrap();
        if !hex_color_regex.is_match(&color) || !hex_color_regex.is_match(&bg_color) {
            return Err("The color input must be in a legal hexadecimal format!".to_string());
        }

        let color = hex_to_rgb(&color);
        let bg_color = hex_to_rgb(&bg_color);

        Ok(Metrics {
            text,
            font,
            size,
            color,
            bg_color,
            padding,
            width,
        })
    }
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = &hex[1..];
    let hex = match hex.len() {
        3 => hex
            .chars()
            .flat_map(|c| std::iter::repeat(c).take(2))
            .collect::<String>(),
        _ => hex.to_string(),
    };
    let rgb = u32::from_str_radix(&hex, 16).unwrap();
    (
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgb_normal() {
        assert_eq!((75, 0, 130), hex_to_rgb("#4B0082"));
    }

    #[test]
    fn hex_to_rgb_three() {
        assert_eq!((255, 255, 255), hex_to_rgb("#fff"));
        assert_eq!((255, 255, 255), hex_to_rgb("#fFf"));
        assert_eq!((0, 0, 0), hex_to_rgb("#000"));
    }

    #[test]
    fn new_metrics_font() {
        let cases = [
            ("Serif", Family::Serif),
            ("SansSerif", Family::SansSerif),
            ("Cursive", Family::Cursive),
            ("Fantasy", Family::Fantasy),
            ("Monospace", Family::Monospace),
            ("Cascadia Mono", Family::Name("Cascadia Mono")),
        ];

        cases.iter().for_each(|c| {
            assert_eq!(
                Metrics {
                    text: "".to_string(),
                    font: c.1,
                    size: 16.0,
                    color: (255, 255, 255),
                    bg_color: (0, 0, 0),
                    padding: 8,
                    width: None,
                },
                Metrics::new(
                "".to_string(),
                &c.0.to_string(),
                16.0,
                &"#FFF".to_string(),
                &"#000".to_string(),
                8,
                None,
            ).expect("new metrics error")
            )
        });
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_color_short() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#0".to_string(),
            &"#000".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_color_miss_hash() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"000".to_string(),
            &"#000".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_color_char_range() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#qw12!@".to_string(),
            &"#000".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_color_length() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#ffffff99".to_string(),
            &"#000".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_bgcolor_short() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#000".to_string(),
            &"#0".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_bgcolor_miss_hash() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#000".to_string(),
            &"000".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_bgcolor_char_range() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#000".to_string(),
            &"#qw12!@".to_string(),
            8,
            None,
        ).unwrap();
    }

    #[test]
    #[should_panic(expected = "The color input must be in a legal hexadecimal format!")]
    fn new_metrics_invaild_bgcolor_length() {
        Metrics::new(
            "".to_string(),
            &"Monospace".to_string(),
            16.0,
            &"#000".to_string(),
            &"#ffffff99".to_string(),
            8,
            None,
        ).unwrap();
    }
}
