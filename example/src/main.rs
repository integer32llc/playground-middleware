extern crate iron;
extern crate staticfile_adv;

use iron::prelude::*;
use staticfile_adv::Staticfile;

const ADDRESS: &'static str = "127.0.0.1:8000";

fn main() {
    let files = Staticfile::new("./").expect("Directory to serve not found");
    let _server = Iron::new(files).http(ADDRESS).expect("Unable to start server");
    println!("Server listening at {}", ADDRESS);
}
