use std::path::PathBuf;

use based::context::Context;
use futures::executor;
use shmontshmend::Frontend;
use url::Url;

pub mod shmontshmend;

#[tokio::main]
async fn main() {
    let mut context = Context::new(Url::parse("https://itcorp.com").unwrap());
    let mut frontend = Frontend::default();
    frontend.run(context).await;
}
