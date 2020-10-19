use std::{env, process, thread};
use std::io::{Write};
use std::net::*;
use std::sync::Arc;

use regex::Regex;
use devtimer::DevTime;
use getopts::Options;
use openssl::ssl::{SslMethod, SslConnector};

fn main() {
    let args: Vec<String> = env::args().collect();
    let (url, count, profile) = parse_args(&args);
    let (url, host, resource, ssl) = delim_url(&url);

    let ip = resolve_host(&url);
    let message = message_constructor(&host, &resource);

    let message = Arc::new(message);
    let ip = Arc::new(ip);
    let host = Arc::new(host);

    let mut children = vec![];

    for _ in 0..count {
        let r_message = message.clone();
        let r_ip = ip.clone();
        let r_host = host.clone();

        children.push(thread::spawn(move || make_request(&r_message, &r_ip, &r_host, ssl, profile)));
    }

    let mut max_size = usize::MIN;
    let mut min_size = usize::MAX;
    let mut times = vec![];
    let mut time_total:u128 = 0;

   	for child in children {
        match child.join() {
            Ok(x) => {

                if x.0 > max_size {
                    max_size = x.0;
                }

                if x.0 < min_size {
                    min_size = x.0;
                }

                times.push(x.1);
                time_total += x.1;
            }
            Err(_) => {}
        };
    }

    if profile {
        times.sort();

        println!("Max size: {} bytes, Min size: {} bytes", max_size, min_size);

        let num_requests = times.len() as u128;
        println!("Max time: {} ms, Min time: {} ms, Mean time: {} ms, Median time: {} ms",
                 times[times.len()-1], times[0], time_total/num_requests, times[times.len()/2]);
    }
}

fn delim_url(url: &str) -> (String, String, String, bool) {

    // Identify protocol
    let regex = Regex::new(r"(?i)(.*)://.*").unwrap();

    let protocol = match regex.captures(&url) {
        Some(c) => c.get(1).map_or("", |m| m.as_str()),
        None => {
            println!("Protocol not defined, assuming HTTP on port 80");
            "http"
        }
    };

    let port: &str;
    let mut ssl = false;

    match protocol {
        "http" => port = "80",
        "https" => {
            port = "443";
            ssl = true;
        }
        _ => {
            println!("Unsupported protocol");
            process::exit(1);
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
    let regex = Regex::new(r"(?i)([a-z0-9._\-]*)(/*[a-z0-9._\-/]*)").unwrap();

    let captures = match regex.captures(&url) {
        Some(c) => c,
        None => {
            println!("Invalid URL");
            process::exit(1);
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

    (url, host.to_string(), resource.to_string(), ssl)

}

fn read_stream<T: std::io::Read>(stream: &mut T, profile: bool) -> usize {

    let mut buffer = [0u8; 1000];
    let mut bytes = 0;

    loop {
        let bytes_read = stream.read(&mut buffer).unwrap();
        if bytes_read > 0 {
            bytes += bytes_read;
            if !profile {
                print!("{}", String::from_utf8_lossy(&buffer));
            }
        } else {
            break;
        }
    }

    if !profile {
        println!("");
    }

    bytes

}

fn make_request(message: &str, ip: &SocketAddr, host: &str, ssl: bool, profile: bool) -> (usize, u128) {
    let message = message.as_bytes();

    let mut timer = DevTime::new_simple();
    timer.start();
    let mut stream = TcpStream::connect(ip).unwrap();
    let bytes;

    if ssl {
        let connector = SslConnector::builder(SslMethod::tls()).unwrap().build();
        let mut ssl_stream = connector.connect(&host, stream).unwrap();
        ssl_stream.write_all(message).unwrap();
        bytes = read_stream(&mut ssl_stream, profile);
    } else {
        stream.write_all(message).unwrap();
        bytes = read_stream(&mut stream, profile);
    }
    timer.stop();
    let time = timer.time_in_millis().unwrap();
    (bytes, time)
}

fn resolve_host(url: &str) -> SocketAddr {

    let resolve = &mut url[..].to_socket_addrs();
    let addrs_iter = match resolve {
        Ok(i) => i,
        Err(_) => {
            println!("Could not resolve host.");
            println!("Please make sure a valid URL is being used.");
            println!("URL must be in format [www.example.com/resource].");
            process::exit(1)
        }
    };

    match addrs_iter.next() {
        Some(i) => i,
        None => {
            println!("Could not resolve host");
            process::exit(1)
        }
    }

}

fn parse_args(args: &[String]) -> (String, i32, bool) {

    let mut opts = Options::new();
    let mut profile = false;

    opts.optopt("u", "url", "Request URL", "URL");
    opts.optopt("p", "profile", "URL to profile", "COUNT");
    opts.optflag("h", "help", "Print help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(_) => {
            println!("Failed to read argument");
            process::exit(1)
        }
    };

    if matches.opt_present("h") {
        println!("{}", opts.usage("Usage"));
        process::exit(1);
    };

    let url = match matches.opt_str("u") {
        Some(u) => u.to_string(),
        None => {
            println!("Error reading URL");
            process::exit(1);
        }
    };

    let count = match matches.opt_str("p") {
        Some(p) => {
            profile = true;
            match p.trim().parse() {
                Ok(x) => x,
                Err(_) => {
                    println!("Failed to argument for profile");
                    process::exit(1)
                }
            }
        },
        None => 1
    };

    (url, count, profile)

}

fn message_constructor(host: &str, resource: &str) -> String {
    let mut message = String::from("GET ");
    message.push_str(&resource);
    message.push_str(" HTTP/1.1\r\n");
    message.push_str("Host: ");
    message.push_str(&host);
    message.push_str("\r\nConnection: close");
    message.push_str("\r\n\r\n");
    message
}
