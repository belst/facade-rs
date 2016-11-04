extern crate dotenv;

use std::str;
use std::env;
use std::fmt::Write;
use std::{thread, time};
use std::sync::{Arc, RwLock};
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use dotenv::dotenv;

fn to_hex_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &byte in bytes {
        write!(&mut s, "{:X} ", byte).unwrap();
    }
    s
}

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

fn add_challenge(haystack: &Vec<u8>, challenge: &str, len: usize) -> Vec<u8> {
    let (first, second) = haystack.split_at(len);
    let mut vec = Vec::new();
    vec.extend(first.iter().cloned());
    vec.extend(b"\\challenge\\");
    vec.extend(challenge.as_bytes().iter().cloned());
    vec.extend(second.iter().cloned());
    vec
}


fn main() {
    dotenv().ok();

    const PREFIX: &'static [u8; 4] = b"\xFF\xFF\xFF\xFF";
    let LISTEN =
        env::var("LISTEN").unwrap_or("0.0.0.0:27960".to_string()).parse::<SocketAddr>().unwrap();
    let HOST = env::var("HOST").expect("No HOST given!").parse::<SocketAddr>().unwrap();
    let CHALLENGERESPONSE = concat_bstring(&[b"\xFF\xFF\xFF\xFFprint\nET://",
                                             env::var("HOST").unwrap().as_bytes()]);

    let info = Arc::new(RwLock::new(getinfo(&HOST).unwrap()));

    let child_info = info.clone();
    thread::spawn(move || {
        loop {
            // update info every 5 minutes
            thread::sleep(time::Duration::from_secs(300));
            let mut info = child_info.write().unwrap();
            println!("Updating info");
            *info = getinfo(HOST).unwrap();
        }
    });

    println!("Spinning up server.");
    let socket = match UdpSocket::bind(LISTEN) {
        Ok(s) => {
            println!("Listening on: {}", s.local_addr().unwrap());
            s
        }
        Err(e) => panic!("Could not bind socket: {}", e),
    };

    let mut buf = [0; 2048];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                if buf[0..4] != *PREFIX {
                    println!("Invalid Prefix");
                    continue;
                };
                let sock = socket.try_clone().unwrap();
                let CHALLENGERESPONSE = CHALLENGERESPONSE.clone();
                let info = info.clone();
                thread::spawn(move || {
                    println!("new thread Spawned.");
                    // println!("amt: {}", amt);
                    // println!("src: {}", src);
                    // println!("{}", to_hex_string(&buf));

                    let s = str::from_utf8(&buf[4..amt]).unwrap_or("Invalid str");
                    println!("{}", s);

                    match s {
                        s if s.starts_with("getinfo") => {
                            let (_, challenge) = s.split_at("getinfo".len());
                            let challenge = challenge.trim();
                            if challenge.len() != 0 {
                                sock.send_to(&*add_challenge(&*info.read().unwrap(), challenge, 17),
                                             src)
                            } else {
                                sock.send_to(&*info.read().unwrap(), src)
                            }
                        }
                        s if s.starts_with("getstatus") => {
                            let (_, challenge) = s.split_at("getstatus".len());
                            let challenge = challenge.trim();
                            let status = if challenge.len() != 0 {
                                add_challenge(&getstatus(HOST).unwrap(), challenge, 19)
                            } else {
                                getstatus(HOST).unwrap()
                            };
                            sock.send_to(&status, src)
                        }
                        s if s.starts_with("getchallenge") => {
                            sock.send_to(&CHALLENGERESPONSE[..], src)
                        }
                        _ => panic!("Invalid request type"),
                    }
                });
            }
            Err(e) => println!("Could not receive a packet: {}", e),
        }
    }
}
