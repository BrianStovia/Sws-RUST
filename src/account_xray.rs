use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, get_ip, send_telegram_notification, run_bash_cmd, generate_uuid, base64_encode};

fn get_config_users(prefix: &str) -> Vec<(String, String)> {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return Vec::new();
    }
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut users = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(prefix) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                users.push((parts[1].to_string(), parts[2].to_string()));
            }
        }
    }
    users.sort();
    users.dedup_by(|a, b| a.0 == b.0);
    users
}

fn user_exists_in_config(username: &str) -> bool {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return false;
    }
    let content = fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("### ") || trimmed.starts_with("#& ") || trimmed.starts_with("#! ") || trimmed.starts_with("#&r "))
            && trimmed.contains(username) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == username {
                return true;
            }
        }
    }
    false
}

fn add_xray_client(protocol: &str, username: &str, exp: &str, uuid: &str) -> io::Result<()> {
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
            // Insert user client block after anchor
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
                "reality" => {
                    new_lines.push(format!("#&r {} {}", username, exp));
                    new_lines.push(format!("}},{{\"id\": \"{}\",\"flow\": \"xtls-rprx-vision\",\"email\": \"{}\"", uuid, username));
                }
                _ => {}
            }
        }
    }
    
    fs::write(path, new_lines.join("\n") + "\n")?;
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }
    Ok(())
}

fn delete_xray_client(username: &str) -> io::Result<()> {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let mut new_lines = Vec::new();
    let mut lines_iter = content.lines().peekable();
    
    while let Some(line) = lines_iter.next() {
        let trimmed = line.trim();
        if (trimmed.starts_with("### ") || trimmed.starts_with("#& ") || trimmed.starts_with("#! ") || trimmed.starts_with("#&r ")) 
            && trimmed.contains(username) {
            // Check if username matches exactly
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == username {
                // Skip this comment line
                // Also peek and skip client line starting with "},{"
                if let Some(next_line) = lines_iter.peek() {
                    if next_line.trim().starts_with("},{") {
                        let _ = lines_iter.next();
                    }
                }
                continue;
            }
        }
        new_lines.push(line.to_string());
    }
    fs::write(path, new_lines.join("\n") + "\n")?;
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }
    Ok(())
}

fn renew_xray_client(prefix: &str, username: &str, days: i64) -> io::Result<String> {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "config.json not found"));
    }
    let content = fs::read_to_string(path)?;
    let mut current_exp = String::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(prefix) && trimmed.contains(username) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == username {
                current_exp = parts[2].to_string();
                break;
            }
        }
    }
    
    if current_exp.is_empty() {
        current_exp = run_bash_cmd("date +%Y-%m-%d");
    }
    
    // Calculate new expiration date
    let today_secs = run_bash_cmd(&format!("date -d '{}' +%s", current_exp)).parse::<i64>().unwrap_or(1770000000);
    let new_secs = today_secs + (days * 86400);
    let new_exp = run_bash_cmd(&format!("date -u --date='1970-01-01 {} sec GMT' +%Y-%m-%d", new_secs));
    let new_exp = if new_exp.is_empty() { "2026-12-31".to_string() } else { new_exp };
    
    let mut new_lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(prefix) && trimmed.contains(username) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == username {
                new_lines.push(format!("{} {} {}", prefix, username, new_exp));
                continue;
            }
        }
        new_lines.push(line.to_string());
    }
    fs::write(path, new_lines.join("\n") + "\n")?;
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }
    Ok(new_exp)
}

