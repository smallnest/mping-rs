use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use std::os::unix::io::AsRawFd;
use libc::{c_void, c_int, setsockopt, recvmsg, iovec, msghdr, cmsghdr, SOL_SOCKET, SO_TIMESTAMP, timeval};


use log::{error, info, warn};
use rand::Rng;
use rate_limit::SyncLimiter;
use ticker::Ticker;

use socket2::{Domain, Protocol, Socket, Type};
use pnet_packet::icmp::{self, echo_reply, echo_request, IcmpTypes};
use pnet_packet::ipv4::Ipv4Packet;
use pnet_packet::Packet;

use crate::stat::{Buckets, Result, TargetResult};

pub struct  PingOption {
    pub timeout: Duration,
    pub ttl: u32,
    pub tos: Option<u32>,
    pub ident: u16,
    pub len: usize,
    pub rate: u64,
    pub delay: u64,
    pub count: Option<i64>,
}

pub fn ping(
    addrs: Vec<IpAddr>,
    popt: PingOption,
) -> anyhow::Result<()> {
    let pid = popt.ident;

    let rand_payload = random_bytes(popt.len);
    let read_rand_payload = rand_payload.clone();

    let buckets = Arc::new(Mutex::new(Buckets::new()));
    let send_buckets = buckets.clone();
    let read_buckets = buckets.clone();
    let stat_buckets = buckets.clone();

    // send
    thread::spawn(move || {
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).unwrap();
        socket.set_ttl(popt.ttl).unwrap();
        socket.set_write_timeout(Some(popt.timeout)).unwrap();
        if let Some(tos_value) = popt.tos  {
            socket.set_tos(tos_value).unwrap();
        }

        let zero_payload = vec![0; popt.len];
        let one_payload = vec![1; popt.len];
        let fivea_payload = vec![0x5A; popt.len];

        let payloads: [&[u8]; 4] = [&rand_payload, &zero_payload, &one_payload, &fivea_payload];

        let limiter = SyncLimiter::full(popt.rate, Duration::from_millis(1000));
        let mut seq = 1u16;
        let mut sent_count = 0;

        loop {
            limiter.take();

            let payload = payloads[seq as usize % payloads.len()];

            let mut buf = vec![0; 8 + payload.len()]; // 8 bytes of header, then payload
            let mut packet = echo_request::MutableEchoRequestPacket::new(&mut buf[..]).unwrap();
            packet.set_icmp_type(icmp::IcmpTypes::EchoRequest);
            packet.set_identifier(pid);
            packet.set_sequence_number(seq);

            let now = SystemTime::now();
            let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
            let timestamp = since_the_epoch.as_nanos();

            let ts_bytes = timestamp.to_be_bytes();
            let mut send_payload = vec![0; payload.len()];
            send_payload[..16].copy_from_slice(&ts_bytes[..16]);
            send_payload[16..].copy_from_slice(&payload[16..]);

            packet.set_payload(&send_payload);

            let icmp_packet = icmp::IcmpPacket::new(packet.packet()).unwrap();
            let checksum = icmp::checksum(&icmp_packet);
            packet.set_checksum(checksum);

            for ip in &addrs {
                let dest = SocketAddr::new(*ip, 0);
                let data = send_buckets.lock().unwrap();
                data.add(
                    timestamp / 1_000_000_000,
                    Result {
                        txts: timestamp,
                        target: dest.ip().to_string(),
                        seq,
                        latency: 0,
                        received: false,
                        bitflip: false,
                        ..Default::default()
                    },
                );
                drop(data);

                match socket.send_to(&buf, &dest.into()) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error in send: {:?}", e);
                        return;
                    }
                }
            }

            seq += 1;
            sent_count += 1;

            if popt.count.is_some() && sent_count >= popt.count.unwrap() {
                thread::sleep(Duration::from_secs(popt.delay));
                info!("reached {} and exit", sent_count);
                std::process::exit(0);
            }
        }
    });

    thread::spawn(move || print_stat(stat_buckets, popt.delay));

    // read
    let zero_payload = vec![0; popt.len];
    let one_payload = vec![1; popt.len];
    let fivea_payload = vec![0x5A; popt.len];

    let payloads: [&[u8]; 4] = [
        &read_rand_payload,
        &zero_payload,
        &one_payload,
        &fivea_payload,
    ];

    let socket2 = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?;
    socket2.set_read_timeout(Some(popt.timeout))?;
    let enable: c_int = 1;
    let ret = unsafe { setsockopt(socket2.as_raw_fd(), SOL_SOCKET, SO_TIMESTAMP, &enable as *const _ as *const c_void, std::mem::size_of_val(&enable) as u32) };
    if ret == -1 {
        warn!("Failed to set SO_TIMESTAMP");
    }

    let mut buffer: [u8; 2048] = [0; 2048];
    let mut control_buf = [0; 1024];

    let mut iovec = iovec {
        iov_base: buffer.as_mut_ptr() as *mut c_void,
        iov_len: buffer.len(),
    };

    let mut msghdr = msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_iov: &mut iovec,
        msg_iovlen: 1,
        msg_control: control_buf.as_mut_ptr() as *mut c_void,
        msg_controllen: control_buf.len(),
        msg_flags: 0,
    };


    loop {
        // let size = match socket2.read(&mut buffer) {
        //     Ok(n) => n,
        //     Err(e) => {
        //         if e.kind() == ErrorKind::WouldBlock {
        //             continue;
        //         }
        //         error!("Error in read: {:?}", &e);

        //         break;
        //     }
        // };
        // let buf = &buffer[..size];


        let nbytes = unsafe { recvmsg(socket2.as_raw_fd(), &mut msghdr, 0) };
        if nbytes == -1 {
            let err = Error::last_os_error();
            if err.kind() == ErrorKind::WouldBlock {
                continue;
            }

            error!("Failed torr receive message");
            return Err(Error::new(ErrorKind::Other, "Failed to receive message").into());
        }

        let buf = &buffer[..nbytes as usize];

        let ipv4_packet = Ipv4Packet::new(buf).unwrap();
        let icmp_packet = pnet_packet::icmp::IcmpPacket::new(ipv4_packet.payload()).unwrap();

        if icmp_packet.get_icmp_type() != IcmpTypes::EchoReply
            || icmp_packet.get_icmp_code() != echo_reply::IcmpCodes::NoCode
        {
            continue;
        }

        let echo_reply = match icmp::echo_reply::EchoReplyPacket::new(icmp_packet.packet()) {
            Some(echo_reply) => echo_reply,
            None => {
                continue;
            }
        };

        if echo_reply.get_identifier() != pid {
            continue;
        }

        if payloads[echo_reply.get_sequence_number() as usize % payloads.len()][16..]
            != echo_reply.payload()[16..]
        {
            warn!(
                "bitflip detected! seq={:?},",
                echo_reply.get_sequence_number()
            );
        }
        
        let payload = echo_reply.payload();
        let ts_bytes = &payload[..16];
        let txts = u128::from_be_bytes(ts_bytes.try_into().unwrap());
        let dest_ip = ipv4_packet.get_source();

        let now = SystemTime::now();
        let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
        let mut timestamp = since_the_epoch.as_nanos();
        if let Some(rxts) = get_timestamp(&mut msghdr) {
            timestamp = rxts.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        }

        let buckets = read_buckets.lock().unwrap();
        buckets.add_reply(
            txts / 1_000_000_000,
            Result {
                txts,
                rxts: timestamp,
                target: dest_ip.to_string(),
                seq: echo_reply.get_sequence_number(),
                latency: 0,
                received: true,
                bitflip: false
            },
        );
    }

    
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut vec = vec![0u8; len];
    rng.fill(&mut vec[..]);

    vec
}

