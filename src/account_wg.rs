use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, get_ip, send_telegram_notification, run_bash_cmd};

const WG_CONF: &str = "/etc/wireguard/wg0.conf";

fn get_wg_peers() -> Vec<(String, String)> {
    if !Path::new(WG_CONF).exists() {
        return Vec::new();
    }
    let content = fs::read_to_string(WG_CONF).unwrap_or_default();
    let mut peers = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#& ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                peers.push((parts[1].to_string(), parts[2].to_string()));
            }
        }
    }
    peers
}

pub fn add_wg() {
    print!("\x1B[2J\x1B[1;1H");
    if !Path::new("/etc/wireguard").exists() {
        println!("WireGuard is not installed on this VPS!");
        return;
    }

    println!("───────────────────────────");
    println!("  Add WireGuard Account   ");
    println!("───────────────────────────");

    let mut user = String::new();
    print!("User: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    if user.is_empty() {
        return;
    }

    let peers = get_wg_peers();
    if peers.iter().any(|p| p.0 == user) {
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

    // Generate keys
    let client_priv = run_bash_cmd("wg genkey");
    let client_pub = run_bash_cmd(&format!("echo '{}' | wg pubkey", client_priv.trim()));
    let _ = &client_pub;
    let client_psk = run_bash_cmd("wg genpsk");

    let server_pub = fs::read_to_string("/etc/wireguard/public.key").unwrap_or_default().trim().to_string();

    // Find next IP in 10.22.0.2..254
    let mut client_ip = String::new();
    let wg_conf_content = fs::read_to_string(WG_CONF).unwrap_or_default();
    for i in 2..255 {
        let test_ip = format!("10.22.0.{}", i);
        if !wg_conf_content.contains(&format!("{}/32", test_ip)) {
            client_ip = test_ip;
            break;
        }
    }

    if client_ip.is_empty() {
        println!("Error: No available IP addresses in 10.22.0.0/24 pool!");
        return;
    }

    let exp = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", days));
    let exp = if exp.is_empty() { "2026-12-31".to_string() } else { exp };

    // Append peer config
    #[cfg(target_os = "linux")]
    {
        if let Ok(mut file) = fs::OpenOptions::new().append(true).open(WG_CONF) {
            let _ = writeln!(file, "\n#& {} {}", user, exp);
            let _ = writeln!(file, "[Peer]");
            let _ = writeln!(file, "PublicKey = {}", client_pub.trim());
            let _ = writeln!(file, "PresharedKey = {}", client_psk.trim());
            let _ = writeln!(file, "AllowedIPs = {}/32", client_ip);
        }
        
        // Sync Wireguard
        let _ = run_bash_cmd("wg syncconf wg0 <(wg-quick strip wg0)");
    }

    // Create client config file
    let domain = get_domain();
    let ip = get_ip();
    let endpoint = if domain.is_empty() || domain == "not.configured.com" { &ip } else { &domain };

    let client_conf = format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}/24\n\
         DNS = 1.1.1.1, 8.8.8.8\n\
         MTU = 1280\n\n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         Endpoint = {}:51820\n\
         AllowedIPs = 0.0.0.0/0\n\
         PersistentKeepalive = 20\n",
        client_priv.trim(), client_ip, server_pub, client_psk.trim(), endpoint
    );

    let _ = fs::create_dir_all("/var/www/html");
    let conf_path = format!("/var/www/html/wg-{}.conf", user);
    let _ = fs::write(&conf_path, &client_conf);

    print!("\x1B[2J\x1B[1;1H");
    println!("─────────────────────────────────────────────────────");
    println!("                WIREGUARD QR CODE");
    println!("─────────────────────────────────────────────────────");
    
    #[cfg(target_os = "linux")]
    {
        let qrcode_status = Command::new("qrencode")
            .args(&["-t", "ansiutf8"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                child.stdin.as_mut().unwrap().write_all(client_conf.as_bytes())?;
                child.wait()
            });
            
        if qrcode_status.is_err() {
            println!("qrencode not installed or failed, cannot display QR code.");
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("(QR code display placeholder for non-Linux)");
    }

    println!("─────────────────────────────────────────────────────");
    println!("             WIREGUARD CONFIGURATION");
    println!("─────────────────────────────────────────────────────");
    println!("{}", client_conf);
    println!("─────────────────────────────────────────────────────");
    println!();
    println!("─────────────────────────────────────────────────────");
    println!("              WIREGUARD ACCOUNT INFO");
    println!("─────────────────────────────────────────────────────");
    println!(" Remarks     : {}", user);
    println!(" IP Address  : {}", client_ip);
    println!(" Port        : 51820");
    println!(" Expired On  : {}", exp);
    println!(" Config Link : http://{}/wg-{}.conf", endpoint, user);
    println!("─────────────────────────────────────────────────────");

    let text_telegram = format!(
        "<b>───────────────────</b>\n\
         WIREGUARD ACCOUNT\n\
         <b>───────────────────</b>\n\
         <b>Remarks     : </b><code>{}</code>\n\
         <b>IP Address  : </b><code>{}</code>\n\
         <b>Port        : </b><code>51820</code>\n\
         <b>Expired On  : </b><code>{}</code>\n\
         <b>Config Link : </b>http://{}/wg-{}.conf\n\
         <b>───────────────────</b>",
        user, client_ip, exp, endpoint, user
    );
    send_telegram_notification(&text_telegram);

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn del_wg() {
    print!("\x1B[2J\x1B[1;1H");
    if !Path::new(WG_CONF).exists() {
        println!("WireGuard configuration not found!");
        return;
    }

    let peers = get_wg_peers();
    if peers.is_empty() {
        println!("───────────────────────────");
        println!(" No WireGuard Accounts Found");
        println!("───────────────────────────");
        println!("\nPress Enter to return to menu...");
        let mut temp = String::new();
        let _ = io::stdin().read_line(&mut temp);
        return;
    }

    println!("───────────────────────────");
    println!("   Delete WireGuard Account");
    println!("───────────────────────────");
    for (idx, p) in peers.iter().enumerate() {
        println!("{}. {}", idx + 1, p.0);
    }
    println!("───────────────────────────");

    let mut choice_str = String::new();
    print!("Select Account (1-{}): ", peers.len());
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut choice_str);
    let choice: usize = choice_str.trim().parse().unwrap_or(0);

    if choice < 1 || choice > peers.len() {
        println!("Invalid Selection!");
        return;
    }

    let selected_user = &peers[choice - 1].0;

    // Delete peer block from wg0.conf
    if let Ok(content) = fs::read_to_string(WG_CONF) {
        let mut new_lines = Vec::new();
        let mut lines_iter = content.lines().peekable();
        
        while let Some(line) = lines_iter.next() {
            let trimmed = line.trim();
            if trimmed.starts_with("#& ") && trimmed.contains(selected_user) {
                // Skip this comment line and next peer details
                while let Some(peek_line) = lines_iter.peek() {
                    let peek_trimmed = peek_line.trim();
                    if peek_trimmed.starts_with("AllowedIPs =") {
                        let _ = lines_iter.next(); // Consume the end of range
                        break;
                    }
                    let _ = lines_iter.next(); // Consume other peer configuration details
                }
                continue;
            }
            new_lines.push(line.to_string());
        }
        let _ = fs::write(WG_CONF, new_lines.join("\n") + "\n");
    }

    #[cfg(target_os = "linux")]
    {
        let _ = run_bash_cmd("wg syncconf wg0 <(wg-quick strip wg0)");
    }
    let _ = fs::remove_file(format!("/var/www/html/wg-{}.conf", selected_user));

    println!("\nWireGuard account '{}' has been deleted successfully.", selected_user);
    println!("───────────────────────────");
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn renew_wg() {
    print!("\x1B[2J\x1B[1;1H");
    if !Path::new(WG_CONF).exists() {
        println!("WireGuard configuration not found!");
        return;
    }

    let peers = get_wg_peers();
    if peers.is_empty() {
        println!("───────────────────────────");
        println!(" No WireGuard Accounts Found");
        println!("───────────────────────────");
        println!("\nPress Enter to return to menu...");
        let mut temp = String::new();
        let _ = io::stdin().read_line(&mut temp);
        return;
    }

    println!("───────────────────────────");
    println!("   Renew WireGuard Account");
    println!("───────────────────────────");
    for (idx, p) in peers.iter().enumerate() {
        println!("{}. {} [Exp: {}]", idx + 1, p.0, p.1);
    }
    println!("───────────────────────────");

    let mut choice_str = String::new();
    print!("Select Account (1-{}): ", peers.len());
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut choice_str);
    let choice: usize = choice_str.trim().parse().unwrap_or(0);

    if choice < 1 || choice > peers.len() {
        println!("Invalid Selection!");
        return;
    }

    let selected_user = &peers[choice - 1].0;
    let existing_exp = &peers[choice - 1].1;

    let mut days_str = String::new();
    print!("Add Days to Expire: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut days_str);
    let days: i64 = days_str.trim().parse().unwrap_or(30);

    let today = run_bash_cmd("date +%Y-%m-%d");
    let new_exp = if existing_exp < &today {
        run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", days))
    } else {
        run_bash_cmd(&format!("date -d '{} + {} days' +%Y-%m-%d", existing_exp, days))
    };
    let new_exp = if new_exp.is_empty() { "2026-12-31".to_string() } else { new_exp };

    if let Ok(content) = fs::read_to_string(WG_CONF) {
        let old_line = format!("#& {} {}", selected_user, existing_exp);
        let new_line = format!("#& {} {}", selected_user, new_exp);
        let updated = content.replace(&old_line, &new_line);
        let _ = fs::write(WG_CONF, updated);
    }

    println!("\nWireGuard account '{}' has been extended to {}.", selected_user, new_exp);
    println!("───────────────────────────");
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cek_wg() {
    print!("\x1B[2J\x1B[1;1H");
    println!("───────────────────────────");
    println!("   WireGuard Active Peers  ");
    println!("───────────────────────────");
    if Path::new("/sys/class/net/wg0").exists() {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("wg").args(&["show", "wg0"]).status();
        }
    } else {
        println!("WireGuard interface wg0 is not active.");
    }
    println!("───────────────────────────");
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
