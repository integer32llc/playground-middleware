use std::time::Duration;
use std::{cmp, u32};

use iron::headers::{CacheDirective, CacheControl};
use iron::modifier::Modifier;
use iron::modifiers::Header;
use iron::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Cache(u32);

impl Cache {
    pub fn new(duration: Duration) -> Cache {
        // Capping the value at ~136 years!
        let duration = cmp::min(duration.as_secs(), u32::MAX as u64) as u32;

        Cache(duration)
    }
}

impl Modifier<Response> for Cache {
    fn modify(self, response: &mut Response) {
        Header(CacheControl(vec![
            CacheDirective::Public,
            CacheDirective::MaxAge(self.0),
        ])).modify(response)
    }
}
