extern crate getopts;
extern crate walkdir;
extern crate regex;
extern crate yaml_rust;
extern crate threadpool;
extern crate core;

pub mod hif;
pub mod utils;

use threadpool::ThreadPool;
use regex::Regex;
use walkdir::WalkDir;
use getopts::Options;
use yaml_rust::YamlLoader;
use yaml_rust::yaml::Yaml;
use core::iter::FromIterator;

use std::io::Read;
use std::fs::File;
use std::{env, io};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::collections::HashSet;

use hif::HifContext;
use hif::{init_libhif, find_file}; // dump_file_list
use utils::AbsolutePath;

static VERSION: &'static str = "0.2.0";

type IoResult<T> = Result<T, io::Error>;

macro_rules! cloned {
    ( $( $x:expr ),* ) => {{
            ($($x.clone(),)*)
    }};
}

#[derive(Clone)]
struct FeadersFile {
    path: String,
    headers: Vec<String>,
}

#[allow(dead_code)]
struct Repository {
    name: String,
    version: String,
    arch: String,
}

#[allow(dead_code)]
struct Settings {
    ignored: HashSet<String>,
    repository: Repository,
    paths: Vec<String>,
}

#[derive(Clone)]
struct ImportPathFilters {
    absolute: HashSet<String>,
    relative: HashSet<String>,
}

struct FileSearcher;

impl FeadersFile {
    fn new(path: &str) -> FeadersFile {
        FeadersFile { path: path.to_string(), headers: Vec::new() }
    }

    fn process(&mut self, verbose: bool, dedup: bool, anchor: &str, filters: &ImportPathFilters) -> IoResult<usize> {
        if verbose {
            println!("processing: {}", self.path);
        }

        let path_anchor = Path::new(anchor).canonical_path();
        let include_matcher: Regex = Regex::new(r#"(?m:^\s*#\s*include\s*[<"]+(.*?)[>"]+)"#).unwrap();

        let mut f = try!(File::open(&self.path));
        let mut s = String::new();
        try!(f.read_to_string(&mut s));

        for capture in include_matcher.captures_iter(&s) {
            let include = capture.at(1).unwrap_or("").to_string();
            let include_path = Path::new(&include);
            let joined = path_anchor.join(&include_path).canonical_path();
            let absolute_path = joined.as_path().to_str().unwrap();

            if verbose {
                println!("found include: {}", include);
            }

            if dedup {
                if filters.absolute.contains(absolute_path) {
                    if verbose {
                        println!("skipping: {} found within the project", &absolute_path);
                    }

                    continue;
                }

                if filters.relative.contains(&include) {
                    if verbose {
                        println!("skipping: {} found within ignored", &absolute_path);
                    }

                    continue;
                }
            }

            self.headers.push(include.clone());
        }

        Ok(self.headers.len())
    }
}

impl FileSearcher {
    fn search(path: &str) -> IoResult<Vec<Arc<FeadersFile>>> {
        let suffixes: Vec<Regex> = vec![Regex::new(r"^.*\.h$").unwrap(), Regex::new(r"^.*\.c$").unwrap(), 
                                        Regex::new(r"^.*\.hpp$").unwrap(), Regex::new(r"^.*\.cc$").unwrap(), 
                                        Regex::new(r"^.*\.cpp$").unwrap()];

        let mut files: Vec<Arc<FeadersFile>> = Vec::new();
        for entry in WalkDir::new(path) {
            let file_entry = entry.unwrap();
            let file = file_entry.path().to_str().unwrap();

            for suffix in &suffixes {
                if suffix.is_match(file) {
                    let abs = Path::new(file).canonical_path();
                    files.push(Arc::new(FeadersFile::new(abs.as_path().to_str().unwrap())));
                    continue
                }
            }
        }

        Ok(files)
    }
}

fn usage(code: i32, program: &str, opts: &Options) {
    let top = format!("Usage: {} [options] PATH", program);
    print!("{}", opts.usage(&top));
    std::process::exit(code);
}

fn version(program: &str) {
    println!("{} - {}", program, &VERSION);
    std::process::exit(0);
}

fn unwrap_string(string: &Yaml) -> String {
    String::from(string.as_str().unwrap_or(""))
}

fn load_settings(file: &str) -> IoResult<Settings> {
    let mut f = try!(File::open(file));
    let mut s = String::new();
    try!(f.read_to_string(&mut s));

    let docs = YamlLoader::load_from_str(&s).unwrap();
    let doc = &docs[0];

    let ignored: HashSet<String> = doc["glibc"].as_vec()
                                    .unwrap()
                                    .iter()
                                    .map(|x| unwrap_string(x))
                                    .collect::<HashSet<_>>();

    let paths = doc["paths"].as_vec()
                 .unwrap()
                 .iter()
                 .map(|x| unwrap_string(x))
                 .collect::<Vec<_>>();

    Ok(Settings { ignored: ignored, 
                  repository: Repository {
                      name: unwrap_string(&doc["repository"]["title"]),
                      version: unwrap_string(&doc["repository"]["version"]),
                      arch: unwrap_string(&doc["repository"]["arch"]) },
                   paths: paths
    })
}

fn find_files(workers: usize, verbose: bool, deduplicate: bool, anchor: &Arc<String>, 
              filters: &Arc<ImportPathFilters>, files: &mut [Arc<FeadersFile>]) 
    -> mpsc::Receiver<Arc<FeadersFile>> {

    let pool = ThreadPool::new(workers);
    let (tx, rx) = mpsc::channel();

    for rf in files {
        let (mut file, path, filters, tx) = cloned![rf, anchor, filters, tx];

        pool.execute(move|| {
            match Arc::make_mut(&mut file).process(verbose, deduplicate, &path, &filters) {
                Ok(count) => {
                    if count > 0 {
                        tx.send(file).unwrap();
                    }
                },
                Err(e) => { panic!(e.to_string()) }
            }
        });
    }

    rx
}

fn find_packages(file: Arc<FeadersFile>, paths: &[String], searched: &mut HashSet<String>,
                 found: &mut HashSet<String>, context: *mut HifContext) 
    -> u32 {
    let mut queries = 0;
    for header in &file.headers {
        if !searched.insert(header.clone()) {
            continue;
        }

        for prefix in paths {
            let full_path = prefix.clone() + header;

            let packages = unsafe {
                find_file(context, full_path.as_str())
            };

            queries += 1;
            
            let mut skip = false;
            for pkg in packages {
                let pkgc = pkg.clone();
                if found.insert(pkg) {
                    println!("{}", pkgc);
                    skip = true;
                }
            }

            if skip {
                break;
            }
       }
    }

    queries
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].split('/').last().unwrap();

