use std::env;
use std::process;
use std::net::*;
use getopts::Options;
use regex::Regex;
use std::io::{Read, Write};

fn request_constructor(url: &str) -> (String, String, String) {

    let regex = Regex::new(r"(?i)([a-z0-9._\-]*)/*([a-z0-9._\-/]*)").unwrap();

    let captures = match regex.captures(&url) {
        Some(c) => c,
        None => {
            println!("Invalid URL");
            process::exit(0);
        }
    };

    let host = captures.get(1).map_or("", |m| m.as_str());
    let resource = captures.get(2).map_or("", |m| m.as_str());
    let port = "80";

    let mut url = String::from("");
    url.push_str(host);
    url.push_str(":");
    url.push_str(port);

    (host.to_string(), url, resource.to_string())
}

fn make_request(host: &str, resource: &str, ip: SocketAddr) {

    let mut stream = TcpStream::connect(ip).unwrap();

    let mut message = String::from("GET /");
    message.push_str(&resource);
    message.push_str(" HTTP/1.1\r\n");
    message.push_str("Host: ");
    message.push_str(&host);
    message.push_str("\r\n\r\n");

    stream.write(message.as_bytes()).unwrap();
    let mut buffer = [0; 1000 * 1000];

    stream.read(&mut buffer).unwrap();
    println!("{}", String::from_utf8_lossy(&buffer[..]));

}

fn resolve_host(url: String) -> SocketAddr {

    let resolve = &mut url[..].to_socket_addrs();
    let addrs_iter = match resolve {
        Ok(i) => i,
        Err(_) => {
            println!("Could not resolve host.");
            println!("Please make sure a valid URL is being used.");
            println!("URL must be in format [www.example.com/resource].");
            process::exit(0)
        }
    };

    match addrs_iter.next() {
        Some(i) => i,
        None => {
            println!("Could not resolve host");
            process::exit(0)
        }
    }

}

fn parse_args(args: &[String]) -> (String, i32) {

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
    let (host, url, resource) = request_constructor(&url[..]);
    let ip = resolve_host(url);

    make_request(&host, &resource, ip);
}
