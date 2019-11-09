#![feature(proc_macro_hygiene, decl_macro)]

extern crate regex;
extern crate futures;

#[macro_use]
extern crate rocket;

extern crate toml;
#[macro_use]
extern crate serde_derive;
extern crate s3;

extern crate url;

use std::sync::{Arc,Mutex};
use std::{env, process};
use std::error::Error;
use std::path::PathBuf;

use futures::future;
use rocket::State;
use rocket::response::{Response, Redirect};
use rocket::request::Request;

mod routecache;

#[catch(404)]
fn sys_not_found(req: &Request) -> String {
    format!("Sorry, that's not a valid path!")
}

#[get("/healthz")]
fn sys_health(cachedb: State<Arc<Mutex<routecache::RouteCache>>>) -> String {
    format!("{{\"status\": \"OK\"}}").to_string()
}

#[get("/")]
fn index(cachedb: State<Arc<Mutex<routecache::RouteCache>>>) -> String {
    let mut cache = cachedb.lock().unwrap();
    cache.sync();
    let branches = cache.branches();
    format!("{:?}", branches).to_string()
}

#[get("/<branch>")]
fn index_branch(cachedb: State<Arc<Mutex<routecache::RouteCache>>>, branch: String) -> String {
    let mut cache = cachedb.lock().unwrap();
    cache.sync();
    let arches = cache.architectures(branch);
    format!("{:?}", arches).to_string()
}

#[get("/<branch>/<arch>")]
fn index_arch(cachedb: State<Arc<Mutex<routecache::RouteCache>>>, branch: String, arch: String) -> String {
    let mut cache = cachedb.lock().unwrap();
    cache.sync();
    let versions = cache.versions(branch, arch);
    format!("{:?}", versions).to_string()
}

#[get("/<branch>/<arch>/current/<path..>")]
fn index_current(cachedb: State<Arc<Mutex<routecache::RouteCache>>>, branch: String, arch: String, path: PathBuf) -> Redirect {
    let mut cache = cachedb.lock().unwrap();
    cache.sync();

    let prefix_url = cache.public_prefix().unwrap();
    let latest = cache.latest_version(branch.clone(), arch.clone()).unwrap();
    let repo_file = path.to_str().unwrap();
    let final_url = prefix_url.join(format!("{}/{}/{}/{}", branch, arch, latest.version, repo_file).as_str()).unwrap();
    Redirect::to(final_url.to_string())
}

fn main() {
    let config = match routecache::RouteConfig::new_from_env() {
        Ok(c) => c,
        Err(e) => {
            println!("Error: {}", e);
            process::exit(1);
        },
    };
    let mut cache = routecache::RouteCache::new(config);
    match cache.sync() {
        Ok(_) => {},
        Err(e) => println!("Cache Sync Error: {}", e),
    };

    rocket::ignite()
        .manage(Arc::new(Mutex::new(cache)))
        .mount("/", routes![sys_health, index, index_branch, index_arch, index_current])
        .register(catchers![sys_not_found])
        .launch();
}
