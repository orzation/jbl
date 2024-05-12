use std::{
    fs::File,
    io::{self, Read},
};

use crate::render::Metrics;
use clap::{arg, value_parser, ArgMatches, Command};

pub trait CmdHandler<'a> {
    fn new_command(ver: &'static str) -> Self;
    fn into_metrics(&'a self) -> Metrics<'a>;
}

/// Use the clap crate to implement the CmdHandler trait
pub type Cmd = ArgMatches;

impl CmdHandler<'_> for Cmd {
    fn new_command(ver: &'static str) -> Self {
        Command::new("JingleBell")
            .bin_name("jbl")
            .version(ver)
            .author("msqtt")
            .about("A simple tool to turn unicode chars into a png image.")
            .arg(
                arg!(-f --font <FONT_NAME> "Set the font family used to draw image")
                    .required(false)
                    .default_value("Monospace")
            )
            .arg(
                arg!(-s --size <VALUE> "Set the font size used to draw image")
                    .value_parser(value_parser!(f32))
                    .required(false)
                    .default_value("18.0")
            )
            .arg(
                arg!(-c --color <COLOR> "Set the color of the font (Only hexadecimal RGB color codes)")
                    .required(false)
                    .default_value("#cdd6f4")
            )
            .arg(
                arg!(-b --"background-color" <COLOR> "Set the color of the background (Only hexadecimal RGB color codes)")
                    .required(false)
                    .default_value("#1e1e2e")
            )
            .arg(
                arg!(-p --padding <VALUE> "Set the padding of the image")
                    .value_parser(value_parser!(u8))
                    .required(false)
                    .default_value("8")
            )
            .arg(
                arg!([FILE] "Set the the text file to read. With no FILE, or when FILE is -, read standard input.")
                .required(false)
                .default_value("-")
            )
            .get_matches()
    }

    fn into_metrics<'a>(&'a self) -> Metrics<'a> {
        let font = self.get_one::<String>("font").unwrap();
        let size = self.get_one::<f32>("size").unwrap();
        let color = self.get_one::<String>("color").unwrap();
        let bg_color = self.get_one::<String>("background-color").unwrap();
        let padding = self.get_one::<u8>("padding").unwrap();
        let file = self.get_one::<String>("FILE").unwrap();

        let mut text_buf = String::new();
        if file != "-" {
            File::open(file)
                .expect("Failed to open file")
                .read_to_string(&mut text_buf)
                .expect("Failed to read file");
        } else {
            // no input file, just block to read std input
            io::stdin()
                .lock()
                .read_to_string(&mut text_buf)
                .expect("Failed to read std input");
        }

        Metrics::new(text_buf, font, *size, color, bg_color, *padding)
    }
}
