use std::env;
use std::process;
use std::time::Duration;
use std::net::{IpAddr, ToSocketAddrs};

use mping::ping_once;

/// ping a target as an example of ping_once.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <ip-address>", args[0]);
        process::exit(1);
    }
    let target = args[1].clone();
    let addr = parse_ip(&target);

    match ping_once(addr, Some(Duration::from_secs(2)), Some(1234), Some(64), None, Some(64)) {
        Ok((bitflip,latency)) => {
            println!("bitflip: {}, latency: {:.2}ms", bitflip, latency.as_secs_f64()*1000.0);
        }
        Err(e) => {
            println!("error: {:?}", e);
        }
    }

}

fn parse_ip(s: &str) -> String {
    if let Ok(_) = s.parse::<IpAddr>() {
        return s.to_string();
    } else if let Ok(addrs) = (s, 0).to_socket_addrs() {
        for addr in addrs {
            if let IpAddr::V4(ipv4) = addr.ip() {
                return IpAddr::V4(ipv4).to_string();
            }
        }
    }
    
    s.to_string()
}