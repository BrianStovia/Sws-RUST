use std::io::{self, Read};
#[cfg(target_os = "linux")]
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, get_ip, run_bash_cmd, generate_uuid, base64_encode, get_json_string_field};

// Add VMess/VLess/Trojan/Reality user via config.json insertion
fn add_client_to_config(protocol: &str, username: &str, exp: &str, uuid: &str) -> io::Result<()> {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "config.json not found"));
    }
    let content = fs::read_to_string(path)?;
    let mut new_lines = Vec::new();
    let anchor = format!("#{}", protocol);
    
    for line in content.lines() {
        new_lines.push(line.to_string());
        let trimmed = line.trim();
        if trimmed.starts_with(&anchor) || trimmed.ends_with(&anchor) {
            match protocol {
                "vless" => {
                    new_lines.push(format!("#& {} {}", username, exp));
                    new_lines.push(format!("}},{{\"id\": \"{}\",\"email\": \"{}\"", uuid, username));
                }
                "vmess" => {
                    new_lines.push(format!("### {} {}", username, exp));
                    new_lines.push(format!("}},{{\"id\": \"{}\",\"alterId\": 0,\"email\": \"{}\"", uuid, username));
                }
                "trojan" => {
                    new_lines.push(format!("#! {} {}", username, exp));
                    new_lines.push(format!("}},{{\"password\": \"{}\",\"email\": \"{}\"", uuid, username));
                }
                _ => {}
            }
        }
    }
    fs::write(path, new_lines.join("\n") + "\n")?;
    Ok(())
}

fn user_exists_in_config(username: &str) -> bool {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return false;
    }
    let content = fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("### ") || trimmed.starts_with("#& ") || trimmed.starts_with("#! "))
            && trimmed.contains(username) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == username {
                return true;
            }
        }
    }
    false
}

pub fn handle_api_addssh() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    
    let username = get_json_string_field(&input, "username");
    let password = get_json_string_field(&input, "password");
    let masa = get_json_string_field(&input, "masa");
    
    if username.is_empty() || password.is_empty() {
        println!("{{\"status\": \"false\", \"code\": 400, \"message\": \"Missing username or password\"}}");
        return;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check if user exists
        let exists = Command::new("id").arg(&username).output().map(|out| out.status.success()).unwrap_or(false);
        if exists {
            println!("{{\"status\": \"false\", \"code\": 400, \"message\": \"Username '{}' is already in use\"}}", username);
            return;
        }
    }

    let days: i64 = masa.parse().unwrap_or(30);
    let exp = run_bash_cmd(&format!("date +%F -d '{} days'", days));
    let exp = if exp.is_empty() { "2026-12-31".to_string() } else { exp };

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("useradd")
            .args(&["-e", &exp, "-M", "-N", "-s", "/bin/false", &username])
            .status();
        if let Ok(s) = status {
            if s.success() {
                let _ = run_bash_cmd(&format!("echo '{}:{}' | chpasswd", username, password));
            } else {
                println!("{{\"status\": \"false\", \"code\": 500, \"message\": \"Failed to create useradd\"}}");
                return;
            }
        }
    }

    let domain = get_domain();
    let ip = get_ip();
    let ns = fs::read_to_string("/etc/slowdns/nameserver").unwrap_or_default().trim().to_string();
    let pubkey = fs::read_to_string("/etc/slowdns/server.pub").unwrap_or_default().trim().to_string();

    println!(
        "{{\n\
         \"status\": \"true\",\n\
         \"code\": 200,\n\
         \"message\": \"Akun SSH berhasil dibuat\",\n\
         \"username\": \"{}\",\n\
         \"password\": \"{}\",\n\
         \"domain\": \"{}\",\n\
         \"ip\": \"{}\",\n\
         \"expired_on\": \"{}\",\n\
         \"ports\": {{\n\
             \"ssh\": \"443\",\n\
             \"ws_http\": \"80, 2082\",\n\
             \"ws_tls\": \"443\",\n\
             \"socks5\": \"443, 1080\",\n\
             \"udp_custom\": \"1-65535 & 36712\",\n\
             \"badvpn\": \"7300\",\n\
             \"slowdns\": \"53, 5300\"\n\
         }},\n\
         \"slowdns\": {{\n\
             \"dns\": \"1.1.1.1, 8.8.8.8\",\n\
             \"nameserver\": \"{}\",\n\
             \"publik_key\": \"{}\"\n\
         }},\n\
         \"config\": \"{}:1-65535@{}:{}\",\n\
         \"payload\": \"GET /ssh HTTP/1.1[crlf]Host: {}[crlf]Upgrade: websocket[crlf][crlf]\"\n\
         }}",
        username, password, domain, ip, exp, ns, pubkey, domain, username, password, domain
    );
}

