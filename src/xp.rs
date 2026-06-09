use std::fs;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Command;
use crate::utils::{run_bash_cmd, send_telegram_notification};

pub fn run_xp() {
    println!("Running Expired Accounts Cleaner (xp)...");
    
    let today = run_bash_cmd("date +%Y-%m-%d");
    let today = if today.is_empty() { "2026-06-09".to_string() } else { today };

    // 1. Clean Xray (Vmess, Vless, Trojan, Reality)
    let xray_config = "/usr/local/etc/v2ray/config.json";
    if Path::new(xray_config).exists() {
        if let Ok(content) = fs::read_to_string(xray_config) {
            let mut new_lines = Vec::new();
            let mut lines_iter = content.lines().peekable();
            let mut restarted = false;

            while let Some(line) = lines_iter.next() {
                let trimmed = line.trim();
                let mut is_expired = false;
                let mut username = String::new();
                let mut exp_date = String::new();
                let mut protocol = "";

                if trimmed.starts_with("### ") {
                    protocol = "VMess";
                } else if trimmed.starts_with("#& ") {
                    protocol = "VLess";
                } else if trimmed.starts_with("#! ") {
                    protocol = "Trojan";
                } else if trimmed.starts_with("#&r ") {
                    protocol = "Reality";
                }

                if !protocol.is_empty() {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        username = parts[1].to_string();
                        exp_date = parts[2].to_string();
                        if exp_date < today {
                            is_expired = true;
                        }
                    }
                }

                if is_expired {
                    // Skip the comment line and the next client block
                    if let Some(next_line) = lines_iter.peek() {
                        if next_line.trim().starts_with("},{") {
                            let _ = lines_iter.next(); // Consume client block
                        }
                    }
                    println!("Expired {} account deleted: {} [Exp: {}]", protocol, username, exp_date);
                    
                    let notification = format!(
                        "🚨 Akun {} Kedaluwarsa Dihapus 🚨\nUsername: {}\nExpired : {}",
                        protocol, username, exp_date
                    );
                    send_telegram_notification(&notification);
                    restarted = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }

            if restarted {
                let _ = fs::write(xray_config, new_lines.join("\n") + "\n");
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
                }
            }
        }
    }

    // 2. Clean SSH
    let shadow_path = "/etc/shadow";
    if Path::new(shadow_path).exists() {
        if let Ok(shadow_content) = fs::read_to_string(shadow_path) {
            let today_days = run_bash_cmd("date +%s").parse::<i64>().unwrap_or(0) / 86400;
            
            if today_days > 0 {
                for line in shadow_content.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 8 {
                        let username = parts[0];
                        if let Ok(exp_days) = parts[7].parse::<i64>() {
                            if exp_days > 0 && exp_days < today_days {
                                // Expired system user
                                println!("Expired SSH account deleted: {}", username);
                                
                                let _ = fs::remove_dir_all(format!("/etc/funny/limit/ssh/ip/{}", username));
                                
                                #[cfg(target_os = "linux")]
                                {
                                    let _ = Command::new("userdel").args(&["--force", username]).status();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Clean NoobzVPN
    let noobz_db = "/etc/noobzvpns/.noobz.db";
    if Path::new(noobz_db).exists() {
        if let Ok(db_content) = fs::read_to_string(noobz_db) {
            let mut new_lines = Vec::new();
            let mut updated = false;

            for line in db_content.lines() {
                if line.starts_with("#noobzvpns# ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let name = parts[1];
                        let expi = parts[3];
                        if expi < today.as_str() {
                            println!("Expired NoobzVPN account deleted: {}", name);
                            #[cfg(target_os = "linux")]
                            {
                                let _ = Command::new("noobzvpns").args(&["remove", name]).status();
                            }
                            
                            let notification = format!(
                                "🚨 Akun NoobzVPN Kedaluwarsa Dihapus 🚨\nUsername: {}\nExpired : {}",
                                name, expi
                            );
                            send_telegram_notification(&notification);
                            updated = true;
                            continue;
                        }
                    }
                }
                new_lines.push(line.to_string());
            }

            if updated {
                let _ = fs::write(noobz_db, new_lines.join("\n") + "\n");
            }
        }
    }

    // 4. Clean Wireguard
    let wg_conf = "/etc/wireguard/wg0.conf";
    if Path::new(wg_conf).exists() {
        if let Ok(content) = fs::read_to_string(wg_conf) {
            let mut new_lines = Vec::new();
            let mut lines_iter = content.lines().peekable();
            let mut updated = false;

            while let Some(line) = lines_iter.next() {
                let trimmed = line.trim();
                let mut is_expired = false;
                let mut username = String::new();
                let mut exp_date = String::new();

                if trimmed.starts_with("#& ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        username = parts[1].to_string();
                        exp_date = parts[2].to_string();
                        if exp_date < today {
                            is_expired = true;
                        }
                    }
                }

                if is_expired {
                    // Skip this comment line and next peer configuration details
                    while let Some(peek_line) = lines_iter.peek() {
                        let peek_trimmed = peek_line.trim();
                        if peek_trimmed.starts_with("AllowedIPs =") {
                            let _ = lines_iter.next(); // Consume AllowedIPs
                            break;
                        }
                        let _ = lines_iter.next(); // Consume other config lines
                    }
                    
                    println!("Expired Wireguard peer deleted: {} [Exp: {}]", username, exp_date);
                    let _ = fs::remove_file(format!("/var/www/html/wg-{}.conf", username));
                    
                    let notification = format!(
                        "🚨 Akun WireGuard Kedaluwarsa Dihapus 🚨\nUsername: {}\nExpired : {}",
                        username, exp_date
                    );
                    send_telegram_notification(&notification);
                    updated = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }

            if updated {
                let _ = fs::write(wg_conf, new_lines.join("\n") + "\n");
                #[cfg(target_os = "linux")]
                {
                    let _ = run_bash_cmd("wg syncconf wg0 <(wg-quick strip wg0)");
                }
            }
        }
    }
    
    println!("Expired accounts cleanup completed.");
}
