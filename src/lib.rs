#[macro_use]
extern crate log;
extern crate iron;
extern crate time;

use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use std::{error, io};

use iron::headers::{HttpDate, LastModified, IfModifiedSince};
use iron::method::Method;
use iron::middleware::Handler;
use iron::modifiers::Header;
use iron::prelude::*;
use iron::status;

mod prefix;
mod cache;
mod modify_with;

pub use prefix::Prefix;
pub use cache::Cache;
pub use modify_with::ModifyWith;

/// Recursively serves files from the specified root directory.
pub struct Staticfile {
    root: PathBuf,
}

impl Staticfile {
    pub fn new<P>(root: P) -> io::Result<Staticfile>
        where P: AsRef<Path>
    {
        let root = try!(root.as_ref().canonicalize());

        Ok(Staticfile {
            root: root,
        })
    }

    fn resolve_path(&self, path: &[&str]) -> Result<PathBuf, Box<error::Error>> {
        let mut resolved = self.root.clone();

        for component in path {
            resolved.push(component);
        }

        let resolved = try!(resolved.canonicalize());

        // Protect against path/directory traversal
        if !resolved.starts_with(&self.root) {
            Err(From::from("Cannot leave root path"))
        } else {
            Ok(resolved)
        }
    }
}

impl Handler for Staticfile {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.method {
            Method::Get => {},
            _ => return Ok(Response::with(status::MethodNotAllowed)),
        }

        let file_path = match self.resolve_path(&req.url.path()) {
            Ok(file_path) => file_path,
            Err(_) => return Ok(Response::with(status::NotFound)),
        };

        let file = match StaticFileWithMetadata::search(file_path) {
            Ok(file) => file,
            Err(_) => return Ok(Response::with(status::NotFound)),
        };

        let client_last_modified = req.headers.get::<IfModifiedSince>();
        let last_modified = file.last_modified().ok().map(HttpDate);

        if let (Some(client_last_modified), Some(last_modified)) = (client_last_modified, last_modified) {
            trace!("Comparing {} (file) <= {} (req)", last_modified, client_last_modified.0);
            if last_modified <= client_last_modified.0 {
                return Ok(Response::with(status::NotModified));
            }
        }

        match last_modified {
            Some(last_modified) => {
                let last_modified = LastModified(last_modified);
                Ok(Response::with((status::Ok, Header(last_modified), file.file)))
            },
            None => Ok(Response::with((status::Ok, file.file)))
        }
    }
}

struct StaticFileWithMetadata {
    file: File,
    metadata: Metadata,
}

impl StaticFileWithMetadata {
    pub fn search<P>(path: P) -> Result<StaticFileWithMetadata, Box<error::Error>> // TODO: unbox
        where P: Into<PathBuf>
    {
        let mut file_path = path.into();
        trace!("Opening {}", file_path.display());
        let mut file = try!(StaticFileWithMetadata::open(&file_path));

        // Look for index.html inside of a directory
        if file.metadata.is_dir() {
            file_path.push("index.html");
            trace!("Redirecting to index {}", file_path.display());
            file = try!(StaticFileWithMetadata::open(&file_path));
        }

        if file.metadata.is_file() {
            Ok(file)
        } else {
            Err(From::from("Requested path was not a regular file"))
        }
    }

    fn open<P>(path: P) -> Result<StaticFileWithMetadata, Box<error::Error>>
        where P: AsRef<Path>
    {
        let file = try!(File::open(path));
        let metadata = try!(file.metadata());

        Ok(StaticFileWithMetadata {
            file: file,
            metadata: metadata,
        })
    }

    pub fn last_modified(&self) -> Result<time::Tm, Box<error::Error>> {
        let modified = try!(self.metadata.modified());
        let since_epoch = try!(modified.duration_since(UNIX_EPOCH));

        // HTTP times don't have nanosecond precision, so we truncate
        // the modification time.
        // Converting to i64 should be safe until we get beyond the
        // planned lifetime of the universe
        //
        // TODO: Investigate how to write a test for this. Changing
        // the modification time of a file with greater than second
        // precision appears to be something that only is possible to
        // do on Linux.
        let ts = time::Timespec::new(since_epoch.as_secs() as i64, 0);
        Ok(time::at_utc(ts))
    }
}

#[cfg(test)]
mod test {
    extern crate iron_test;
    extern crate hyper;
    extern crate tempdir;

    use super::*;

    use std::path::{Path, PathBuf};
    use std::fs::{File, DirBuilder};

    use ::iron::status;
    use self::iron_test::request;
    use self::hyper::header::Headers;
    use self::tempdir::TempDir;

    struct TestFilesystemSetup(TempDir);

    impl TestFilesystemSetup {
        fn new() -> Self {
            TestFilesystemSetup(TempDir::new("test").expect("Could not create test directory"))
        }

        fn path(&self) -> &Path {
            self.0.path()
        }

        fn dir(&self, name: &str) -> PathBuf {
            let p = self.path().join(name);
            DirBuilder::new().recursive(true).create(&p).expect("Could not create directory");
            p
        }

        fn file(&self, name: &str) -> PathBuf {
            let p = self.path().join(name);
            File::create(&p).expect("Could not create file");
            p
        }
    }

    #[test]
    fn staticfile_resolves_paths() {
        let fs = TestFilesystemSetup::new();
        fs.file("index.html");

        let sf = Staticfile::new(fs.path()).unwrap();
        let path = sf.resolve_path(&["index.html"]);
        assert!(path.unwrap().ends_with("index.html"));
    }

    #[test]
    fn staticfile_resolves_nested_paths() {
        let fs = TestFilesystemSetup::new();
        fs.dir("dir");
        fs.file("dir/index.html");

        let sf = Staticfile::new(fs.path()).unwrap();
        let path = sf.resolve_path(&["dir", "index.html"]);
        assert!(path.unwrap().ends_with("dir/index.html"));
    }

    #[test]
    fn staticfile_disallows_resolving_out_of_root() {
        let fs = TestFilesystemSetup::new();
        fs.file("naughty.txt");
        let dir = fs.dir("dir");

        let sf = Staticfile::new(dir).unwrap();
        let path = sf.resolve_path(&["..", "naughty.txt"]);
        assert!(path.is_err());
    }

    #[test]
    fn staticfile_disallows_post_requests() {
        let fs = TestFilesystemSetup::new();
        let sf = Staticfile::new(fs.path()).unwrap();

        let response = request::post("http://127.0.0.1/", Headers::new(), "", &sf);

        let response = response.expect("Response was an error");
        assert_eq!(response.status, Some(status::MethodNotAllowed));
    }
}