    let settings: Settings = match load_settings("config.yaml") {
        Ok(m) => { m }
        Err(e) => { panic!(e.to_string()) }
    };

    let hif_context = unsafe { 
        init_libhif("/etc/yum.repos.d", "/tmp/feaders")
    };

    //let packages = unsafe {
    //    dump_file_list(hif_context)
    //};

    let mut opts = Options::new();
    opts.optflag("h", "help", "prints this menu");
    opts.optflag("r", "repo", "repository to use for resolution");
    opts.optflag("v", "verbose", "verbose mode");
    opts.optflag("d", "deduplicate", "try to deduplicate headers");
    opts.optflag("", "version", "display version information");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(e) => { panic!(e.to_string()) }
    };

    if matches.opt_present("h") {
        usage(0, &program, &opts);
    }
    if matches.opt_present("version") {
        version(&program);
    }

    let dedup = matches.opt_present("d");
    let verbose = matches.opt_present("v");
    if matches.free.is_empty() {
        usage(-1, &program, &opts);
    } else {
        let path = Arc::new(matches.free[0].clone());
        let mut items = match FileSearcher::search(&path) {
            Ok(d) => d,
            Err(e) => { panic!(e.to_string()) }
        };

        let map = HashSet::from_iter(items.iter().map(|x| x.path.clone()));
        let filters = Arc::new(ImportPathFilters { absolute: map, 
                                                   relative: settings.ignored });

        let mut found = HashSet::new();
        let mut searched = HashSet::new();
        let mut queries = 0;
        let rx = find_files(16, verbose, dedup, &path, &filters, &mut items);

        while let Ok(i) = rx.recv() {
            queries += find_packages(i, &settings.paths, &mut searched, 
                                     &mut found, hif_context);
        }

        if verbose {
            println!("{} queries executed", queries);
        }
    }
}
