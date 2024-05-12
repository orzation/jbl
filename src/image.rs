use std::io::{stdout, Cursor, Write};

use image::{ImageBuffer, Rgb};

pub trait ImageHandler {
    fn print_out(&self);
}

/// Use the image crate to implement the CmdHandler trait
type Img = ImageBuffer<Rgb<u8>, Vec<u8>>;

impl ImageHandler for Img {
    fn print_out(&self) {
        let mut buffer = Cursor::new(Vec::new());
        self.write_to(&mut buffer, image::ImageFormat::Png).unwrap();
        stdout().write_all(buffer.get_ref()).unwrap();
    }
}
