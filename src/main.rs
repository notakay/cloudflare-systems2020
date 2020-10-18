use std::env;
use std::net::*;
use getopts::Options;
use regex::Regex;

fn request_constructor(url: &str) -> (String, String) {

    let regex = Regex::new(r"(?i)^(https?)://([^\s$.?#/][a-zA-Z0-9_.]*)/*([^\s$?#]*)").unwrap();

    if !regex.is_match(&url) {
        panic!{"Invalid URL. URL must be in format http(s)://example.com/(resource)"};
    }

    let captures = regex.captures(&url).unwrap(); // Check if this fails??

    let protocol = captures.get(1).map_or("", |m| m.as_str());
    let host = captures.get(2).map_or("", |m| m.as_str());
    let resource = captures.get(3).map_or("", |m| m.as_str());

    let port = match &protocol.to_lowercase()[..] {
        "http" => "80",
        "https" => "443",
        _ => {
            panic!{"Invalid Protocol. Use HTTP or HTTPS"};
        }
    };

    let mut url = String::from("");
    url.push_str(host);
    url.push_str(":");
    url.push_str(port);

    (url, resource.to_string())
}

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

    let (url, resource) = request_constructor(&url[..]);

    println!("{} {}", url, resource);

    let addrs_iter = &mut url.to_socket_addrs().unwrap();
    println!("{:?}", addrs_iter.next());

}
