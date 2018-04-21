/*!
Treesum calculates check|sums| for files in directory |tree|s. The successful
results are written to stdout while errors and status messages are written to
stderr. Example output:

```
01010101010101010101	root/some_subdir/cat.gif
12121212121212121212	root/I Like Space In My Paths/HerpDerp Final 3 (17).xlsx
```
*/
#![deny(warnings)]

extern crate sgiter;
extern crate sha1;
extern crate walkdir;

extern crate env_logger;
#[macro_use]
extern crate log;

use sha1::Sha1;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io;
use std::path::Path;
use walkdir::DirEntry;
use walkdir::WalkDir;

/// Calculate a checksum for a file.
fn calc_hash(
    p: &Path,
    hasher: &mut Sha1,
    buf: &mut [u8],
) -> io::Result<String> {
    hasher.reset();
    let mut f = File::open(p)?;
    loop {
        let num_read = f.read(buf)?;
        if num_read == 0 {
            break;
        }
        hasher.update(&buf[..num_read]);
    }
    Ok(hasher.digest().to_string())
}

fn print_success(t: &(DirEntry, String)) {
    println!("{}\t{}", t.1, t.0.path().display());
}

#[derive(Debug)]
enum Error {
    Io(io::Error),
    WalkDir(walkdir::Error),
}

type ResultIter = sgiter::GatherIter<Result<(DirEntry, String), Error>>;

/// Allocates memory for and collects all successfull hashes before sorting and
/// then printing. Errors are printed immediately.
fn do_sorted_output(res: ResultIter) {
    let mut tuples = vec![];
    for r in res {
        match r {
            Ok(t) => tuples.push(t),
            Err(e) => error!("{:?}", e)
        }
    }
    tuples.sort_by(|a, b| a.1.cmp(&b.1));
    for t in tuples {
        print_success(&t);
    }
}

/// Prints both errors and successful hashes immediately without sorting.
fn do_unsorted_output(res: ResultIter) {
    for r in res {
        match r {
            Ok(t) => print_success(&t),
            Err(e) => error!("{:?}", e)
        }
    }
}

/// Bind constructors for producer and consumers, start scatter gather and
/// pass on result iterator to handler for either sorted or unsorted output.
fn process_root(root: &Path) -> io::Result<()> {
    let pb = root.to_path_buf();
    let producer_ctor = || {
        WalkDir::new(pb).into_iter().filter(|r| match *r {
            Err(_) => true,
            Ok(ref r) => r.file_type().is_file(),
        })
    };
    let xform_ctor = || {
        let mut hasher = Sha1::new();
        let mut buf = vec![0u8; 1024 * 8];
        move |e: walkdir::Result<DirEntry>| {
            let e = e.map_err(Error::WalkDir)?;
            calc_hash(e.path(), &mut hasher, buf.as_mut_slice())
                .map_err(Error::Io)
                .map(|s| (e, s))
        }
    };
    let results = sgiter::scatter_gather(producer_ctor, xform_ctor);
    let sort_successes = false; // TODO: add command line flag
    if sort_successes {
        do_sorted_output(results)
    } else {
        do_unsorted_output(results)
    }
    Ok(())
}

fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let root = Path::new(root.as_str());
    process_root(root).unwrap()
}
