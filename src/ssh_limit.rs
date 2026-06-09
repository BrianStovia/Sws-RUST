use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ConnInfo {
    pid: i32,
    local_ip: String,
    local_port: u16,
    remote_ip: String,
    remote_port: u16,
    proc: String,
}

#[derive(Clone, Debug)]
struct SessionInfo {
    pid: i32,
    username: String,
    real_ip: String,
    session_type: String, // "Dropbear" or "OpenSSH"
}

struct PasswdInfo {
    limits: HashMap<String, i32>,
    uid_map: HashMap<u32, String>,
}

#[cfg(target_os = "linux")]
fn get_proc_uid(pid: i32) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(format!("/proc/{}", pid))
        .map(|meta| meta.uid())
        .ok()
}

#[cfg(not(target_os = "linux"))]
fn get_proc_uid(_pid: i32) -> Option<u32> {
    None
}

fn parse_ip_port(s: &str) -> (String, u16) {
    if let Some(idx) = s.rfind(':') {
        let ip_part = &s[..idx];
        let port_part = &s[idx + 1..];
        let ip = ip_part.replace('[', "").replace(']', "");
        let port = port_part.parse::<u16>().unwrap_or(0);
        (ip, port)
    } else {
        (s.to_string(), 0)
    }
}

fn parse_users_column(users_str: &str) -> Vec<(String, i32)> {
    let mut result = Vec::new();
    let mut current = users_str;
    while let Some(pid_idx) = current.find("pid=") {
        let before = &current[..pid_idx];
        if let Some(quote2) = before.rfind('"') {
            if let Some(quote1) = before[..quote2].rfind('"') {
                let proc_name = &before[quote1 + 1..quote2];
                let after = &current[pid_idx + 4..];
                let mut len = 0;
                for c in after.chars() {
                    if c.is_ascii_digit() {
                        len += 1;
                    } else {
                        break;
                    }
                }
                if len > 0 {
                    if let Ok(pid) = after[..len].parse::<i32>() {
                        result.push((proc_name.to_string(), pid));
                    }
                }
            }
        }
        current = &current[pid_idx + 4..];
    }
    result
}

fn get_ss_connections() -> (Vec<ConnInfo>, HashMap<u16, String>) {
    let mut connections = Vec::new();
    let mut stunnel_map = HashMap::new();
    
    let output = match Command::new("ss").args(&["-tnp"]).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(_) => return (connections, stunnel_map),
    };
    
    let mut stunnel_groups: HashMap<i32, Vec<ConnInfo>> = HashMap::new();
    
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(users_idx) = trimmed.find("users:(") {
            let before_users = &trimmed[..users_idx];
            let users_str = &trimmed[users_idx..];
            
            let cols: Vec<&str> = before_users.split_whitespace().collect();
            if cols.len() < 2 {
                continue;
            }
            let local_col = cols[cols.len() - 2];
            let remote_col = cols[cols.len() - 1];
            
            let (local_ip, local_port) = parse_ip_port(local_col);
            let (remote_ip, remote_port) = parse_ip_port(remote_col);
            
            let procs = parse_users_column(users_str);
            for (proc_name, pid) in procs {
                let conn = ConnInfo {
                    pid,
                    local_ip: local_ip.clone(),
                    local_port,
                    remote_ip: remote_ip.clone(),
                    remote_port,
                    proc: proc_name.clone(),
                };
                connections.push(conn.clone());
                
                if proc_name.to_lowercase().contains("stunnel") {
                    stunnel_groups.entry(pid).or_default().push(conn);
                }
            }
        }
    }
    
    for (_pid, sockets) in stunnel_groups {
        let mut public_ip = None;
        let mut local_ports = Vec::new();
        
        for s in sockets {
            let is_local = s.remote_ip == "127.0.0.1" || s.remote_ip == "::1" || s.remote_ip == "localhost";
            if is_local && (s.remote_port == 22 || s.remote_port == 111 || s.remote_port == 109 || s.remote_port == 3303) {
                local_ports.push(s.local_port);
            } else if !is_local {
                public_ip = Some(s.remote_ip.clone());
            }
        }
        
        if let Some(pub_ip) = public_ip {
            if !local_ports.is_empty() {
                for lp in local_ports {
                    stunnel_map.insert(lp, pub_ip.clone());
                }
            }
        }
    }
    
    (connections, stunnel_map)
}

