#![allow(unknown_lints, dead_code, non_camel_case_types, expl_impl_clone_on_copy, 
         used_underscore_binding, non_snake_case, type_complexity, useless_transmute)]

mod libhif;

use std;
use std::str;
use std::ffi::{CStr, CString};

pub use self::libhif::{hif_context_new, hif_state_new, hif_context_setup_sack, 
             hif_context_setup, GError, hif_context_set_solv_dir, 
             hif_context_set_repo_dir, hif_context_set_lock_dir,
             hif_context_set_cache_dir, HifContext, hy_query_filter_latest_per_arch,
             hy_query_create, hy_query_filter, hy_query_run, HyQuery, GPtrArray,
             HifPackage, HifSack, hif_context_get_sack, hif_context_get_native_arches,
             hy_query_filter_in, hif_package_get_nevra, g_ptr_array_free, hy_query_free,
             hif_package_get_files};

pub use self::libhif::Enum__hy_comparison_type_e::*;
pub use self::libhif::Enum__hy_key_name_e::*;

struct PathFiles {
    path: String,
    files: Vec<String>
}

struct Package {
    nevra: String,
    files: Vec<PathFiles>,
}

//
// glib_check_call - macro
// 
// Call a function that accepts `GError` as its final argument
// and panic with error message if return value is false.
macro_rules! glib_check_call {
    ( $func:expr; $( $arg:expr ),+ ) => {{
        // 1+ argument expansion
        let mut err: *mut GError = std::ptr::null_mut();
        let r = $func($($arg,)* &mut err);

        if r == 0 {
            panic!(cstring_to_string((*err).message));
        }
    }};
    ( $func:expr ) => {{
        // 0 argument expansion
        let mut err: *mut GError = std::ptr::null_mut();
        let r = $func(&mut err);

        if r == 0 {
            panic!(cstring_to_string((*err).message));
        }
    }};
}

//
// g_ptr_array_iterate<T, F> - unsafe fn
//   T - Element type
//   F - Closure type
//
// Iterate over a `GPtrArray` and invoke `func` closure for each element `T`
pub unsafe fn g_ptr_array_iterate<T, F>(array: *mut GPtrArray, func: F) 
    where F : Fn(*mut T) {
    for i in 0..(*array).len  {
        func(*((*array).pdata.offset(i as isize) as *mut *mut T));
    }
}

//
// g_ptr_arrat_map_vector<T, F, R> - unsafe fn
//   T - Element type
//   F - Closure type
//   R - Return type
// 
// Iterate over a `GPtrArray` and map each element to a type `R` using `func`
// closure and append that element to a result vector which is returned in the end
pub unsafe fn g_ptr_array_map_vector<T, F, R>(array: *mut GPtrArray, func: F) -> Vec<R>
    where F : Fn(*mut T) -> R {
    let mut ret: Vec<R> = Vec::new();
    for i in 0..(*array).len  {
        ret.push(func(*((*array).pdata.offset(i as isize) as *mut *mut T)));
    }
    ret
}

//
// cstring_to_string - unsafe fn
//
// Convert a C string to a Rust's String
pub unsafe fn cstring_to_string(cstring: *const i8) -> String {
    let c_str: &CStr = CStr::from_ptr(cstring);
    let buf = c_str.to_bytes();

    str::from_utf8(buf).unwrap().to_owned()
}

//
// init_libhif - unsafe fn
//
// Initialize a libhif context using parameters `repos` and `caches` then return it
pub unsafe fn init_libhif(repos: &str, caches: &str) -> *mut HifContext {
    let context = hif_context_new();
    let state = hif_state_new();
    let repos_dir = CString::new(repos).unwrap();
    let cache_dir = CString::new(caches).unwrap();

    hif_context_set_repo_dir(context, repos_dir.as_ptr() as *const i8);
    hif_context_set_solv_dir(context, cache_dir.as_ptr() as *const i8);
    hif_context_set_lock_dir(context, cache_dir.as_ptr() as *const i8);
    hif_context_set_cache_dir(context, cache_dir.as_ptr() as *const i8);

    glib_check_call![hif_context_setup; context, std::ptr::null_mut()];
    glib_check_call![hif_context_setup_sack; context, state];

    context
}

/*
TODO: Make this work
pub unsafe fn dump_file_list(context: *mut HifContext) -> String {
    let sack: *mut HifSack = hif_context_get_sack(context);
    let query = hy_query_create(sack);
    let arches = hif_context_get_native_arches(context);

    hy_query_filter_latest_per_arch(query, 1);
    hy_query_filter_in(query, HY_PKG_ARCH as i32, HY_EQ as i32, arches);
    let pkglist: *mut GPtrArray = hy_query_run(query);
    let packages = g_ptr_array_map_vector(pkglist, |pkg: *mut HifPackage| {
        let files: *mut *mut std::os::raw::c_char = hif_package_get_files(pkg);
        let name = cstring_to_string(hif_package_get_nevra(pkg));
        let mut c1 = 0;

        loop {
            let fp = *files.offset(c1 as isize);
            if fp.is_null() {
                break;
            }
            println!("files: {}", cstring_to_string(fp));
            c1+=1;
        }
    });

    hy_query_free(query);

    "".to_string()
}
*/

//
// find_file - unsafe fn
//
// Finds packages that provide a file specified by `name` and returns them
pub unsafe fn find_file(context: *mut HifContext, name: &str) -> Vec<String> {
    let sack: *mut HifSack = hif_context_get_sack(context);
    let query = hy_query_create(sack);
    let arches = hif_context_get_native_arches(context);
    let filename = CString::new(name).unwrap();

    hy_query_filter_latest_per_arch(query, 1);
    // limiting to a single repository doesn't really help (1-2 sec slowdown)
    //hy_query_filter(query, HY_PKG_REPONAME as i32, HY_EQ as i32, "fedora".as_ptr() as *const i8);
    hy_query_filter_in(query, HY_PKG_ARCH as i32, HY_EQ as i32, arches);
    hy_query_filter(query, HY_PKG_FILE as i32, HY_EQ as i32, filename.as_ptr() as *const i8);
    let pkglist: *mut GPtrArray = hy_query_run(query);
    let packages = g_ptr_array_map_vector(pkglist, |pkg: *mut HifPackage| {
       cstring_to_string(hif_package_get_nevra(pkg))
    });

    // this causes weird SIGSEGV's but it seems correct to free the pkglist
    // so let's just leak a few bytes until I figure out why it fails
    //g_ptr_array_free(pkglist, 1);
    hy_query_free(query);

    packages
}

