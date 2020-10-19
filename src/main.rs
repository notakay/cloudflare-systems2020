use std::{env, process, thread};
use std::io::{Write};
use std::net::*;
use std::sync::Arc;
use std::collections::HashSet;

use regex::Regex;
use devtimer::DevTime;
use getopts::Options;
use openssl::ssl::{SslMethod, SslConnector};

fn main() {

    // Get arguments using get opts. Parse URL using regexp
    // Resolve DNS using networking primitives from std::net
    // Construct a GET request to the resource in URL

    let args: Vec<String> = env::args().collect();
    let (url, count, profile) = parse_args(&args);
    let (url, host, resource, ssl) = delim_url(&url);

    let ip = resolve_host(&url);
    let message = message_constructor(&host, &resource);

    // Perform a GET requests on separate threads
    // Read/Write over sockets using TcpStream (also part of std:net)
    // Used openssl to set up TLS for websites using TLS/SSL

    let message = Arc::new(message);
    let ip = Arc::new(ip);
    let host = Arc::new(host);

    let mut children = vec![];
    let mut err_codes = HashSet::new();

    let mut max_size = usize::MIN;
    let mut min_size = usize::MAX;
    let mut times = vec![];
    let mut time_total:u128 = 0;
    let mut success = 0;
    let mut fail = 0;

    for _ in 0..count {
        let r_message = message.clone();
        let r_ip = ip.clone();
        let r_host = host.clone();

        children.push(thread::spawn(move || make_request(&r_message, &r_ip, &r_host, ssl, profile)));
    }

   	for child in children {
        match child.join() {
            Ok(x) => {
                if x.0 > max_size { max_size = x.0; }
                if x.0 < min_size { min_size = x.0; }

                if x.2 == "200" {
                    success += 1;
                } else {
                    err_codes.insert(x.2);
                    fail += 1;
                }

                times.push(x.1);
                time_total += x.1;
            },
            Err(_) => {}
        };
    }

    // Output for profiling

    if profile {
        times.sort();
        let rate = (success as f32 / count as f32) * 100.0;
        println!("Made {} GET requests with {}% success rate ({} requests failed)", count, rate, fail);
        println!("The maximum response size was {} bytes. The minimum response size was {} bytes", max_size, min_size);
        println!("The maximum time taken to receive the response was {} ms. The minimum time taken to receive the response was {} ms.", times[times.len()-1], times[0]);
        println!("The average time to download the response was {} ms, whereas the median was {} ms.", time_total/count as u128, times[times.len()/2]);
        println!("Error Codes: {:?}", err_codes);
    }
}

// Breaks down a URL into protocol, host and resource
// Protocol has to be http or https
// If a protocol is not defined assume it's http and prot 80
//
// Takes in a raw URL, example "https://www.example.com/res"
// Returns host:port, host, resource, sslEnabled
//  (example: "www.example.com:443", "www.example.com", "/res", true)
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

// Extract HTTP code from response header
// Returns status code
//  (example: "HTTP/1.1 200 ok" returns "200")
fn extract_http_code(buffer: &str) -> String {
    let regex = Regex::new(r"(?i)HTTP/.* (\d{3}) .*").unwrap();
    let captures = match regex.captures(&buffer) {
        Some(c) => c,
        None => {
            println!("Invalid HTTP Response");
            process::exit(1);
        }
    };
    captures.get(1).map_or("", |m| m.as_str()).to_string()
}

// Generic function for reading from socket stream for both TcpStream and SSL connection
// If profiling is enabled, function will print content to stdout
// Returns size of resource and the status code
fn read_stream<T: std::io::Read>(stream: &mut T, profile: bool) -> (usize, String) {
    let mut buffer = [0u8; 1000];
    let mut bytes = 0;
    let mut get_status = false;
    let mut status_code = String::from("");

    // while input is coming from stream, place data in buffer
    // extract HTTP code for the first iteration
    loop {
        let bytes_read = stream.read(&mut buffer).unwrap();
        if bytes_read > 0 {
            bytes += bytes_read;
            if !profile {
                print!("{}", String::from_utf8_lossy(&buffer));
            }
            if !get_status {
                status_code = String::from_utf8_lossy(&buffer[0..15]).to_string();
                status_code = extract_http_code(&status_code);
                get_status = true;
            }
            // Set buffer to a new chunk of memory
            buffer = [0u8; 1000];
        } else {
            break;
        }
    }

    if !profile {
        println!("");
    }

    (bytes, status_code)
}

// Create a connection to the destination ip and port
// If connection to a https website, set up a ssl connector using openssl
// Write GET request to socket and read/process response
// Start the timer when first connecting to destination address
// Stop timer after reading all the data
// Returns size of content, time taken and status code
fn make_request(message: &str, ip: &SocketAddr, host: &str, ssl: bool, profile: bool) -> (usize, u128, String) {
    let message = message.as_bytes();

    let mut timer = DevTime::new_simple();
    timer.start();
    let mut stream = TcpStream::connect(ip).unwrap();
    let r_tuple;

    if ssl {
        let connector = SslConnector::builder(SslMethod::tls()).unwrap().build();
        let mut ssl_stream = connector.connect(&host, stream).unwrap();
        ssl_stream.write_all(message).unwrap();
        r_tuple = read_stream(&mut ssl_stream, profile);
    } else {
        stream.write_all(message).unwrap();
        r_tuple = read_stream(&mut stream, profile);
    }
    timer.stop();
    let time = timer.time_in_millis().unwrap();
    (r_tuple.0, time, r_tuple.1)
}

// Resolve DNS
// Returns SocketAddr enum after resolving DNS name
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

// Parses arguments using the getopt
// Returns the url, number of request to make
// and a boolean to check if profiling is enabled
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
                Ok(x) => {
                    if x <= 0{
                        println!("Argument for profile must be greater than 0");
                        process::exit(1);
                    }
                    x
                },
                Err(_) => {
                    println!("Failed to read argument for profile");
                    process::exit(1)
                }
            }
        },
        None => 1
    };

    (url, count, profile)

}

// Constructs a GET request
// Returns a GET request
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
