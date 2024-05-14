use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Shaping, SwashCache};
use image::{ImageBuffer, Rgb};
use regex::Regex;

pub trait FontRenderHandler {
    fn render(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>>;
}

#[derive(Debug, PartialEq)]
pub struct Metrics<'a> {
    pub text: String,
    pub font: Family<'a>,
    pub size: f32,
    pub color: (u8, u8, u8),
    pub bg_color: (u8, u8, u8),
    pub padding: u8,
}

impl FontRenderHandler for Metrics<'_> {
    fn render(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        // stage1: layout all text
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        let carrige_pos: Vec<(usize, char)> = self
            .text
            .chars()
            .enumerate()
            .filter(|(_, c)| *c == '\n')
            .collect();
        let line_number = carrige_pos.len() + 1;
        let max_line_length = match line_number {
            1 => self.text.len(),
            _ => {
                carrige_pos
                    .iter()
                    .fold((0, 0), |(len, pre), (idx, _)| {
                        ((*idx - pre - 1).max(len), *idx)
                    })
                    .0
            }
        };

        let font_size: f32 = self.size;
        let line_height: f32 = self.size * 1.2;

        let render_width = font_size * max_line_length as f32;
        let render_height = line_height * line_number as f32;

        let metrics = cosmic_text::Metrics::new(font_size, line_height);

        // stage2: render text
        let mut buffer = Buffer::new(&mut font_system, metrics);
        let mut buffer = buffer.borrow_with(&mut font_system);
        buffer.set_size(render_width, render_height);

        let attrs = Attrs::new().family(self.font);
        buffer.set_text(&self.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(true);

        // stage3: draw the image
        let max_width = buffer
            .layout_runs()
            .fold(0.0, |width, run| run.line_w.max(width)) as u32;
        let max_height = render_height as u32;
        let mut img_buf: ImageBuffer<Rgb<u8>, Vec<_>> = ImageBuffer::new(
            max_width + self.padding as u32 * 2,
            max_height + self.padding as u32 * 2,
        );

        // a. draw the background
        for pixel in img_buf.pixels_mut() {
            *pixel = image::Rgb([self.bg_color.0, self.bg_color.1, self.bg_color.2]);
        }

        // b. draw the text
        let text_color: Color = Color::rgb(self.color.0, self.color.1, self.color.2);
        buffer.draw(&mut swash_cache, text_color, |x, y, w, h, color| {
            let a = color.a();
            if a == 0
                || x < 0
                || x >= max_width as i32
                || y < 0
                || y >= max_height as i32
                || w != 1
                || h != 1
            {
                // Ignore alphas of 0, or invalid x, y coordinates, or unimplemented sizes
                return;
            }

            // Scale by alpha (mimics blending with black)
            let scale = |c: u8| (c as i32 * a as i32 / 255).clamp(0, 255) as u8;

            let r = scale(color.r());
            let g = scale(color.g());
            let b = scale(color.b());
            img_buf.put_pixel(
                x as u32 + self.padding as u32,
                y as u32 + self.padding as u32,
                Rgb([r, g, b]),
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
    ) -> Metrics<'a> {
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
            panic!("The color input must be in a legal hexadecimal format!")
        }

        let color = hex_to_rgb(&color);
        let bg_color = hex_to_rgb(&bg_color);

        Metrics {
            text,
            font,
            size,
            color,
            bg_color,
            padding,
        }
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
                    padding: 8
                },
                Metrics::new(
                    "".to_string(),
                    &c.0.to_string(),
                    16.0,
                    &"#FFF".to_string(),
                    &"#000".to_string(),
                    8,
                )
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
        );
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
        );
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
        );
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
        );
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
        );
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
        );
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
        );
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
        );
    }
}
