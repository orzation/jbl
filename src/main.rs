use cli::{Cmd, CmdHandler};
use image::ImageHandler;
use render::FontRenderHandler;

mod cli;
mod image;
mod render;

fn main() {
    let ver: &str = env!("CARGO_PKG_VERSION");
    Cmd::new_command(ver).into_metrics().render().print_out()
}