fn get_ws_mappings() -> HashMap<u16, String> {
    let mut ws_map = HashMap::new();
    let path = if Path::new("/dev/shm").exists() {
        "/dev/shm/ws-ports.txt"
    } else {
        "ws-ports.txt"
    };
    
    if Path::new(path).exists() {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.trim().splitn(2, ':').collect();
                if parts.len() == 2 {
                    if let Ok(port) = parts[0].parse::<u16>() {
                        ws_map.insert(port, parts[1].to_string());
                    }
                }
            }
        }
    }
    ws_map
}

fn parse_passwd() -> PasswdInfo {
    let mut limits = HashMap::new();
    let mut uid_map = HashMap::new();
    let path = "/etc/passwd";
    
    if Path::new(path).exists() {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.trim().split(':').collect();
                if parts.len() >= 5 {
                    let username = parts[0].to_string();
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        uid_map.insert(uid, username.clone());
                    }
                    
                    let gecos = parts[4];
                    let limit_opt = if let Some(limit_idx) = gecos.find("limit=") {
                        let after = &gecos[limit_idx + 6..];
                        let len = after.chars().take_while(|c| c.is_ascii_digit()).count();
                        after[..len].parse::<i32>().ok()
                    } else if let Some(limit_idx) = gecos.find("limit:") {
                        let after = &gecos[limit_idx + 6..];
                        let len = after.chars().take_while(|c| c.is_ascii_digit()).count();
                        after[..len].parse::<i32>().ok()
                    } else {
                        None
                    };
                    
                    if let Some(lim) = limit_opt {
                        limits.insert(username, lim);
                    }
                }
            }
        }
    }
    PasswdInfo { limits, uid_map }
}

fn get_dropbear_log_lines() -> Vec<String> {
    let mut log_lines = Vec::new();
    
    if let Ok(output) = Command::new("journalctl")
        .args(&["-u", "dropbear", "-n", "500", "--no-pager"])
        .output()
    {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                log_lines.push(line.to_string());
            }
        }
    }
    
    if log_lines.is_empty() && Path::new("/var/log/auth.log").exists() {
        if let Ok(content) = fs::read_to_string("/var/log/auth.log") {
            let lines: Vec<&str> = content.lines().collect();
            let start = if lines.len() > 500 { lines.len() - 500 } else { 0 };
            for &line in &lines[start..] {
                log_lines.push(line.to_string());
            }
        }
    }
    
    log_lines
}

fn get_active_sessions(
    connections: Vec<ConnInfo>,
    stunnel_map: HashMap<u16, String>,
    ws_map: HashMap<u16, String>,
    uid_map: &HashMap<u32, String>,
) -> Vec<SessionInfo> {
    let mut sessions = Vec::new();
    let mut dropbear_logins: HashMap<i32, (String, String, i32)> = HashMap::new();
    
    let log_lines = get_dropbear_log_lines();
    for line in log_lines {
        if let Some(db_idx) = line.find("dropbear[") {
            let after_db = &line[db_idx + 9..];
            if let Some(close_bracket_idx) = after_db.find(']') {
                if let Ok(pid) = after_db[..close_bracket_idx].parse::<i32>() {
                    let msg = &after_db[close_bracket_idx + 1..];
                    if msg.contains("Password auth succeeded for '") {
                        if let Some(user_start) = msg.find("Password auth succeeded for '") {
                            let user_part = &msg[user_start + 29..];
                            if let Some(user_end) = user_part.find('\'') {
                                let username = &user_part[..user_end];
                                let from_part = &user_part[user_end + 1..];
                                if let Some(from_idx) = from_part.find(" from ") {
                                    let ip_port = &from_part[from_idx + 6..];
                                    let parts: Vec<&str> = ip_port.trim().split(':').collect();
                                    if parts.len() == 2 {
                                        let ip = parts[0];
                                        if let Ok(port) = parts[1].parse::<i32>() {
                                            dropbear_logins.insert(pid, (username.to_string(), ip.to_string(), port));
                                        }
                                    }
                                }
                            }
                        }
                    } else if msg.contains("Exit (") {
                        dropbear_logins.remove(&pid);
                    }
                }
            }
        }
    }
    
    for (pid, (user, ip, port)) in dropbear_logins {
        let proc_path = format!("/proc/{}", pid);
        if !Path::new(&proc_path).exists() {
            continue;
        }
        
        let mut real_ip = ip.clone();
        let is_local = ip == "127.0.0.1" || ip == "::1" || ip == "localhost";
        if is_local {
            if let Some(mapped_ip) = ws_map.get(&(port as u16)) {
                real_ip = mapped_ip.clone();
            } else if let Some(mapped_ip) = stunnel_map.get(&(port as u16)) {
                real_ip = mapped_ip.clone();
            }
        }
        
        sessions.push(SessionInfo {
            pid,
            username: user,
            real_ip,
            session_type: "Dropbear".to_string(),
        });
    }
    
    for conn in connections {
        if conn.proc == "sshd" || conn.proc == "sshd-session" {
            let pid = conn.pid;
            if sessions.iter().any(|s| s.pid == pid) {
                continue;
            }
            
            if let Some(uid) = get_proc_uid(pid) {
                if uid < 1000 {
                    continue;
                }
                
                if let Some(username) = uid_map.get(&uid) {
                    if username == "root" || username == "sshd" || username == "nobody" {
                        continue;
                    }
                    
                    let mut real_ip = conn.remote_ip.clone();
                    let is_local = real_ip == "127.0.0.1" || real_ip == "::1" || real_ip == "localhost";
                    if is_local {
                        let port = conn.remote_port;
                        if let Some(mapped_ip) = ws_map.get(&port) {
                            real_ip = mapped_ip.clone();
                        } else if let Some(mapped_ip) = stunnel_map.get(&port) {
                            real_ip = mapped_ip.clone();
                        }
                    }
                    
                    sessions.push(SessionInfo {
                        pid,
                        username: username.clone(),
                        real_ip,
                        session_type: "OpenSSH".to_string(),
                    });
                }
            }
        }
    }
    
    sessions
}

