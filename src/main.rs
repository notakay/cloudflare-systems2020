use std::env;
use std::net::*;
use getopts::Options;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();

    opts.optopt("u", "url", "Request URL", "URL");
    opts.optopt("p", "profile", "URL to profile", "COUNT");
    opts.optflag("h", "help", "Print help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        println!("{}", opts.usage("Usage"));
        return;
    }

    let url = match matches.opt_str("u") {
        Some(u) => u,
        None => return
    };

    // check for http or https

    //let mut addr_iter = "general-engineering-assignment.akay.workers.dev:443".to_socket_addrs().unwrap();
    //println!("{:?}", addr_iter.next());
}
