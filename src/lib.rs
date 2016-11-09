#[macro_use]
extern crate log;
extern crate iron;
extern crate time;
extern crate mime;
extern crate mime_guess;

mod cache;
mod guess_content_type;
mod modify_with;
mod prefix;
mod staticfile;

pub use cache::Cache;
pub use guess_content_type::GuessContentType;
pub use modify_with::ModifyWith;
pub use prefix::Prefix;
pub use staticfile::Staticfile;