fn check_logins(sessions: &[SessionInfo]) {
    let mut user_ips: HashMap<String, HashSet<String>> = HashMap::new();
    for s in sessions {
        user_ips
            .entry(s.username.clone())
            .or_default()
            .insert(s.real_ip.clone());
    }
    
    println!("\033[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m");
    println!("     =[ SSH User Login ]=         ");
    println!("\033[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m");
    for (user, ips) in &user_ips {
        let ips_vec: Vec<String> = ips.iter().cloned().collect();
        println!("\033[33;1mUser\033[32;1m  : {}", user);
        println!("\033[33;1mLogin\033[32;1m : {} IP Login ({})", ips.len(), ips_vec.join(", "));
        println!("\033[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m");
    }
    println!("{} User Online", user_ips.len());
    println!("\033[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m");
}

fn kill_pid(pid: i32) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("kill")
            .args(&["-9", &pid.to_string()])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("kill exited with code: {:?}", status.code()),
            ))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("Mocking: kill -9 {}", pid);
        Ok(())
    }
}

fn enforce_limits(sessions: &[SessionInfo], limits: &HashMap<String, i32>, default_limit: i32) {
    let mut user_sessions: HashMap<String, Vec<&SessionInfo>> = HashMap::new();
    for s in sessions {
        user_sessions.entry(s.username.clone()).or_default().push(s);
    }
    
    for (user, sess_list) in user_sessions {
        let limit = limits.get(&user).copied().unwrap_or(default_limit);
        
        let mut ip_sessions: HashMap<String, Vec<&SessionInfo>> = HashMap::new();
        for s in sess_list {
            ip_sessions.entry(s.real_ip.clone()).or_default().push(s);
        }
        
        let mut unique_ips: Vec<String> = ip_sessions.keys().cloned().collect();
        if unique_ips.len() <= limit as usize {
            continue;
        }
        
        unique_ips.sort_by_key(|ip| {
            ip_sessions.get(ip)
                .and_then(|sesss| sesss.iter().map(|s| s.pid).min())
                .unwrap_or(i32::MAX)
        });
        
        let ips_to_keep = &unique_ips[..limit as usize];
        let ips_to_kill = &unique_ips[limit as usize..];
        
        println!(
            "User '{}' exceeds limit ({}/{} IPs). Keeping: {:?}. Killing: {:?}.",
            user,
            unique_ips.len(),
            limit,
            ips_to_keep,
            ips_to_kill
        );
        
        for ip in ips_to_kill {
            if let Some(sesss) = ip_sessions.get(ip) {
                for s in sesss {
                    let pid = s.pid;
                    println!("  Killing PID {} ({} connection from {})", pid, s.session_type, ip);
                    if let Err(e) = kill_pid(pid) {
                        println!("  Error killing PID {}: {}", pid, e);
                    }
                }
            }
        }
    }
}

fn main() {
    let (connections, stunnel_map) = get_ss_connections();
    let ws_map = get_ws_mappings();
    let passwd_info = parse_passwd();
    
    let sessions = get_active_sessions(connections, stunnel_map, ws_map, &passwd_info.uid_map);
    
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--check" {
        check_logins(&sessions);
    } else {
        enforce_limits(&sessions, &passwd_info.limits, 2);
    }
}
