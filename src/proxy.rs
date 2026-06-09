use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

// Set large OS socket send/receive buffers (8 MB each) to prevent
// kernel-level buffering from capping throughput on fast links.
#[cfg(unix)]
fn set_socket_buffers(stream: &TcpStream) {
    use std::os::unix::io::AsRawFd;
    let fd = stream.as_raw_fd();
    let buf_size: libc::c_int = 8 * 1024 * 1024; // 8 MB
    unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_SNDBUF,
            &buf_size as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVBUF,
            &buf_size as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
    }
}

#[cfg(not(unix))]
fn set_socket_buffers(_stream: &TcpStream) {}

static WS_LOCK: Mutex<()> = Mutex::new(());

fn get_ws_ports_path() -> &'static str {
    if Path::new("/dev/shm").exists() {
        "/dev/shm/ws-ports.txt"
    } else {
        "ws-ports.txt"
    }
}

fn add_ws_mapping(local_port: u16, real_ip: &str) {
    let _lock = WS_LOCK.lock().unwrap();
    let path = get_ws_ports_path();
    
    if let Some(parent) = Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}:{}", local_port, real_ip);
    }
}

fn remove_ws_mapping(local_port: u16) {
    let _lock = WS_LOCK.lock().unwrap();
    let path = get_ws_ports_path();
    if !Path::new(path).exists() {
        return;
    }
    
    if let Ok(content) = fs::read_to_string(path) {
        let prefix = format!("{}:", local_port);
        let mut new_lines = Vec::new();
        for line in content.lines() {
            if !line.starts_with(&prefix) {
                new_lines.push(line);
            }
        }
        let new_content = new_lines.join("\n") + if new_lines.is_empty() { "" } else { "\n" };
        let _ = fs::write(path, new_content);
    }
}

// Pure Rust SHA-1 Implementation
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let orig_len = data.len();
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() * 8) % 512 != 448 {
        msg.push(0);
    }
    let bits = (orig_len as u64) * 8;
    msg.extend_from_slice(&bits.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | (!b & d), 0x5A827999)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ED9EBA1)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8F1BBCDC)
            } else {
                (b ^ c ^ d, 0xCA62C1D6)
            };

            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

// Pure Rust Base64 Encoder
fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut chunks = data.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        result.push(CHARSET[(n & 63) as usize] as char);
    }
    let remainder = chunks.remainder();
    if remainder.len() == 1 {
        let n = (remainder[0] as u32) << 16;
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push('=');
        result.push('=');
    } else if remainder.len() == 2 {
        let n = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        result.push('=');
    }
    result
}

fn read_client_initial(client: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 65536];
    
    client.set_read_timeout(Some(Duration::from_millis(1000)))?;
    
    loop {
        match client.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&temp[..n]);
                
                let has_ssh = buffer.windows(4).any(|w| {
                    (w[0] == b's' || w[0] == b'S') &&
                    (w[1] == b's' || w[1] == b'S') &&
                    (w[2] == b'h' || w[2] == b'H') &&
                    w[3] == b'-'
                });
                if has_ssh {
                    break;
                }
                
                if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                    let _ = client.set_read_timeout(Some(Duration::from_millis(100)));
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                break;
            }
            Err(e) => return Err(e),
        }
    }
    
    client.set_read_timeout(Some(Duration::from_secs(60)))?;
    Ok(buffer)
}

fn find_ssh_idx(buf: &[u8]) -> Option<usize> {
    if buf.len() < 4 {
        return None;
    }
    for i in 0..=buf.len() - 4 {
        let w = &buf[i..i+4];
        if (w[0] == b's' || w[0] == b'S') &&
           (w[1] == b's' || w[1] == b'S') &&
           (w[2] == b'h' || w[2] == b'H') &&
           w[3] == b'-' {
            return Some(i);
        }
    }
    None
}

fn split_parts(payload: &[u8]) -> Vec<Vec<u8>> {
    let mut parts = Vec::new();
    let mut current = 0;
    while current < payload.len() {
        let mut found_idx = None;
        if payload.len() - current >= 4 {
            for i in current..=payload.len() - 4 {
                if &payload[i..i+4] == b"\r\n\r\n" {
                    found_idx = Some(i);
                    break;
                }
            }
        }
        
        match found_idx {
            Some(idx) => {
                let part = &payload[current..idx];
                if !part.iter().all(|&b| b.is_ascii_whitespace()) {
                    parts.push(part.to_vec());
                }
                current = idx + 4;
            }
            None => {
                let part = &payload[current..];
                if !part.iter().all(|&b| b.is_ascii_whitespace()) {
                    parts.push(part.to_vec());
                }
                break;
            }
        }
    }
    parts
}

