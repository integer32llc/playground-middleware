extern crate env_logger;
extern crate iron;
extern crate staticfile_adv;

use iron::prelude::*;
use staticfile_adv::{Staticfile, Prefix, Cache};

const ADDRESS: &'static str = "127.0.0.1:8000";

fn main() {
    env_logger::init().expect("Unable to initialize logger");

    let files = Staticfile::new("./files").expect("Directory to serve not found");
    let mut files = Chain::new(files);

    let one_year = ::std::time::Duration::new(60 * 60* 24 * 365, 0);
    files.link_after(Prefix::new(&["assets"], Cache::new(one_year)));

    let _server = Iron::new(files).http(ADDRESS).expect("Unable to start server");
    println!("Server listening at {}", ADDRESS);
}