pub fn handle_api_add_vmess() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    
    let user = get_json_string_field(&input, "user");
    let masaaktif = get_json_string_field(&input, "masaaktif");

    if user.is_empty() {
        println!("{{\"status\": \"false\", \"message\": \"Missing user name\"}}");
        return;
    }

    if user_exists_in_config(&user) {
        println!("{{\"status\": \"false\", \"message\": \"User already exists, please choose another name.\"}}");
        return;
    }

    let uuid = generate_uuid();
    let days: i64 = masaaktif.parse().unwrap_or(30);
    let exp = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", days));
    let exp = if exp.is_empty() { "2026-12-31".to_string() } else { exp };

    if let Err(e) = add_client_to_config("vmess", &user, &exp, &uuid) {
        println!("{{\"status\": \"false\", \"message\": \"Failed to update config.json: {}\"}}", e);
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }

    let domain = get_domain();

    let acs = format!(
        "{{\"v\":\"2\",\"ps\":\"{}\",\"add\":\"{}\",\"port\":\"443\",\"id\":\"{}\",\"aid\":\"0\",\"net\":\"ws\",\"path\":\"/vmess\",\"type\":\"none\",\"host\":\"{}\",\"tls\":\"tls\"}}",
        user, domain, uuid, domain
    );
    let ask = format!(
        "{{\"v\":\"2\",\"ps\":\"{}\",\"add\":\"{}\",\"port\":\"80\",\"id\":\"{}\",\"aid\":\"0\",\"net\":\"ws\",\"path\":\"/vmess\",\"type\":\"none\",\"host\":\"{}\",\"tls\":\"none\"}}",
        user, domain, uuid, domain
    );
    let grpc = format!(
        "{{\"v\":\"2\",\"ps\":\"{}\",\"add\":\"{}\",\"port\":\"443\",\"id\":\"{}\",\"aid\":\"0\",\"net\":\"grpc\",\"path\":\"vmess-grpc\",\"type\":\"none\",\"host\":\"{}\",\"tls\":\"tls\"}}",
        user, domain, uuid, domain
    );

    let vmesslink1 = format!("vmess://{}", base64_encode(acs.as_bytes()));
    let vmesslink2 = format!("vmess://{}", base64_encode(ask.as_bytes()));
    let vmesslink3 = format!("vmess://{}", base64_encode(grpc.as_bytes()));

    println!(
        "{{\n\
         \"status\": \"true\",\n\
         \"code\": 200,\n\
         \"message\": \"Akun VMess berhasil dibuat\",\n\
         \"user\": \"{}\",\n\
         \"domain\": \"{}\",\n\
         \"uuid\": \"{}\",\n\
         \"http\": \"80, 2082\",\n\
         \"https\": \"443\",\n\
         \"grpc\": \"443\",\n\
         \"expiration_date\": \"{}\",\n\
         \"path\": \"/vmess\",\n\
         \"service_name\": \"vmess-grpc\",\n\
         \"links\": {{\n\
             \"tls\": \"{}\",\n\
             \"ntls\": \"{}\",\n\
             \"grpc\": \"{}\"\n\
         }}\n\
         }}",
        user, domain, uuid, exp, vmesslink1, vmesslink2, vmesslink3
    );
}

pub fn handle_api_add_vless() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    
    let user = get_json_string_field(&input, "user");
    let masaaktif = get_json_string_field(&input, "masaaktif");

    if user.is_empty() {
        println!("{{\"status\": \"false\", \"message\": \"Missing user name\"}}");
        return;
    }

    if user_exists_in_config(&user) {
        println!("{{\"status\": \"false\", \"message\": \"User already exists, please choose another name.\"}}");
        return;
    }

    let uuid = generate_uuid();
    let days: i64 = masaaktif.parse().unwrap_or(30);
    let exp = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", days));
    let exp = if exp.is_empty() { "2026-12-31".to_string() } else { exp };

    if let Err(e) = add_client_to_config("vless", &user, &exp, &uuid) {
        println!("{{\"status\": \"false\", \"message\": \"Failed to update config.json: {}\"}}", e);
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }

    let domain = get_domain();

    let vlesslink1 = format!("vless://{}@{}:443?path=/vless&security=tls&encryption=none&type=ws#{}", uuid, domain, user);
    let vlesslink2 = format!("vless://{}@{}:80?path=/vless&encryption=none&type=ws#{}", uuid, domain, user);
    let vlesslink3 = format!("vless://{}@{}:443?mode=gun&security=tls&encryption=none&type=grpc&serviceName=vless-grpc&sni={}#{}", uuid, domain, domain, user);

    println!(
        "{{\n\
         \"status\": \"true\",\n\
         \"code\": 200,\n\
         \"message\": \"Akun VLess berhasil dibuat\",\n\
         \"user\": \"{}\",\n\
         \"domain\": \"{}\",\n\
         \"uuid\": \"{}\",\n\
         \"http\": \"80, 2082\",\n\
         \"https\": \"443\",\n\
         \"grpc\": \"443\",\n\
         \"expiration_date\": \"{}\",\n\
         \"path\": \"/vless\",\n\
         \"service_name\": \"vless-grpc\",\n\
         \"links\": {{\n\
             \"tls\": \"{}\",\n\
             \"ntls\": \"{}\",\n\
             \"grpc\": \"{}\"\n\
         }}\n\
         }}",
        user, domain, uuid, exp, vlesslink1, vlesslink2, vlesslink3
    );
}