fn find_header(head: &[u8], header_name: &str) -> String {
    let head_str = String::from_utf8_lossy(head);
    let header_lower = header_name.to_lowercase() + ":";
    for line in head_str.lines() {
        if line.to_lowercase().starts_with(&header_lower) {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                return parts[1].trim().to_string();
            }
        }
    }
    String::new()
}

fn handle_connection(mut client: TcpStream) {
    let peer_addr = match client.peer_addr() {
        Ok(addr) => addr,
        Err(_) => return,
    };
    
    let buffer = match read_client_initial(&mut client) {
        Ok(buf) => buf,
        Err(e) => {
            println!("Error reading initial payload from {:?}: {}", peer_addr, e);
            return;
        }
    };
    
    let ssh_idx = find_ssh_idx(&buffer);
    let (http_payload, leftover) = match ssh_idx {
        Some(idx) => (&buffer[..idx], &buffer[idx..]),
        None => (buffer.as_slice(), &[][..]),
    };
    
    let parts = split_parts(http_payload);
    let mut real_req = Vec::new();
    for part in parts.iter().rev() {
        let part_lower: Vec<u8> = part.iter().map(|&b| b.to_ascii_lowercase()).collect();
        let has_upgrade = part_lower.windows(7).any(|w| w == b"upgrade");
        let starts_connect = part_lower.starts_with(b"connect");
        
        if has_upgrade || starts_connect {
            real_req = part.clone();
            break;
        }
    }
    if real_req.is_empty() && !parts.is_empty() {
        real_req = parts.last().unwrap().clone();
    }
    
    let mut is_connect = false;
    let mut request_host = String::new();
    
    let req_str = String::from_utf8_lossy(&real_req);
    if let Some(first_line) = req_str.lines().next() {
        let req_parts: Vec<&str> = first_line.split_whitespace().collect();
        if req_parts.len() >= 2 {
            if req_parts[0].to_uppercase() == "CONNECT" {
                is_connect = true;
                request_host = req_parts[1].to_string();
            }
        }
    }
    
    let host_port = if !request_host.is_empty() {
        request_host
    } else {
        let real_host = find_header(&real_req, "X-Real-Host");
        if !real_host.is_empty() {
            real_host
        } else {
            let host = find_header(&real_req, "Host");
            if !host.is_empty() {
                host
            } else {
                "127.0.0.1:111".to_string()
            }
        }
    };
    
    let passwd = find_header(&real_req, "X-Pass");
    let global_pass = ""; // Enforced if not empty
    if !global_pass.is_empty() && passwd != global_pass {
        let _ = client.write_all(b"HTTP/1.1 400 WrongPass!\r\n\r\n");
        return;
    }
    
    let port = if let Some(colon_idx) = host_port.rfind(':') {
        &host_port[colon_idx + 1..]
    } else {
        "111"
    };
    
    let enforced_port = match port {
        "22" | "109" | "110" | "111" | "3303" => port,
        _ => "111",
    };
    
    let target_host = format!("127.0.0.1:{}", enforced_port);
    
    let mut target = match TcpStream::connect(&target_host) {
        Ok(t) => {
            let _ = t.set_nodelay(true);
            #[cfg(unix)]
            set_socket_buffers(&t);
            t
        }
        Err(e) => {
            println!("Error connecting to target {}: {}", target_host, e);
            return;
        }
    };
    
    // Also apply no-delay + large buffers to the client socket
    let _ = client.set_nodelay(true);
    #[cfg(unix)]
    set_socket_buffers(&client);
    
    let local_port = match target.local_addr() {
        Ok(addr) => addr.port(),
        Err(_) => 0,
    };
    
    let mut real_ip = find_header(&real_req, "X-Real-IP");
    if real_ip.is_empty() {
        real_ip = find_header(&real_req, "X-Forwarded-For");
    }
    if !real_ip.is_empty() {
        if let Some(first_ip) = real_ip.split(',').next() {
            real_ip = first_ip.trim().to_string();
        }
    } else {
        real_ip = peer_addr.ip().to_string();
    }
    
    if local_port > 0 {
        add_ws_mapping(local_port, &real_ip);
    }
    
    if !leftover.is_empty() {
        let _ = target.write_all(leftover);
    }
    
    if is_connect {
        let _ = client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n");
    } else {
        let sec_ws_key = find_header(&real_req, "Sec-WebSocket-Key");
        if !sec_ws_key.is_empty() {
            let magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
            let combined = sec_ws_key + magic;
            let hash = sha1(combined.as_bytes());
            let accept_val = base64_encode(&hash);
            
            let dyn_response = format!(
                "HTTP/1.1 101 Switching Protocols\r\n\
                 Upgrade: websocket\r\n\
                 Connection: Upgrade\r\n\
                 Server: rbstv-Proxy\r\n\
                 Sec-WebSocket-Accept: {}\r\n\r\n",
                accept_val
            );
            let _ = client.write_all(dyn_response.as_bytes());
        } else {
            let static_response = b"HTTP/1.1 101 Switching Protocols\r\n\
                                    Upgrade: websocket\r\n\
                                    Connection: Upgrade\r\n\
                                    Server: rbstv-Proxy\r\n\
                                    Sec-WebSocket-Accept: foo\r\n\r\n";
            let _ = client.write_all(static_response);
        }
    }
    
    println!("Connection: {:?} - CONNECT {}", peer_addr, target_host);
    
    let _ = client.set_read_timeout(Some(Duration::from_secs(300)));
    let _ = target.set_read_timeout(Some(Duration::from_secs(300)));
    
    let mut client_read = match client.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut target_read = match target.try_clone() {
        Ok(t) => t,
        Err(_) => return,
    };
    
    let mut client_write = client;
    let mut target_write = target;
    
    let t1 = std::thread::spawn(move || {
        let mut buf = [0u8; 262144];
        loop {
            match client_read.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if target_write.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = target_write.shutdown(std::net::Shutdown::Write);
    });
    
    let t2 = std::thread::spawn(move || {
        let mut buf = [0u8; 262144];
        loop {
            match target_read.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if client_write.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = client_write.shutdown(std::net::Shutdown::Write);
    });
    
    let _ = t1.join();
    let _ = t2.join();
    
    if local_port > 0 {
        remove_ws_mapping(local_port);
    }
}

fn print_usage() {
    println!("Usage: proxy -p <port>");
    println!("       proxy -b <bindAddr> -p <port>");
    println!("       proxy -b 0.0.0.0 -p 80");
}

fn parse_args() -> (String, u16) {
    let args: Vec<String> = std::env::args().collect();
    let mut bind_addr = "127.0.0.1".to_string();
    let mut port = 700;
    
    if args.len() > 1 {
        if args.len() == 2 && !args[1].starts_with('-') {
            if let Ok(p) = args[1].parse::<u16>() {
                return (bind_addr, p);
            }
        }
        
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--help" => {
                    print_usage();
                    std::process::exit(0);
                }
                "-b" | "--bind" => {
                    if i + 1 < args.len() {
                        bind_addr = args[i + 1].clone();
                        i += 2;
                    } else {
                        eprintln!("Error: Missing value for bind address");
                        print_usage();
                        std::process::exit(2);
                    }
                }
                "-p" | "--port" => {
                    if i + 1 < args.len() {
                        if let Ok(p) = args[i + 1].parse::<u16>() {
                            port = p;
                        } else {
                            eprintln!("Error: Invalid port number: {}", args[i + 1]);
                            std::process::exit(2);
                        }
                        i += 2;
                    } else {
                        eprintln!("Error: Missing value for port");
                        print_usage();
                        std::process::exit(2);
                    }
                }
                _ => {
                    eprintln!("Warning: Unrecognized argument: {}", args[i]);
                    i += 1;
                }
            }
        }
    }
    (bind_addr, port)
}

fn main() {
    let (bind_addr, port) = parse_args();
    
    println!("\n:-------RustProxy-------:");
    println!("Listening addr: {}", bind_addr);
    println!("Listening port: {}", port);
    println!(":-------------------------:");
    
    let addr = format!("{}:{}", bind_addr, port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    
    for stream_res in listener.incoming() {
        match stream_res {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                std::thread::spawn(move || {
                    handle_connection(stream);
                });
            }
            Err(e) => {
                eprintln!("Failed to accept incoming connection: {}", e);
            }
        }
    }
}
