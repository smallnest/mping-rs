use std::env;
use std::process;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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

    let addrs = vec![target.parse().unwrap()];
    let popt = PingOption {
        timeout: Duration::from_secs(1),
        ttl: 64,
        tos: None,
        ident: pid,
        len: 56,
        rate: 100,
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
                target,
                total,
                tr.received,
                loss_rate * 100.0,
                0
            )
        } else {
            println!(
                "{}: sent:{}, recv:{},  loss rate: {:.2}%, latency: {:.2}ms",
                target,
                total,
                tr.received,
                loss_rate * 100.0,
                Duration::from_nanos(tr.latency as u64 / (tr.received as u64)).as_secs_f64()
                    * 1000.0
            )
        }
    }
}
