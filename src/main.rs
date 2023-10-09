use std::io::Write;
use std::process;
use std::time::Duration;

use anyhow::Result;
use chrono::Local;
use structopt::StructOpt;

use ipnetwork::IpNetwork;
use std::net::{IpAddr, ToSocketAddrs};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mping",
    version = "0.1.5",
    about = "A multi-targets ping tool, which supports 10,000 packets/second."
)]
struct Opt {
    #[structopt(
        short = "w",
        long = "timeout",
        default_value = "1",
        help = "timeout in seconds"
    )]
    timeout: u64,

    #[structopt(short = "t", long = "ttl", default_value = "64", help = "time to live")]
    ttl: u32,

    #[structopt(short = "z", long = "tos", help = "type of service")]
    tos: Option<u32>,

    #[structopt(
        short = "s",
        long = "size",
        default_value = "64",
        help = "payload size"
    )]
    size: usize,

    #[structopt(
        short = "r",
        long = "rate",
        default_value = "100",
        help = "rate in packets/second"
    )]
    rate: u64,

    #[structopt(
        short = "d",
        long = "delay",
        default_value = "3",
        help = "delay in seconds"
    )]
    delay: u64,

    #[structopt(short = "c", long = "count", help = "max packet count")]
    count: Option<i64>,

    #[structopt(
        parse(from_os_str),
        required = true,
        name = "ip address",
        help = "one ip address or more, e.g. 127.0.0.1,8.8.8.8/24,bing.com"
    )]
    free: Vec<std::path::PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}",
                Local::now().format("%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    let opt = Opt::from_args();

    if opt.free.is_empty() {
        println!("Please input ip address");
        return Ok(());
    }

    let _ = opt.count;

    let addrs = opt.free.last().unwrap().to_string_lossy();
    let ip_addrs = parse_ips(&addrs);

    let timeout = Duration::from_secs(opt.timeout);
    let pid = process::id() as u16;

    let popt = mping::ping::PingOption {
        timeout,
        ttl: opt.ttl,
        tos: opt.tos,
        ident: pid,
        len: opt.size,
        rate: opt.rate,
        delay: opt.delay,
        count: opt.count,
    };
    mping::ping::ping(ip_addrs, popt)?;

    Ok(())
}

fn parse_ips(input: &str) -> Vec<IpAddr> {
    let mut ips = Vec::new();

    for s in input.split(',') {
        match s.parse::<IpNetwork>() {
            Ok(network) => {
                for ip in network.iter() {
                    ips.push(ip);
                }
            }
            Err(_) => {
                if let Ok(ip) = s.parse::<IpAddr>() {
                    ips.push(ip);
                } else if let Ok(addrs) = (s, 0).to_socket_addrs() {
                    for addr in addrs {
                        if let IpAddr::V4(ipv4) = addr.ip() {
                            ips.push(IpAddr::V4(ipv4));
                            break;
                        }
                    }
                }
            }
        }
    }

    ips
}