pub fn add_xray(protocol: &str) {
    print!("\x1B[2J\x1B[1;1H");
    
    // Reality special check
    if protocol == "reality" && !Path::new("/usr/local/etc/v2ray/reality.conf").exists() {
        println!("Reality configuration not found on this VPS!");
        return;
    }

    let title = match protocol {
        "vmess" => "Add Xray/Vmess Account",
        "vless" => "Add Xray/Vless Account",
        "trojan" => "Add Xray/Trojan Account",
        "reality" => "Add Xray/Reality Account",
        _ => "Add Xray Account",
    };

    println!("───────────────────────────");
    println!("   {}   ", title);
    println!("───────────────────────────");

    let mut user = String::new();
    print!("User: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    if user.is_empty() {
        return;
    }

    if user_exists_in_config(&user) {
        println!("\nUsername already exists, please choose another name.");
        println!("───────────────────────────");
        println!("\nPress Enter to return to menu...");
        let mut temp = String::new();
        let _ = io::stdin().read_line(&mut temp);
        return;
    }

    let mut days_str = String::new();
    print!("Expired (days): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut days_str);
    let days: i64 = days_str.trim().parse().unwrap_or(30);

    let mut uuid_in = String::new();
    print!("Input UUID/Password (Empty Default): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut uuid_in);
    let mut uuid = uuid_in.trim().to_string();

    if uuid.is_empty() || uuid.contains(' ') || uuid.len() < 5 {
        uuid = generate_uuid();
        println!("Generated new UUID/Password: {}", uuid);
    } else {
        println!("Using provided UUID/Password: {}", uuid);
    }

    let exp = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", days));
    let exp = if exp.is_empty() { "2026-12-31".to_string() } else { exp };

    if let Err(e) = add_xray_client(protocol, &user, &exp, &uuid) {
        println!("Error creating user: {}", e);
        return;
    }

    let domain = get_domain();
    let ip = get_ip();

    print!("\x1B[2J\x1B[1;1H");

    let mut details = String::new();
    let mut links = String::new();
    let mut html_telegram = String::new();

    match protocol {
        "vless" => {
            let link1 = format!("vless://{}@{}:443?path=/vless&security=tls&encryption=none&type=ws#{}", uuid, domain, user);
            let link2 = format!("vless://{}@{}:80?path=/vless&encryption=none&type=ws#{}", uuid, domain, user);
            let link3 = format!("vless://{}@{}:443?mode=gun&security=tls&encryption=none&type=grpc&serviceName=vless-grpc&sni={}#{}", uuid, domain, domain, user);

            details = format!(
                "Remarks     : {}\n\
                 Domain      : {}\n\
                 Port Tls    : 443\n\
                 Port Ntls   : 80, 2082\n\
                 UUID/ID     : {}\n\
                 Path        : /vless\n\
                 ServiceName : vless-grpc\n\
                 Expired On  : {}",
                user, domain, uuid, exp
            );

            links = format!(
                "Link TLS    : {}\n\
                 ───────────────────\n\
                 Link NTLS   : {}\n\
                 ───────────────────\n\
                 Link GRPC   : {}",
                link1, link2, link3
            );

            html_telegram = format!(
                "<b>───────────────────</b>\n\
                 VLESS ACCOUNT\n\
                 <b>───────────────────</b>\n\
                 <b>Remarks     : </b><code>{}</code>\n\
                 <b>Domain      : </b><code>{}</code>\n\
                 <b>Port Tls    : </b><code>443</code>\n\
                 <b>Port Ntls   : </b><code>80, 2082</code>\n\
                 <b>UUID/ID     : </b><code>{}</code>\n\
                 <b>Path        : </b><code>/vless</code>\n\
                 <b>ServiceName : </b><code>vless-grpc</code>\n\
                 <b>───────────────────</b>\n\
                 <b>Link TLS    :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Link NTLS   :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Link GRPC   :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Expired On  : </b><code>{}</code>\n\
                 <b>───────────────────</b>",
                user, domain, uuid, link1, link2, link3, exp
            );
        }
        "vmess" => {
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

            let link1 = format!("vmess://{}", base64_encode(acs.as_bytes()));
            let link2 = format!("vmess://{}", base64_encode(ask.as_bytes()));
            let link3 = format!("vmess://{}", base64_encode(grpc.as_bytes()));

            details = format!(
                "Remarks     : {}\n\
                 Domain      : {}\n\
                 Port Tls    : 443\n\
                 Port Ntls   : 80, 2082\n\
                 ID          : {}\n\
                 Encryption  : none\n\
                 Path        : /whatever, /multipath\n\
                 ServiceName : vmess-grpc\n\
                 Expired On  : {}",
                user, domain, uuid, exp
            );

            links = format!(
                "Link TLS    : {}\n\
                 ───────────────────\n\
                 Link NTLS   : {}\n\
                 ───────────────────\n\
                 Link GRPC   : {}",
                link1, link2, link3
            );

            html_telegram = format!(
                "<b>───────────────────</b>\n\
                 VMESS ACCOUNT\n\
                 <b>───────────────────</b>\n\
                 <b>Remarks     : </b><code>{}</code>\n\
                 <b>Domain      : </b><code>{}</code>\n\
                 <b>Port Tls    : </b><code>443</code>\n\
                 <b>Port Ntls   : </b><code>80, 2082</code>\n\
                 <b>ID          : </b><code>{}</code>\n\
                 <b>Encryption  : </b><code>none</code>\n\
                 <b>Path        : </b><code>/whatever, /multipath</code>\n\
                 <b>ServiceName : </b><code>vmess-grpc</code>\n\
                 <b>───────────────────</b>\n\
                 <b>Link TLS    :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Link NTLS   :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Link GRPC   :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Expired On  : </b><code>{}</code>\n\
                 <b>───────────────────</b>",
                user, domain, uuid, link1, link2, link3, exp
            );
        }
        "trojan" => {
            let link1 = format!("trojan://{}@{}:443?path=%2Ftrojan-ws&security=tls&type=ws#{}", uuid, domain, user);
            let link2 = format!("trojan://{}@{}:443?mode=gun&security=tls&type=grpc&serviceName=trojan-grpc&sni={}#{}", uuid, domain, domain, user);

            details = format!(
                "Remarks     : {}\n\
                 Domain      : {}\n\
                 Port Tls    : 443\n\
                 Password    : {}\n\
                 Path        : /trojan-ws\n\
                 ServiceName : trojan-grpc\n\
                 Expired On  : {}",
                user, domain, uuid, exp
            );

            links = format!(
                "Link TLS    : {}\n\
                 ───────────────────\n\
                 Link GRPC   : {}",
                link1, link2
            );

            html_telegram = format!(
                "<b>───────────────────</b>\n\
                 TROJAN ACCOUNT\n\
                 <b>───────────────────</b>\n\
                 <b>Remarks     : </b><code>{}</code>\n\
                 <b>Domain      : </b><code>{}</code>\n\
                 <b>Port Tls    : </b><code>443</code>\n\
                 <b>Password    : </b><code>{}</code>\n\
                 <b>Path        : </b><code>/trojan-ws</code>\n\
                 <b>ServiceName : </b><code>trojan-grpc</code>\n\
                 <b>───────────────────</b>\n\
                 <b>Link WS     :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Link GRPC   :</b>\n\
                 <pre>{}</pre>\n\
                 <b>───────────────────</b>\n\
                 <b>Expired On  : </b><code>{}</code>\n\
                 <b>───────────────────</b>",
                user, domain, uuid, link1, link2, exp
            );
        }
        "reality" => {
            // Load config
            let conf_content = fs::read_to_string("/usr/local/etc/v2ray/reality.conf").unwrap_or_default();
            let mut port = "8443".to_string();
            let mut pub_key = String::new();
            let mut short_id = String::new();
            let mut sni_list = "yahoo.com".to_string();
            
            for line in conf_content.lines() {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    match parts[0] {
                        "REALITY_PORT" => port = parts[1].to_string(),
                        "REALITY_PUB" => pub_key = parts[1].to_string(),
                        "REALITY_SID" => short_id = parts[1].to_string(),
                        "REALITY_SNI" => sni_list = parts[1].to_string(),
                        _ => {}
                    }
                }
            }
            let first_sni = sni_list.split(',').next().unwrap_or("yahoo.com");
            let target_host = if domain.is_empty() || domain == "not.configured.com" { &ip } else { &domain };

            let link = format!(
                "vless://{}@{}:{}?security=reality&sni={}&fp=chrome&pbk={}&sid={}&flow=xtls-rprx-vision&type=tcp#{}",
                uuid, target_host, port, first_sni, pub_key, short_id, user
            );

            details = format!(
                "Remarks      : {}\n\
                 Domain/IP    : {}\n\
                 Port         : {}\n\
                 UUID         : {}\n\
                 Encryption   : none\n\
                 Network      : tcp\n\
                 Security     : reality\n\
                 Flow         : xtls-rprx-vision\n\
                 Public Key   : {}\n\
                 Short ID     : {}\n\
                 SNI / Dest   : {}\n\
                 Expired On   : {}",
                user, target_host, port, uuid, pub_key, short_id, first_sni, exp
            );

            links = format!("Link Reality : {}", link);

            html_telegram = format!(
                "<b>───────────────────</b>\n\
                 REALITY ACCOUNT\n\
                 <b>───────────────────</b>\n\
                 <b>Remarks      : </b><code>{}</code>\n\
                 <b>Domain/IP    : </b><code>{}</code>\n\
                 <b>Port         : </b><code>{}</code>\n\
                 <b>UUID         : </b><code>{}</code>\n\
                 <b>Network      : </b><code>tcp</code>\n\
                 <b>Security     : </b><code>reality</code>\n\
                 <b>Flow         : </b><code>xtls-rprx-vision</code>\n\
                 <b>Public Key   : </b><code>{}</code>\n\
                 <b>Short ID     : </b><code>{}</code>\n\
                 <b>SNI / Dest   : </b><code>{}</code>\n\
                 <b>Expired On   : </b><code>{}</code>\n\
                 <b>───────────────────</b>\n\
                 <b>Link Reality :</b>\n\
                 <code>{}</code>\n\
                 <b>───────────────────</b>",
                user, target_host, port, uuid, pub_key, short_id, first_sni, exp, link
            );
        }
        _ => {}
    }

    send_telegram_notification(&html_telegram);

    println!("────────────────────────────────");
    println!("   {}   ", title.to_uppercase());
    println!("────────────────────────────────");
    println!("{}", details);
    println!("────────────────────────────────");
    println!("{}", links);
    println!("────────────────────────────────");

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn del_xray(protocol: &str) {
    print!("\x1B[2J\x1B[1;1H");
    
    let tag = match protocol {
        "vmess" => "### ",
        "vless" => "#& ",
        "trojan" => "#! ",
        "reality" => "#&r ",
        _ => "### ",
    };

    let title = match protocol {
        "vmess" => "VMESS ACCOUNT",
        "vless" => "VLESS ACCOUNT",
        "trojan" => "TROJAN ACCOUNT",
        "reality" => "REALITY ACCOUNT",
        _ => "ACCOUNT",
    };

    println!("────────────────────────────────");
    println!("   {}   ", title);
    println!("────────────────────────────────");
    println!("USERNAME          EXP DATE");
    println!("────────────────────────────────");

    let users = get_config_users(tag);
    for u in &users {
        println!("{:<17} {}", u.0, u.1);
    }
    
    println!("────────────────────────────────");
    println!("Account number: {} user", users.len());
    println!("────────────────────────────────");
    println!();

    let mut username = String::new();
    print!("Username to Delete : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut username);
    let username = username.trim().to_string();

    if username.is_empty() {
        return;
    }

    let exists = users.iter().any(|u| u.0 == username);
    if exists {
        if let Err(e) = delete_xray_client(&username) {
            println!("Error deleting user: {}", e);
        } else {
            println!("User {} was removed.", username);
        }
    } else {
        println!("Failure: User {} Not Exist.", username);
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn renew_xray(protocol: &str) {
    print!("\x1B[2J\x1B[1;1H");

    let tag = match protocol {
        "vmess" => "### ",
        "vless" => "#& ",
        "trojan" => "#! ",
        "reality" => "#&r ",
        _ => "### ",
    };

    let title = match protocol {
        "vmess" => "RENEW VMESS ACCOUNT",
        "vless" => "RENEW VLESS ACCOUNT",
        "trojan" => "RENEW TROJAN ACCOUNT",
        "reality" => "RENEW REALITY ACCOUNT",
        _ => "RENEW ACCOUNT",
    };

    println!("───────────────────────────");
    println!("    {}    ", title);
    println!("───────────────────────────");
    println!();

    let users = get_config_users(tag);
    if users.is_empty() {
        println!("You have no existing clients!");
        println!("───────────────────────────");
        println!("\nPress Enter to return to menu...");
        let mut temp = String::new();
        let _ = io::stdin().read_line(&mut temp);
        return;
    }

    for u in &users {
        println!("{:<17} {}", u.0, u.1);
    }
    println!("───────────────────────────");
    println!();

    let mut user = String::new();
    print!("Input Username : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    if user.is_empty() {
        return;
    }

    let exists = users.iter().any(|u| u.0 == user);
    if exists {
        let mut days_str = String::new();
        print!("Expired (days): ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut days_str);
        let days: i64 = days_str.trim().parse().unwrap_or(30);

        match renew_xray_client(tag.trim(), &user, days) {
            Ok(new_exp) => {
                print!("\x1B[2J\x1B[1;1H");
                println!("───────────────────────────");
                println!("RENEW ACCOUNT SUCCESSFULLY ");
                println!("───────────────────────────");
                println!();
                println!(" Client Name : {}", user);
                println!(" Expired On  : {}", new_exp);
                println!();
                println!("───────────────────────────");
            }
            Err(e) => {
                println!("Error renewing client: {}", e);
            }
        }
    } else {
        println!("Failure: User {} Not Exist.", user);
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cek_xray(protocol: &str) {
    print!("\x1B[2J\x1B[1;1H");

    let tag = match protocol {
        "vmess" => "### ",
        "vless" => "#& ",
        "trojan" => "#! ",
        "reality" => "#&r ",
        _ => "### ",
    };

    let title = match protocol {
        "vmess" => "VMESS LOGIN ACCOUNT",
        "vless" => "VLESS LOGIN ACCOUNT",
        "trojan" => "TROJAN LOGIN ACCOUNT",
        "reality" => "REALITY LOGIN ACCOUNT",
        _ => "LOGIN ACCOUNT",
    };

    println!("───────────────────────────");
    println!("    {}    ", title);
    println!("───────────────────────────");

    let users = get_config_users(tag);
    
    // Read access.log (normally /var/log/v2ray/access.log)
    let log_path = "/var/log/v2ray/access.log";
    let log_content = if Path::new(log_path).exists() {
        // Run tail -n 500 under the hood
        run_bash_cmd(&format!("tail -n 500 {}", log_path))
    } else {
        String::new()
    };

    for (user, _) in users {
        let mut active_ips = Vec::new();
        for line in log_content.lines() {
            if line.contains(&user) {
                // Parse line. Split by spaces
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    // Third part is tcp:1.2.3.4:5678 or similar
                    let addr = parts[2].strip_prefix("tcp:").unwrap_or(parts[2]);
                    let ip = if let Some(colon_idx) = addr.find(':') {
                        &addr[..colon_idx]
                    } else {
                        addr
                    };
                    if !ip.is_empty() && !active_ips.contains(&ip.to_string()) {
                        active_ips.push(ip.to_string());
                    }
                }
            }
        }

        if !active_ips.is_empty() {
            println!("user : {}", user);
            for (idx, ip) in active_ips.iter().enumerate() {
                // Get ISP org
                let res = run_bash_cmd(&format!("curl -s --max-time 3 http://ip-api.com/json/{}", ip));
                let mut org = String::new();
                if let Some(org_idx) = res.find("\"org\":\"") {
                    let after = &res[org_idx + 7..];
                    if let Some(end_idx) = after.find('"') {
                        org = after[..end_idx].to_string();
                    }
                }
                if org.is_empty() {
                    if let Some(isp_idx) = res.find("\"isp\":\"") {
                        let after = &res[isp_idx + 7..];
                        if let Some(end_idx) = after.find('"') {
                            org = after[..end_idx].to_string();
                        }
                    }
                }
                if org.is_empty() {
                    org = "Unknown Org/ISP".to_string();
                }
                println!("  {}) {} / {}", idx + 1, ip, org);
            }
            println!("───────────────────────────");
        }
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn trafik_xray(protocol: &str) {
    print!("\x1B[2J\x1B[1;1H");

    let tag = match protocol {
        "vmess" => "### ",
        "vless" => "#& ",
        "trojan" => "#! ",
        "reality" => "#&r ",
        _ => "### ",
    };

    let title = match protocol {
        "vmess" => "Trafik Vmess",
        "vless" => "Trafik Vless",
        "trojan" => "Trafik Trojan",
        "reality" => "Trafik Reality",
        _ => "Trafik",
    };

    // Run xray stats API
    let stats = run_bash_cmd("v2ray api stats -server 127.0.0.1:10085 2>/dev/null");
    let total_all = if stats.contains("Total:") {
        stats.lines()
            .find(|line| line.contains("Total:"))
            .map(|line| line.split_whitespace().nth(1).unwrap_or("0"))
            .unwrap_or("0")
            .to_string()
    } else {
        "0 MB".to_string()
    };

    println!("======");
    println!("{}", title);
    println!("======");

    // Extract server level stats
    let get_api_stat = |direction: &str| -> String {
        stats.lines()
            .find(|line| line.contains(&format!("inbound>>>api>>>traffic>>>{}", direction)))
            .map(|line| line.split_whitespace().nth(1).unwrap_or("0"))
            .unwrap_or("0")
            .to_string()
    };

    let down_api = get_api_stat("downlink");
    let up_api = get_api_stat("uplink");

    println!("[ server ]");
    println!("inbound : {}", down_api);
    println!("outbound: {}", up_api);
    println!("server total: {}", total_all);
    println!();

    println!("======");
    println!("[ user ]");

    let users = get_config_users(tag);
    for (user, _) in users {
        let down = stats.lines()
            .find(|line| line.contains(&format!("user>>>{}>>>traffic>>>downlink", user)))
            .map(|line| line.split_whitespace().nth(1).unwrap_or("0"))
            .unwrap_or("0");
        let up = stats.lines()
            .find(|line| line.contains(&format!("user>>>{}>>>traffic>>>uplink", user)))
            .map(|line| line.split_whitespace().nth(1).unwrap_or("0"))
            .unwrap_or("0");

        if down != "0" || up != "0" {
            println!("{}: {} / {} / (refer to total)", user, down, up);
        }
    }
    println!("======");

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
