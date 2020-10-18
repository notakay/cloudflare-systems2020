use std::env;
use std::process;
use std::net::*;
use getopts::Options;
use regex::Regex;

fn request_constructor(url: &str) -> (String, String) {

    let regex = Regex::new(r"(?i)^(https?)://([^\s$.?#/][a-zA-Z0-9_.]*)/*([^\s$?#]*)").unwrap();

    let captures = match regex.captures(&url) {
        Some(c) => c,
        None => {
            println!("Invalid URL");
            process::exit(0);
        }
    };

    let protocol = captures.get(1).map_or("", |m| m.as_str());
    let host = captures.get(2).map_or("", |m| m.as_str());
    let resource = captures.get(3).map_or("", |m| m.as_str());

    let port = match &protocol.to_lowercase()[..] {
        "http" => "80",
        "https" => "443",
        _ => {
            println!("Invalid URL");
            process::exit(0);
        }
    };

    let mut url = String::from("");
    url.push_str(host);
    url.push_str(":");
    url.push_str(port);

    (url, resource.to_string())
}

fn parse_args(args: &[String]) -> (String, i32){

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
        process::exit(0);
    };

    let url = match matches.opt_str("u") {
        Some(u) => u.to_string(),
        None => {
            println!("Error reading URL");
            process::exit(0);
        }
    };

    let profile = match matches.opt_str("p") {
        Some(p) => p.trim().parse().expect("Expected a number for --profile"),
        None => 0
    };

    (url, profile)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let (url, profile) = parse_args(&args);
    println!("{} {}", url, profile);

    let (url, resource) = request_constructor(&url[..]);
    println!("{} {}", url, resource);

    let resolve = &mut url[..].to_socket_addrs();
    let addrs_iter = match resolve {
        Ok(i) => i,
        Err(_) => {
            println!("Could not resolve host");
            process::exit(0)
        }
    };

    let ip = match addrs_iter.next() {
        Some(i) => i,
        None => {
            println!("Could not resolve host");
            process::exit(0)
        }
    };

    let stream = TcpStream::connect(ip);

    println!("{:?}", stream);

}
