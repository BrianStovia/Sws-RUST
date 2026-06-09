use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::SystemTime;

fn format_utc_timestamp(secs: u64) -> String {
    let days = secs / 86400;
    let sec_of_day = secs % 86400;
    let hour = sec_of_day / 3600;
    let min = (sec_of_day % 3600) / 60;
    let sec = sec_of_day % 60;
    
    let mut year = 1970;
    let mut days_left = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }
    
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    let mut month = 1;
    for &d in &month_days {
        if days_left < d {
            break;
        }
        days_left -= d;
        month += 1;
    }
    let day = days_left + 1;
    
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, min, sec)
}

fn get_timestamp() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format_utc_timestamp(d.as_secs()),
        Err(_) => "1970-01-01 00:00:00".to_string(),
    }
}

fn write_log(level: &str, message: &str) {
    let ts = get_timestamp();
    let log_line = format!("{} - {} - {}\n", ts, level, message);
    
    print!("{}", log_line);
    
    let log_path = "/var/log/api.log";
    let fallback_path = "api.log";
    
    let target_path = if Path::new("/var/log").exists() {
        if let Err(_) = fs::create_dir_all("/var/log") {}
        log_path
    } else {
        fallback_path
    };
    
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(target_path) {
        let _ = file.write_all(log_line.as_bytes());
    } else if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(fallback_path) {
        let _ = file.write_all(log_line.as_bytes());
    }
}

fn load_valid_tokens() -> Vec<String> {
    let key_path = "/etc/api/key";
    let fallback_path = "key.txt";
    
    let path = if Path::new(key_path).exists() {
        key_path
    } else {
        fallback_path
    };
    
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        write_log("WARNING", &format!("Could not read API keys from {}. No tokens loaded.", path));
        Vec::new()
    }
}

fn read_headers(stream: &mut TcpStream) -> std::io::Result<Option<(String, Vec<u8>)>> {
    let mut header_buf = Vec::new();
    let mut temp = [0u8; 1];
    
    loop {
        if stream.read(&mut temp)? == 0 {
            return Ok(None);
        }
        header_buf.push(temp[0]);
        if header_buf.len() >= 8192 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Headers too large"));
        }
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    
    let headers_str = String::from_utf8_lossy(&header_buf).into_owned();
    Ok(Some((headers_str, header_buf)))
}

