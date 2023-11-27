# mping-rs
[![License](https://img.shields.io/:license-apache%202-blue.svg)](https://opensource.org/licenses/Apache-2.0) ![GitHub Action](https://github.com/smallnest/mping-rs/actions/workflows/quickstart.yml/badge.svg) [![Crate](https://img.shields.io/crates/v/mping.svg)](https://crates.io/crates/mping)
[![API](https://docs.rs/mping/badge.svg)](https://docs.rs/mping)

a multi-targets ping tool, which supports 10,000 packets/second, accurate latency.

> 一个高频ping工具，支持多个目标。
> 正常的ping一般用来做探测工具，mping还可以用来做压测工具。
> Go版本: [smallnest/mping](https://github.com/smallnest/mping)


And you can use it as a lib to implement your multi-targets and handle the ping results. See the `ping.rs` example in the examples folder.

## Usage - as a tool

compile

```sh
cargo build --release
```

options usage.

```sh
> $$ mping  -h
mping 0.4.2
A multi-targets ping tool, which supports 10,000 packets/second.

USAGE:
    mping [OPTIONS] <ip address>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --count <count>        max packet count
    -d, --delay <delay>        delay in seconds [default: 3]
    -r, --rate <rate>          rate in packets/second [default: 100]
    -s, --size <size>          payload size [default: 64]
    -w, --timeout <timeout>    timeout in seconds [default: 1]
    -z, --tos <tos>            type of service
    -t, --ttl <ttl>            time to live [default: 64]

ARGS:
    <ip address>...    one ip address or more, e.g. 127.0.0.1,8.8.8.8/24,bing.com
```

example

```sh
sudo ./mping -r 5 8.8.8.8
sudo ./mping -r 100 8.8.8.8/30,8.8.4.4,github.com
sudo ./mping -r 100 github.com,bing.com
```

docker:
```
sudo docker run --rm -it smallnest/mping-rs:latest 8.8.8.8
```


## Usage - as a library

### ping a target once

```rust
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
    let addr = args[1].clone();

    match ping_once(addr, Some(Duration::from_secs(2)), Some(1234), Some(64), None, Some(64)) {
        Ok((bitflip,latency)) => {
            println!("bitflip: {}, latency: {:.2}ms", bitflip, latency.as_secs_f64()*1000.0);
        }
        Err(e) => {
            println!("error: {:?}", e);
        }
    }

}
```

### ping many targets forever

```rust
use std::env;
use std::process;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::net::{IpAddr, ToSocketAddrs};

use ::mping::{ping, PingOption};

/// A multi-targets ping example, which use mping crate.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <ip-address>", args[0]);
        process::exit(1);
    }

    let pid = process::id() as u16;
    let target = args[1].clone();
    let addr = parse_ip(&target);

    let addrs = vec![addr.parse().unwrap()];
    let popt = PingOption {
        timeout: Duration::from_secs(1),
        ttl: 64,
        tos: None,
        ident: pid,
        len: 56,
        rate: 100,
        rate_for_all: false,
        delay: 3,
        count: None,
    };
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || match ping(addrs, popt, false, Some(tx)) {
        Ok(_) => {}
        Err(e) => {
            println!("error: {:?}", e);
        }
    });

    for tr in rx {
        let total = tr.received + tr.loss;
        let loss_rate = if total == 0 {
            0.0
        } else {
            (tr.loss as f64) / (total as f64)
        };

        if tr.received == 0 {
            println!(
                "{}: sent:{}, recv:{}, loss rate: {:.2}%, latency: {}ms",
                addr,
                total,
                tr.received,
                loss_rate * 100.0,
                0
            )
        } else {
            println!(
                "{}: sent:{}, recv:{},  loss rate: {:.2}%, latency: {:.2}ms",
                addr,
                total,
                tr.received,
                loss_rate * 100.0,
                Duration::from_nanos(tr.latency as u64 / (tr.received as u64)).as_secs_f64()
                    * 1000.0
            )
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
```
