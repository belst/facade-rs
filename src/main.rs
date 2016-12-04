extern crate dotenv;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate threadpool;

use std::str;
use std::env;
use std::{thread, time};
use std::sync::{Arc, RwLock};
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use dotenv::dotenv;
use regex::bytes::Regex;
use threadpool::ThreadPool;

fn concat_bstring<T: Clone>(strs: &[&[T]]) -> Vec<T> {
    strs.into_iter().flat_map(|str| str.iter().cloned()).collect()
}

#[test]
fn concat_bstring_test() {
    let a = concat_bstring(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);
    assert_eq!(a, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let empty: &[&[u8]] = &[];
    let emptyvec: Vec<u8> = vec![];
    assert_eq!(concat_bstring(empty), emptyvec);
}

fn getstatus<A: ToSocketAddrs>(target: A) -> Result<Vec<u8>, String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => panic!("Could not bind socket: {}", e),
    };
    if let Err(e) = socket.connect(target) {
        return Err(format!("{}", e));
    }
    if let Err(e) = socket.send(b"\xFF\xFF\xFF\xFFgetstatus\n") {
        return Err(format!("{}", e));
    }
    let mut buf = [0; 2048];
    match socket.recv(&mut buf) {
        Ok(amt) => {
            if buf.starts_with(b"\xFF\xFF\xFF\xFFstatusResponse") {
                Ok((&buf[0..amt]).to_owned())
            } else {
                Err("Invalid response".to_string())
            }
        }
        Err(e) => Err(format!("{}", e)),
    }
}

fn getinfo<A: ToSocketAddrs>(target: A) -> Result<Vec<u8>, String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => panic!("Could not bind socket: {}", e),
    };
    if let Err(e) = socket.connect(target) {
        return Err(format!("{}", e));
    }
    if let Err(e) = socket.send(b"\xFF\xFF\xFF\xFFgetinfo\n") {
        return Err(format!("{}", e));
    }
    let mut buf = [0; 2048];
    match socket.recv(&mut buf) {
        Ok(amt) => {
            if buf.starts_with(b"\xFF\xFF\xFF\xFFinfoResponse") {
                Ok((&buf[0..amt]).to_owned())
            } else {
                Err("Invalid response".to_string())
            }
        }
        Err(e) => Err(format!("{}", e)),
    }
}

fn add_challenge(haystack: &[u8], challenge: &str, len: usize) -> Vec<u8> {
    let (first, second) = haystack.split_at(len);
    let mut vec = Vec::new();
    vec.extend(first.iter().cloned());
    vec.extend_from_slice(b"\\challenge\\");
    vec.extend(challenge.as_bytes().iter().cloned());
    vec.extend(second.iter().cloned());
    vec
}