fn handle_connection(mut stream: TcpStream) {
    let client_ip = match stream.peer_addr() {
        Ok(addr) => addr.ip().to_string(),
        Err(_) => "unknown".to_string(),
    };
    
    let headers_opt = match read_headers(&mut stream) {
        Ok(Some(h)) => Some(h),
        _ => None,
    };
    
    if headers_opt.is_none() {
        return;
    }
    let (headers_str, _) = headers_opt.unwrap();
    
    let mut lines = headers_str.lines();
    let req_line = match lines.next() {
        Some(line) => line,
        None => return,
    };
    
    let req_parts: Vec<&str> = req_line.split_whitespace().collect();
    if req_parts.len() < 2 {
        return;
    }
    let method = req_parts[0];
    let path = req_parts[1];
    
    let mut user_agent = "User-Agent not provided".to_string();
    let mut auth_header = None;
    let mut content_length = 0;
    
    for line in lines {
        if line.is_empty() {
            break;
        }
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_ascii_lowercase();
            let val = parts[1].trim();
            if key == "user-agent" {
                user_agent = val.to_string();
            } else if key == "authorization" {
                auth_header = Some(val.to_string());
            } else if key == "content-length" {
                content_length = val.parse::<usize>().unwrap_or(0);
            }
        }
    }
    
    let log_request_info = |status: &str, detail: &str| {
        let msg = format!("Access from IP: {}, User-Agent: {}, Path: {}, {} ({})", client_ip, user_agent, path, detail, status);
        write_log("INFO", &msg);
    };
    
    let valid_tokens = load_valid_tokens();
    let mut authorized = false;
    if let Some(auth) = auth_header {
        if auth.starts_with("Bearer ") {
            let token = auth[7..].trim();
            if valid_tokens.contains(&token.to_string()) {
                authorized = true;
            }
        }
    }
    
    if !authorized {
        log_request_info("401", "Unauthorized access attempt");
        write_log("WARNING", "Unauthorized access attempt");
        
        let response = concat!(
            "HTTP/1.1 401 Unauthorized\r\n",
            "WWW-Authenticate: Bearer realm=\"Authentication required\"\r\n",
            "Content-Type: application/json\r\n",
            "Content-Length: 59\r\n",
            "Connection: close\r\n\r\n",
            "{\"message\": \"Unauthorized: Missing or invalid Bearer token\"}"
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }
    
    if method == "OPTIONS" {
        let body = "{\"message\": \"OPTIONS request received\"}";
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Allow: GET, POST, DELETE, PUT, PATCH, CONNECT, TRACE, HEAD, OPTIONS\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        log_request_info("200", "OPTIONS request processed");
        return;
    }
    
    let mut post_data = Vec::new();
    let method_upper = method.to_uppercase();
    if method_upper == "POST" || method_upper == "PUT" || method_upper == "PATCH" || method_upper == "DELETE" {
        if content_length > 0 {
            post_data.resize(content_length, 0);
            if let Err(e) = stream.read_exact(&mut post_data) {
                write_log("ERROR", &format!("Failed to read request body: {}", e));
                return;
            }
        }
    }
    
    let path_str = path.trim_start_matches('/');
    if path_str.contains("..") || path_str.contains('\\') {
        let body = "Invalid path";
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        log_request_info("400", "Invalid path traversal attempt");
        return;
    }
    
    let base_api_dir = "/usr/local/sbin/api";
    let fallback_api_dir = "api";
    
    let target_dir = if Path::new(base_api_dir).exists() {
        base_api_dir
    } else {
        fallback_api_dir
    };
    
    let script_path = Path::new(target_dir).join(path_str);
    
    if !script_path.is_file() {
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 16\r\nConnection: close\r\n\r\nScript not found";
        let _ = stream.write_all(response.as_bytes());
        log_request_info("404", &format!("Script not found: {:?}", script_path));
        write_log("ERROR", &format!("Script not found: {:?}", script_path));
        return;
    }
    
    let mut cmd = Command::new(&script_path);
    if !post_data.is_empty() {
        cmd.stdin(Stdio::piped());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    
    match cmd.spawn() {
        Ok(mut child) => {
            if !post_data.is_empty() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&post_data);
                }
            }
            
            match child.wait_with_output() {
                Ok(output) => {
                    if output.status.success() {
                        let response_body = output.stdout;
                        let response_header = format!(
                            "HTTP/1.1 200 OK\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n",
                            response_body.len()
                        );
                        let _ = stream.write_all(response_header.as_bytes());
                        let _ = stream.write_all(&response_body);
                        
                        log_request_info("200", &format!("Successfully executed script: {:?}", script_path));
                        let stdout_str = String::from_utf8_lossy(&response_body);
                        write_log("INFO", &format!("Successfully executed script: {:?}, Output: {}", script_path, stdout_str.trim()));
                    } else {
                        let err_msg = String::from_utf8_lossy(&output.stderr);
                        let error_json = format!("{{\"error\": {:?}}}", err_msg.trim());
                        let response = format!(
                            "HTTP/1.1 500 Internal Server Error\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n\
                             {}",
                            error_json.len(),
                            error_json
                        );
                        let _ = stream.write_all(response.as_bytes());
                        log_request_info("500", &format!("Error executing script: {:?}, Exit Code: {:?}", script_path, output.status.code()));
                        write_log("ERROR", &format!("Error executing script: {:?}, Error: {}", script_path, err_msg.trim()));
                    }
                }
                Err(e) => {
                    let error_json = format!("{{\"error\": {:?}}}", e.to_string());
                    let response = format!(
                        "HTTP/1.1 500 Internal Server Error\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: {}\r\n\
                         Connection: close\r\n\r\n\
                         {}",
                        error_json.len(),
                        error_json
                    );
                    let _ = stream.write_all(response.as_bytes());
                    log_request_info("500", &format!("Failed to wait for script execution: {:?}", e));
                    write_log("ERROR", &format!("Error executing script: {:?}, Error: {}", script_path, e));
                }
            }
        }
        Err(e) => {
            let error_json = format!("{{\"error\": {:?}}}", e.to_string());
            let response = format!(
                "HTTP/1.1 500 Internal Server Error\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n\
                 {}",
                error_json.len(),
                error_json
            );
            let _ = stream.write_all(response.as_bytes());
            log_request_info("500", &format!("Failed to spawn script execution: {:?}", e));
            write_log("ERROR", &format!("Error executing script: {:?}, Error: {}", script_path, e));
        }
    }
}

fn main() {
    let port = 9000;
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    
    write_log("INFO", &format!("Starting httpd server on port {}", port));
    
    for stream_res in listener.incoming() {
        match stream_res {
            Ok(stream) => {
                std::thread::spawn(|| {
                    handle_connection(stream);
                });
            }
            Err(e) => {
                write_log("ERROR", &format!("Failed to accept connection: {}", e));
            }
        }
    }
}
