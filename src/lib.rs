extern crate iron;
extern crate time;

use std::path::{Path, PathBuf};
use std::fs::{File, Metadata};

use iron::prelude::*;
use iron::middleware::Handler;
use iron::method::Method;
use iron::modifiers::Header;
use iron::headers::{HttpDate, LastModified, IfModifiedSince};
use iron::status;

pub struct Staticfile {
    root: PathBuf,
}

impl Staticfile {
    pub fn new<P>(root: P) -> Staticfile
        where P: AsRef<Path>
    {
        Staticfile {
            root: root.as_ref().into(),
        }
    }

    fn resolve_path(&self, path: &[&str]) -> PathBuf {
        // TODO: prevent escaping the root path via '..'
        let path = path.join("/");
        self.root.join(path)
    }
}

// TODO [TEST]: Returns "not allowed" for POST etc.

impl Handler for Staticfile {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.method {
            Method::Get => {},
            _ => return Ok(Response::with(status::MethodNotAllowed)),
        }

        let file_path = self.resolve_path(&req.url.path());

        println!("Accessing {}", file_path.display());

        let zeta = match Zeta::search(file_path) {
            Ok(zeta) => zeta,
            Err(_) => return Ok(Response::with(status::NotFound)),
        };

        let client_last_modified = req.headers.get::<IfModifiedSince>();
        let last_modified = zeta.last_modified().ok().map(HttpDate);

        if let (Some(client_last_modified), Some(last_modified)) = (client_last_modified, last_modified) {
            println!("Comparing {} <= {}", last_modified, client_last_modified.0);
            if last_modified <= client_last_modified.0 {
                return Ok(Response::with(status::NotModified));
            }
        }

        match last_modified {
            Some(last_modified) => {
                let last_modified = LastModified(last_modified);
                Ok(Response::with((status::Ok, Header(last_modified), zeta.file)))
            },
            None => Ok(Response::with((status::Ok, zeta.file)))
        }
    }
}

struct Zeta {
    file: File,
    metadata: Metadata,
}

impl Zeta {
    pub fn search<P>(path: P) -> Result<Zeta, Box<::std::error::Error>> // TODO: unbox
        where P: Into<PathBuf>
    {
        let mut file_path = path.into();
        let mut zeta = try!(Zeta::open(&file_path));

        // Look for index.html inside of a directory
        if zeta.metadata.is_dir() {
            file_path.push("index.html");
            zeta = try!(Zeta::open(&file_path));
        }

        assert!(zeta.metadata.is_file()); // TODO: Panicking

        Ok(zeta)
    }

    fn open<P>(path: P) -> Result<Zeta, Box<::std::error::Error>>
        where P: AsRef<Path>
    {
        let file = try!(File::open(path));
        let metadata = try!(file.metadata());

        Ok(Zeta {
            file: file,
            metadata: metadata,
        })
    }

    pub fn last_modified(&self) -> Result<time::Tm, Box<::std::error::Error>> {
        let modified = try!(self.metadata.modified());
        let since_epoch = try!(modified.duration_since(::std::time::UNIX_EPOCH));

        // TODO [TEST]: HTTP times don't have nanosec precision

        // Converting to i64 should be safe until we get beyond the
        // planned lifetime of the universe
        let ts = time::Timespec::new(since_epoch.as_secs() as i64, 0);
        Ok(time::at_utc(ts))
    }
}
