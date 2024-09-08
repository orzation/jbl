use cli::{Cmd, CmdHandler};
use image::ImageHandler;
use render::FontRenderHandler;

mod cli;
mod image;
mod render;

fn main() {
    let ver: &str = env!("CARGO_PKG_VERSION");
    let cmd = Cmd::new_command(ver);

    let metrics = match cmd.into_metrics() {
       Ok(result)  =>  result,
       Err(e) => {
        eprintln!("Something Wrong: {}", e);
        std::process::exit(1);
       }
    };

    metrics.render().print_out()
}