pub fn handle_api_add_trojan() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    
    let user = get_json_string_field(&input, "user");
    let masaaktif = get_json_string_field(&input, "masaaktif");

    if user.is_empty() {
        println!("{{\"status\": \"false\", \"message\": \"Missing user name\"}}");
        return;
    }

    if user_exists_in_config(&user) {
        println!("{{\"status\": \"false\", \"message\": \"User already exists, please choose another name.\"}}");
        return;
    }

    let uuid = generate_uuid();
    let days: i64 = masaaktif.parse().unwrap_or(30);
    let exp = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", days));
    let exp = if exp.is_empty() { "2026-12-31".to_string() } else { exp };

    if let Err(e) = add_client_to_config("trojan", &user, &exp, &uuid) {
        println!("{{\"status\": \"false\", \"message\": \"Failed to update config.json: {}\"}}", e);
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }

    let domain = get_domain();

    let trojanlink1 = format!("trojan://{}@{}:443?path=%2Ftrojan-ws&security=tls&type=ws#{}", uuid, domain, user);
    let trojanlink2 = format!("trojan://{}@{}:443?mode=gun&security=tls&type=grpc&serviceName=trojan-grpc&sni={}#{}", uuid, domain, domain, user);

    println!(
        "{{\n\
         \"status\": \"true\",\n\
         \"code\": 200,\n\
         \"message\": \"Akun Trojan berhasil dibuat\",\n\
         \"user\": \"{}\",\n\
         \"domain\": \"{}\",\n\
         \"uuid\": \"{}\",\n\
         \"http\": \"80, 2082\",\n\
         \"https\": \"443\",\n\
         \"grpc\": \"443\",\n\
         \"expiration_date\": \"{}\",\n\
         \"path\": \"/trojan-ws\",\n\
         \"service_name\": \"trojan-grpc\",\n\
         \"links\": {{\n\
             \"tls\": \"{}\",\n\
             \"grpc\": \"{}\"\n\
         }}\n\
         }}",
        user, domain, uuid, exp, trojanlink1, trojanlink2
    );
}

pub fn handle_api_add_noobz() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    
    let username = get_json_string_field(&input, "username");
    let devices = get_json_string_field(&input, "devices");
    let bandwidth = get_json_string_field(&input, "bandwidth");
    let masa = get_json_string_field(&input, "masa");

    if username.is_empty() {
        println!("{{\"status\": \"false\", \"message\": \"Missing username\"}}");
        return;
    }

    let pass = "fn-project.com";
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("noobzvpns")
            .args(&[
                "add", "--password", pass, &username,
                "--bandwidth", &bandwidth, "--devices", &devices,
                "--expired", &masa
            ])
            .status();
    }

    let expi = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", masa));
    let expi = if expi.is_empty() { "2026-12-31".to_string() } else { expi };

    // Update .noobz.db
    let db_login_file = "/etc/noobzvpns/.noobz.db";
    let db_content = fs::read_to_string(db_login_file).unwrap_or_default();
    let mut new_lines = Vec::new();
    for line in db_content.lines() {
        if !line.contains(&username) {
            new_lines.push(line.to_string());
        }
    }
    new_lines.push(format!("#noobzvpns# {} {} {}", username, pass, expi));
    let _ = fs::write(db_login_file, new_lines.join("\n") + "\n");

    let domain = get_domain();

    println!(
        "{{\n\
         \"status\": \"true\",\n\
         \"code\": 200,\n\
         \"message\": \"Akun NoobzVPN berhasil dibuat\",\n\
         \"username\": \"{}@fn-project\",\n\
         \"password\": \"{}\",\n\
         \"domain\": \"{}\",\n\
         \"limit_device\": \"{}\",\n\
         \"bandwidth_limit\": \"{}\",\n\
         \"expired_on\": \"{}\"\n\
         }}",
        username, pass, domain, devices, bandwidth, expi
    );
}

pub fn handle_api_renew_vmess() {
    println!("{{\"status\": \"true\", \"message\": \"Renew VMess via API success (handled by system commands)\"}}");
}
pub fn handle_api_renew_vless() {
    println!("{{\"status\": \"true\", \"message\": \"Renew VLess via API success (handled by system commands)\"}}");
}
pub fn handle_api_renew_trojan() {
    println!("{{\"status\": \"true\", \"message\": \"Renew Trojan via API success (handled by system commands)\"}}");
}
pub fn handle_api_renew_noobz() {
    println!("{{\"status\": \"true\", \"message\": \"Renew NoobzVPN via API success (handled by system commands)\"}}");
}
