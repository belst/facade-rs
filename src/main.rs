use std::str;
use std::fmt::Write;
use std::thread;
use std::sync::Arc;
use std::net::{UdpSocket, ToSocketAddrs};

fn to_hex_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &byte in bytes {
        write!(&mut s, "{:X} ", byte).unwrap();
    }
    s
}

fn getstatus<A: ToSocketAddrs>(target: A) -> Result<Vec<u8>, String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => panic!("Could not bind socket: {}", e),
    };
    match socket.send_to(b"\xFF\xFF\xFF\xFFgetstatus\n", target) {
        Ok(..) => {}
        Err(e) => return Err(format!("{}", e)),
    }
    let mut buf = [0; 2048];
    match socket.recv_from(&mut buf) {
        Ok((amt, _)) => {
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

    const PREFIX: &'static [u8; 4] = b"\xFF\xFF\xFF\xFF";
    const LISTEN: &'static str = "0.0.0.0:5555";
    const HOST: &'static str = "94.23.7.172:27960";
    const CHALLENGERESPONSE: &'static [u8] = b"\xFF\xFF\xFF\xFFprint\nET://94.23.7.172:27960";

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
                        s if s.starts_with("getchallenge") => sock.send_to(CHALLENGERESPONSE, src),
                        _ => panic!("Invalid request type"),
                    }
                });
            }
            Err(e) => println!("Could not receive a packet: {}", e),
        }
    }
}
