use std;
use std::path::{Path, PathBuf};

pub trait AbsolutePath {
    fn absolute_path(&self, canonicalize: bool) -> PathBuf;
    fn canonical_path(&self) -> PathBuf { self.absolute_path(true) }
}

impl AbsolutePath for Path {
    fn absolute_path(&self, canonicalize: bool) -> PathBuf {
        let mut absolute_path = std::env::current_dir().unwrap();
        absolute_path.push(self);

        if !canonicalize {
            absolute_path
        } else {
            let mut buf = PathBuf::new();

            for c in absolute_path.components() {
                let strref = c.as_ref();

                if strref == "." {
                    continue
                } else if strref == ".." {
                    buf.pop();
                } else {
                    buf.push(c.as_ref());
                }
            }

            buf
        }
    }
}

