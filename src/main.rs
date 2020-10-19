use std::env;
use std::process;
use std::io::{Read, Write};
use std::net::*;
use devtimer::DevTime;
use getopts::Options;
use regex::Regex;

fn delim_url(url: &str) -> (String, String, String) {

    // Identify protocol
    let regex = Regex::new(r"(?i)(.*)://.*").unwrap();

    let mut protocol = match regex.captures(&url) {
        Some(c) => c.get(1).map_or("", |m| m.as_str()),
        None => ""
    };

    let mut port: &str;

    match protocol {
        "http" => port = "80",
        "https" => port = "443",
        "" => {
            protocol = "http";
            port = "80";
            println!("Protocol not defined, assuming HTTP on port 80");
        },
        _ => {
            println!("Unsupported protocol");
            process::exit(0);
        }
    };

    // Remove scheme
    let regex = Regex::new(r"(?i).*://(.*)").unwrap();

    let url = {
        if regex.is_match(&url) {
            let capture = match regex.captures(&url) {
                Some(c) => c.get(1).map_or("", |m| m.as_str()),
                None => &url
            };
            capture
        } else {
            &url
        }
    };

    // Separate host from resource
    let regex = Regex::new(r"(?i)([a-z0-9._\-]*)/*([a-z0-9._\-/]*)").unwrap();

    let captures = match regex.captures(&url) {
        Some(c) => c,
        None => {
            println!("Invalid URL");
            process::exit(0);
        }
    };

    let host = captures.get(1).map_or("", |m| m.as_str());

    let mut resource = captures.get(2).map_or("", |m| m.as_str());
    if resource == "" {
        resource = "/";
    }

    // Construct URL for TCPStream
    let mut url = String::from("");
    url.push_str(host);
    url.push_str(":");
    url.push_str(port);

    (url, host.to_string(), resource.to_string())

}

fn make_request(message: &[u8], ip: SocketAddr) {

    let mut timer = DevTime::new_simple();
    timer.start();
    let mut stream = TcpStream::connect(ip).unwrap();

    stream.write(message).unwrap();
    let mut buffer = [0; 1000 * 1000];

    stream.read(&mut buffer).unwrap();
    timer.stop();
    println!("{}", String::from_utf8_lossy(&buffer));
    println!("The time taken for the operation was: {} millis", timer.time_in_millis().unwrap());
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

fn profile(count: i32) {
    println!("{}", count);
}

fn message_constructor(host: &str, resource: &str) -> String {
    let mut message = String::from("GET /");
    message.push_str(&resource);
    message.push_str(" HTTP/1.1\r\n");
    message.push_str("Host: ");
    message.push_str(&host);
    message.push_str("\r\n\r\n");
    message
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (url, count) = parse_args(&args);
    let (url, host, resource) = delim_url(&url[..]);
    println!("{} {} {}", url, host, resource);
    /*
    let (host, url, resource) = delim_url(&url[..]);
    let ip = resolve_host(url);

    if count > 0 {
        profile(count);
    } else {
        let message = message_constructor(&host, &resource);
        make_request(&message.as_bytes(), ip);
    }
    */
}
