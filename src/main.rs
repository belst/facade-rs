extern crate dotenv;

use std::str;
use std::env;
use std::fmt::Write;
use std::thread;
use std::sync::Arc;
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use dotenv::dotenv;

fn to_hex_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &byte in bytes {
        write!(&mut s, "{:X} ", byte).unwrap();
    }
    s
}

fn concat_bstring(str1: &[u8], str2: &[u8]) -> Vec<u8> {
    str1.iter().cloned().chain(str2.iter().cloned()).collect()
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

fn main() {
    dotenv().ok();

    const PREFIX: &'static [u8; 4] = b"\xFF\xFF\xFF\xFF";
    let LISTEN =
        env::var("LISTEN").unwrap_or("0.0.0.0:27960".to_string()).parse::<SocketAddr>().unwrap();
    let HOST = env::var("HOST").expect("No HOST given!").parse::<SocketAddr>().unwrap();
    let CHALLENGERESPONSE = concat_bstring(b"\xFF\xFF\xFF\xFFprint\nET://",
                                           env::var("HOST").unwrap().as_bytes());

    let mut getinfo: Arc<(u32, &mut [u8])> = Arc::new((0, &mut []));

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
                thread::spawn(move || {
                    println!("new thread Spawned.");
                    println!("amt: {}", amt);
                    println!("src: {}", src);
                    println!("{}", to_hex_string(&buf));

                    let s = str::from_utf8(&buf[4..amt]).unwrap_or("Invalid str");
                    println!("{}", s);

                    match s {
                        s if s.starts_with("getinfo") => unimplemented!(),
                        s if s.starts_with("getstatus") => {
                            sock.send_to(&getstatus(HOST).unwrap(), src)
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
