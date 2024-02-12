use std::path::PathBuf;
use futures::executor;

use based::context::Context;
use shmontshmend::Frontend;

pub mod shmontshmend;

fn main() {
    let mut context = Context::default();
    let mut frontend = Frontend::default();
    executor::block_on(frontend.run(context));
}