fn print_stat(buckets: Arc<Mutex<Buckets>>, delay: u64) -> anyhow::Result<()> {
    let delay = Duration::from_secs(delay).as_nanos(); // 5s
    let mut last_key = 0;

    let ticker = Ticker::new(0.., Duration::from_secs(1));
    for _ in ticker {
        let buckets = buckets.lock().unwrap();
        let bucket = buckets.last();
        if bucket.is_none() {
            continue;
        }

        let bucket = bucket.unwrap();
        if bucket.key <= last_key {
            buckets.pop();
            continue;
        }

        if bucket.key
            <= SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                - delay
        {
            if let Some(pop) = buckets.pop() {
                if pop.key < bucket.key {
                    continue;
                }

                last_key = pop.key;

                // 统计和打印逻辑
                // 统计
                let mut target_results = BTreeMap::new();

                for r in pop.values() {
                    let target_result = target_results
                        .entry(r.target.clone())
                        .or_insert_with(TargetResult::default);

                    target_result.latency += r.latency;

                    if r.received {
                        target_result.received += 1;
                    } else {
                        target_result.loss += 1;
                    }
                }

                // 打印
                for (target, tr) in &target_results {
                    let total = tr.received + tr.loss;
                    let loss_rate = if total == 0 {
                        0.0
                    } else {
                        (tr.loss as f64) / (total as f64)
                    };

                    if tr.received == 0 {
                        info!(
                            "{}: sent:{}, recv:{}, loss rate: {:.2}%, latency: {}ms",
                            target,
                            total,
                            tr.received,
                            loss_rate * 100.0,
                            0
                        )
                    } else {
                        info!(
                            "{}: sent:{}, recv:{},  loss rate: {:.2}%, latency: {:.2}ms",
                            target,
                            total,
                            tr.received,
                            loss_rate * 100.0,
                            Duration::from_nanos(tr.latency as u64 / (tr.received as u64)).as_secs_f64()*1000.0
                        )
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_timestamp(msghdr: &mut msghdr) -> Option<SystemTime> {
    let mut cmsg: *mut cmsghdr = unsafe { libc::CMSG_FIRSTHDR(msghdr) };

    while !cmsg.is_null() {
        if unsafe { (*cmsg).cmsg_level == SOL_SOCKET && (*cmsg).cmsg_type == SO_TIMESTAMP } {
            let tv: *mut timeval = unsafe { libc::CMSG_DATA(cmsg) } as *mut timeval;
            let timestamp = unsafe { *tv };
            return Some(SystemTime::UNIX_EPOCH + Duration::new(timestamp.tv_sec as u64, timestamp.tv_usec as u32 * 1000));
        }

        cmsg = unsafe { libc::CMSG_NXTHDR(msghdr, cmsg) };
    }

    None
}