fn replace_hostname(haystack: &[u8], hostname: &str) -> Vec<u8> {
    assert!(!hostname.contains(r"\")); // hostname mustn't contain a \
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\\(?P<hn>(?:sv_)?hostname)\\.+?\\").unwrap();
    }

    RE.replace(haystack,
               concat_bstring(&[br"\$hn\", hostname.as_bytes(), br"\"]).as_slice())
}

#[test]
fn replace_hostname_test() {
    let before = b"\xFF\xFF\xFF\xFFinfoResponse\n\\challenge\\HvpWVoTjnBI\\version\\ET Legacy 2.74a linux-i386 Jan  1 2016\\protocol\\82\\hostname\\^7sKy^2-^7e^2.^7Begin^2ners XPS^7ave\\serverload\\0\\mapname\\baserace_desert\\clients\\18\\humans\\0\\sv_maxclients\\34\\gametype\\6\\pure\\1\\game\\silent\\friendlyFire\\0\\maxlives\\0\\needpass\0\\gamename\\et\\g_antilag\\1\\weaprestrict\\100\\balancedteams\\1";
    let expected = b"\xFF\xFF\xFF\xFFinfoResponse\n\\challenge\\HvpWVoTjnBI\\version\\ET Legacy 2.74a linux-i386 Jan  1 2016\\protocol\\82\\hostname\\abcdef\\serverload\\0\\mapname\\baserace_desert\\clients\\18\\humans\\0\\sv_maxclients\\34\\gametype\\6\\pure\\1\\game\\silent\\friendlyFire\\0\\maxlives\\0\\needpass\0\\gamename\\et\\g_antilag\\1\\weaprestrict\\100\\balancedteams\\1";
    let mut v: Vec<u8> = Vec::new();
    v.extend(expected.iter());
    let after = replace_hostname(before, "abcdef");
    assert_eq!(after, v);
}

fn replace_ver(haystack: &[u8]) -> Vec<u8> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\\protocol\\\d{2}").unwrap();
    }
    RE.replace(haystack, &b"\\protocol\\84"[..])
}

#[test]
fn replace_ver_test() {
    let before = b"\xFF\xFF\xFF\xFFinfoResponse\n\\challenge\\HvpWVoTjnBI\\version\\ET Legacy 2.74a linux-i386 Jan  1 2016\\protocol\\82\\hostname\\^7sKy^2-^7e^2.^7Begin^2ners XPS^7ave\\serverload\\0\\mapname\\baserace_desert\\clients\\18\\humans\\0\\sv_maxclients\\34\\gametype\\6\\pure\\1\\game\\silent\\friendlyFire\\0\\maxlives\\0\\needpass\0\\gamename\\et\\g_antilag\\1\\weaprestrict\\100\\balancedteams\\1";
    let expected = b"\xFF\xFF\xFF\xFFinfoResponse\n\\challenge\\HvpWVoTjnBI\\version\\ET Legacy 2.74a linux-i386 Jan  1 2016\\protocol\\84\\hostname\\^7sKy^2-^7e^2.^7Begin^2ners XPS^7ave\\serverload\\0\\mapname\\baserace_desert\\clients\\18\\humans\\0\\sv_maxclients\\34\\gametype\\6\\pure\\1\\game\\silent\\friendlyFire\\0\\maxlives\\0\\needpass\0\\gamename\\et\\g_antilag\\1\\weaprestrict\\100\\balancedteams\\1";
    let mut v: Vec<u8> = Vec::new();
    let mut v2: Vec<u8> = Vec::new();
    v.extend(before.iter());
    v2.extend(expected.iter());
    let after = replace_ver(&v);
    assert_eq!(after, v2);
}


const HEARTBEAT: &'static [u8] = b"\xFF\xFF\xFF\xFFheartbeat EnemyTerritory-1\n";
const PREFIX: &'static [u8] = b"\xFF\xFF\xFF\xFF";

fn upd_info_and_heartbeat<A: ToSocketAddrs>(socket: UdpSocket,
                                            host: A,
                                            info: Arc<RwLock<Vec<u8>>>,
                                            master_servers: &[A]) {
    println!("Updating Info");
    {
        // subscope so write lock gets freed early
        let tmp = getinfo(host).unwrap();
        let mut info = info.write().unwrap();
        *info = tmp;
    }
    println!("Sending heartbeats");
    for server in master_servers.iter() {
        let _ = socket.send_to(HEARTBEAT, server);
    }
}

fn main() {
    dotenv().ok();

    let listen = env::var("LISTEN")
        .unwrap_or("0.0.0.0:27960".into())
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let host = env::var("SERVER_ADDR")
        .expect("No SERVER_ADDR given!")
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let challengeresponse = concat_bstring(&[b"\xFF\xFF\xFF\xFFprint\nET://",
                                             env::var("SERVER_ADDR").unwrap().as_bytes()]);
    let hostname = env::var("HOSTNAME").unwrap_or("".into());
    let master_servers: Vec<SocketAddr> = env::var("MASTER_SERVERS")
        .unwrap_or("etmaster.idsoftware.com:27950".into())
        .split_whitespace()
        .filter_map(|s| s.to_socket_addrs().unwrap().next())
        .collect();
    let num_threads = env::var("WORKER_THREADS").unwrap_or("10".into()).parse().unwrap();

    let info = Arc::new(RwLock::new(vec![]));


    println!("Spinning up server.");
    let socket = match UdpSocket::bind(listen) {
        Ok(s) => {
            println!("Listening on: {}", s.local_addr().unwrap());
            s
        }
        Err(e) => panic!("Could not bind socket: {}", e),
    };

    let child_info = info.clone();
    let sock = socket.try_clone().unwrap();
    let child_master_servers = master_servers.clone();
    // do it once blocking
    upd_info_and_heartbeat(sock.try_clone().unwrap(),
                           host,
                           child_info.clone(),
                           &child_master_servers);
    thread::spawn(move || {
        loop {
            // update info every 5 minutes
            thread::sleep(time::Duration::from_secs(300));
            upd_info_and_heartbeat(sock.try_clone().unwrap(),
                                   host,
                                   child_info.clone(),
                                   &child_master_servers);
        }
    });

    let pool = ThreadPool::new(num_threads);

    let mut buf = [0; 2048];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                if buf[0..4] != *PREFIX {
                    println!("Invalid Prefix");
                    continue;
                };
                let sock = socket.try_clone().unwrap();
                let challengeresponse = challengeresponse.clone();
                let info = info.clone();
                let master_servers = master_servers.clone();
                let hostname = hostname.clone();
                pool.execute(move || {
                    let s = str::from_utf8(&buf[4..amt]).unwrap_or("Invalid str");
                    println!("{}", s);

                    match s {
                        s if s.starts_with("getinfo") => {
                            let (_, challenge) = s.split_at("getinfo".len());
                            let challenge = challenge.trim();
                            let info = info.read().unwrap();
                            let mut info = replace_ver(info.as_slice());
                            if !hostname.is_empty() {
                                info = replace_hostname(info.as_slice(), &hostname);
                            }
                            if !challenge.is_empty() {
                                sock.send_to(&*add_challenge(&info, challenge, 17), src).unwrap()
                            } else {
                                sock.send_to(&*info, src).unwrap()
                            };
                        }
                        s if s.starts_with("getstatus") => {
                            let (_, challenge) = s.split_at("getstatus".len());
                            let challenge = challenge.trim();
                            let mut status = if !challenge.is_empty() {
                                add_challenge(&getstatus(host).unwrap(), challenge, 19)
                            } else {
                                getstatus(host).unwrap()
                            };
                            if !hostname.is_empty() {
                                status = replace_hostname(status.as_slice(), &hostname);
                            };

                            sock.send_to(&replace_ver(status.as_slice()), src).unwrap();
                        }
                        s if s.starts_with("getchallenge") => {
                            sock.send_to(challengeresponse.as_slice(), src).unwrap();
                            upd_info_and_heartbeat(sock, host, info, &master_servers);
                        }
                        s => panic!("Invalid request type: {}", s),
                    }
                });
            }
            Err(e) => println!("Could not receive a packet: {}", e),
        }
    }
}
