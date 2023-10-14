# mping-rs
[![License](https://img.shields.io/:license-apache%202-blue.svg)](https://opensource.org/licenses/Apache-2.0) ![GitHub Action](https://github.com/smallnest/mping-rs/actions/workflows/quickstart.yml/badge.svg) [![Crate](https://img.shields.io/crates/v/mping.svg)](https://crates.io/crates/mping)
[![API](https://docs.rs/mping/badge.svg)](https://docs.rs/mping)

a multi-targets ping tool, which supports 10,000 packets/second, accurate latency.

> 一个高频ping工具，支持多个目标。
> 正常的ping一般用来做探测工具，mping还可以用来做压测工具。
> Go版本: [smallnest/mping](https://github.com/smallnest/mping)


And you can use it as a lib to implement your multi-targets and handle the ping results. See the `ping.rs` example in the examples folder.

## Usage

compile

```sh
cargo build --release
```

options usage.

```sh
> $$ mping  -h
mping 0.3.0
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