use iron::prelude::*;
use iron::AfterMiddleware;
use iron::headers::ContentType;

use mime::Mime;

/// Attempts to guess the content type of the response based on the
/// requested URL. Existing content types will not be modified.
pub struct GuessContentType {
    default: Mime,
}

impl GuessContentType {
    pub fn new(default: Mime) -> GuessContentType {
        GuessContentType {
            default: default,
        }
    }
}

impl Default for GuessContentType {
    fn default() -> GuessContentType {
        let default = "application/octet-stream".parse()
            .expect("Unable to create default MIME type");
        GuessContentType::new(default)
    }
}

impl AfterMiddleware for GuessContentType {
    fn after(&self, req: &mut Request, mut res: Response) -> IronResult<Response> {
        match res.headers.get::<ContentType>() {
            Some(_) => {},
            None => {
                let new_content_type = req.url.path().last()
                    .and_then(|filename| ::mime_guess::guess_mime_type_opt(filename))
                    .unwrap_or_else(|| self.default.clone());

                let header = ContentType(new_content_type);
                res.headers.set(header);
            }
        }
        Ok(res)
    }
}
