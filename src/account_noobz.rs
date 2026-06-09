use std::io::{self, Write};
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, send_telegram_notification, run_bash_cmd};

const DB_FILE: &str = "/etc/noobzvpns/db_user.json";
const DB_LOGIN_FILE: &str = "/etc/noobzvpns/.noobz.db";

fn run_jq(query: &str) -> String {
    if !Path::new(DB_FILE).exists() {
        return String::new();
    }
    let out = Command::new("jq")
        .args(&[query, DB_FILE])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

pub fn add_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("────────────────────────────");
    println!("Add Account NoobzVPN");
    println!("────────────────────────────");

    let mut user = String::new();
    print!("Username  : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    if user.is_empty() {
        return;
    }

    let mut device = String::new();
    print!("Limit Device: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut device);
    let device = device.trim().to_string();

    let mut bw = String::new();
    print!("Limit Bandwidth (GB): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut bw);
    let bw = bw.trim().to_string();

    let mut masaaktif = String::new();
    print!("Masa Aktif (Days): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut masaaktif);
    let masaaktif = masaaktif.trim().to_string();

    if device.is_empty() || bw.is_empty() || masaaktif.is_empty() {
        println!("Error: Fields cannot be empty.");
        return;
    }

    let pass = "fn-project.com";
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("noobzvpns")
            .args(&[
                "add", "--password", pass, &user,
                "--bandwidth", &bw, "--devices", &device,
                "--expired", &masaaktif
            ])
            .status();
    }

    let expi = run_bash_cmd(&format!("date -d '{} days' +%Y-%m-%d", masaaktif));
    let expi = if expi.is_empty() { "2026-12-31".to_string() } else { expi };

    // Update .noobz.db
    let db_content = fs::read_to_string(DB_LOGIN_FILE).unwrap_or_default();
    let mut new_lines = Vec::new();
    for line in db_content.lines() {
        if !line.contains(&user) {
            new_lines.push(line.to_string());
        }
    }
    new_lines.push(format!("#noobzvpns# {} {} {}", user, pass, expi));
    let _ = fs::write(DB_LOGIN_FILE, new_lines.join("\n") + "\n");

    let domain = get_domain();

    let text_telegram = format!(
        "───────────────────────────\n\
         <b>NoobzVPN Account</b>\n\
         <b>────────────────────────────</b>\n\
         <b>Hostname  :</b> <code>{}</code>\n\
         <b>Username  :</b> <code>{}@fn-project</code>\n\
         <b>Password  :</b> <code>fn-project.com</code>\n\
         <b>────────────────────────────</b>\n\
         <b>Limit Device:</b> <code>{}</code>\n\
         <b>Limit Bandwidth:</b> <code>{}</code> <b>GB</b>\n\
         <b>────────────────────────────</b>\n\
         <b>HTTP      :</b> <code>80, 2082</code>\n\
         <b>HTTP(S)   :</b> <code>443</code>\n\
         <b>────────────────────────────</b>\n\
         <b>PAYLOAD   :</b> <code>GET / HTTP/1.1[crlf]Host: [host][crlf]Upgrade: websocket[crlf][crlf]</code>\n\
         <b>────────────────────────────</b>\n\
         <b>Expired   :</b> <code>{}</code>\n\
         <b>────────────────────────────</b>",
        domain, user, device, bw, expi
    );

    send_telegram_notification(&text_telegram);

    print!("\x1B[2J\x1B[1;1H");
    println!("────────────────────────────");
    println!("NoobzVPN Account");
    println!("────────────────────────────");
    println!("Hostname  : {}", domain);
    println!("Username  : {}@fn-project", user);
    println!("Password  : {}", pass);
    println!("────────────────────────────");
    println!("Limit Device: {}", device);
    println!("Limit Bandwidth: {} GB", bw);
    println!("────────────────────────────");
    println!("HTTP      : 80, 2082");
    println!("HTTP(S)   : 443");
    println!("────────────────────────────");
    println!("PAYLOAD   : GET / HTTP/1.1[crlf]Host: [host][crlf]Upgrade: websocket[crlf][crlf]");
    println!("────────────────────────────");
    println!("Expired   : {}", expi);
    println!("────────────────────────────");

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn list_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("List Account Noobzvpns");
    println!("===========================");

    let users_list = run_jq(".users | keys[]");
    if users_list.is_empty() {
        println!("No users found or jq error.");
    } else {
        for user in users_list.lines() {
            let password = run_jq(&format!(".users[\"{}\"].password", user));
            let blocked = run_jq(&format!(".users[\"{}\"].blocked", user));
            let devices = run_jq(&format!(".users[\"{}\"].devices", user));
            let bandwidth = run_jq(&format!(".users[\"{}\"].bandwidth", user));
            let expired = run_jq(&format!(".users[\"{}\"].expired", user));
            let issued = run_jq(&format!(".users[\"{}\"].issued", user));

            // Convert issued/expired date
            let exp_sec = run_bash_cmd(&format!("date --date='{}' +%s", issued)).parse::<i64>().unwrap_or(0);
            let exp_val = expired.parse::<i64>().unwrap_or(0);
            let exp_date = if exp_sec > 0 {
                run_bash_cmd(&format!("date -d '@{}' '+%Y-%m-%d %H:%M:%S'", exp_sec + (exp_val * 86400)))
            } else {
                "Unknown".to_string()
            };

            let bw_val = bandwidth.parse::<f64>().unwrap_or(0.0);
            let bw_str = if bw_val >= 1024.0 {
                format!("{:.2} TB", bw_val / 1024.0)
            } else {
                format!("{} GB", bw_val)
            };

            println!("Username         : {}", user);
            println!("Password         : {}", password);
            println!("Blocked          : {}", blocked);
            println!("Limit Device     : {} Device", devices);
            println!("Limit Bandwidth  : {}", bw_str);
            println!("Expired          : {}", exp_date);
            println!("------------------------------");
        }
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn ex_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("List Account Noobzvpns");
    println!("===========================");

    let users_list = run_jq(".users | keys[]");
    for user in users_list.lines() {
        let expired = run_jq(&format!(".users[\"{}\"].expired", user));
        let issued = run_jq(&format!(".users[\"{}\"].issued", user));
        let exp_sec = run_bash_cmd(&format!("date --date='{}' +%s", issued)).parse::<i64>().unwrap_or(0);
        let exp_val = expired.parse::<i64>().unwrap_or(0);
        let exp_date = if exp_sec > 0 {
            run_bash_cmd(&format!("date -d '@{}' '+%Y-%m-%d %H:%M:%S'", exp_sec + (exp_val * 86400)))
        } else {
            "Unknown".to_string()
        };
        println!("Username         : {}", user);
        println!("Expired          : {}", exp_date);
        println!("------------------------------");
    }

    let mut username = String::new();
    print!("Input Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut username);
    let username = username.trim().to_string();

    if username.is_empty() {
        return;
    }

    let user_exists = run_jq(&format!(".users[\"{}\"]", username));
    if user_exists.is_empty() || user_exists == "null" {
        println!("Username not found.");
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("noobzvpns").args(&["renew", &username]).status();
    }

    // Refresh expiration date
    let expired = run_jq(&format!(".users[\"{}\"].expired", username));
    let issued = run_jq(&format!(".users[\"{}\"].issued", username));
    let exp_sec = run_bash_cmd(&format!("date --date='{}' +%s", issued)).parse::<i64>().unwrap_or(0);
    let exp_val = expired.parse::<i64>().unwrap_or(0);
    let exp_date = if exp_sec > 0 {
        run_bash_cmd(&format!("date -d '@{}' '+%Y-%m-%d %H:%M:%S'", exp_sec + (exp_val * 86400)))
    } else {
        "Unknown".to_string()
    };

    println!("\nSuccess Renew NoobzVPN");
    println!("=============================");
    println!("Username: {}@fn-project", username);
    println!("New Expired: {}", exp_date);
    println!("=============================");

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn ubl_noobz() {
    list_noobz();
    let mut username = String::new();
    print!("Input Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut username);
    let username = username.trim().to_string();

    if username.is_empty() {
        return;
    }

    let user_exists = run_jq(&format!(".users[\"{}\"]", username));
    if user_exists.is_empty() || user_exists == "null" {
        println!("Username not found.");
        return;
    }

    let blocked = run_jq(&format!(".users[\"{}\"].blocked", username));
    let new_blocked = if blocked == "true" { "false" } else { "true" };

    #[cfg(target_os = "linux")]
    {
        if new_blocked == "true" {
            let _ = Command::new("noobzvpns").args(&["block", &username]).status();
        } else {
            let _ = Command::new("noobzvpns").args(&["unblock", &username]).status();
        }
    }

    println!("\nSuccess Change Block Status");
    println!("=============================");
    println!("Username      : {}@fn-project", username);
    println!("Status Blocked: {}", new_blocked);
    println!("=============================");

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn delete_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    
    let users_list = run_jq(".users | keys[]");
    for user in users_list.lines() {
        let expired = run_jq(&format!(".users[\"{}\"].expired", user));
        let issued = run_jq(&format!(".users[\"{}\"].issued", user));
        let exp_sec = run_bash_cmd(&format!("date --date='{}' +%s", issued)).parse::<i64>().unwrap_or(0);
        let exp_val = expired.parse::<i64>().unwrap_or(0);
        let exp_date = if exp_sec > 0 {
            run_bash_cmd(&format!("date -d '@{}' '+%Y-%m-%d %H:%M:%S'", exp_sec + (exp_val * 86400)))
        } else {
            "Unknown".to_string()
        };
        println!("Username         : {}", user);
        println!("Expired          : {}", exp_date);
        println!("------------------------------");
    }

    let mut name = String::new();
    print!("Input Name: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut name);
    let name = name.trim().to_string();

    if name.is_empty() {
        return;
    }

    if let Ok(content) = fs::read_to_string(DB_LOGIN_FILE) {
        let mut new_lines = Vec::new();
        let mut found = false;
        for line in content.lines() {
            if line.contains(&name) {
                found = true;
            } else {
                new_lines.push(line.to_string());
            }
        }
        if found {
            let _ = fs::write(DB_LOGIN_FILE, new_lines.join("\n") + "\n");
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("noobzvpns").args(&["remove", &name]).status();
            }
            
            let text_telegram = format!(
                "════════════════════════════\n\
                 Username Delete\n\
                 ════════════════════════════\n\n\
                 User: {}\n\
                 ════════════════════════════",
                name
            );
            send_telegram_notification(&text_telegram);
            println!("User {} berhasil dihapus!", name);
        } else {
            println!("User {} tidak ditemukan di database!", name);
        }
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn rbd_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("noobzvpns").arg("print-all").status();
    }
    println!("======================");
    println!("Format Reset");
    println!("======================");
    println!("username = Reset Username");
    println!("all = Reset All Usernames");
    println!("======================");

    let mut format = String::new();
    print!("Input Reset Format: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut format);
    let format = format.trim().to_string();

    match format.as_str() {
        "username" => {
            let mut username = String::new();
            print!("Input Username For Reset: ");
            let _ = io::stdout().flush();
            let _ = io::stdin().read_line(&mut username);
            let username = username.trim().to_string();
            
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("noobzvpns").args(&["reset", &username]).status();
            }
            println!("\nSuccess Reset");
            println!("=================");
            println!("Username: {}@fn-project", username);
            println!("Status  : Reset");
            println!("=================");
        }
        "all" => {
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("noobzvpns").args(&["opts", "--reset-all"]).status();
            }
            println!("\nSuccess Reset");
            println!("=================");
            println!("Username: All Account on Database");
            println!("Status  : Reset");
            println!("=================");
        }
        _ => {
            println!("Invalid format.");
        }
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cek_login_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("────────────────────────────────────────────────────");
    println!("** Cek User Login NoobzVPN **");
    println!("────────────────────────────────────────────────────");

    let users_list = run_jq(".users | keys[]");
    for user in users_list.lines() {
        let active = run_jq(&format!(".users[\"{}\"].statistic.active_devices[]?", user));
        let limit = run_jq(&format!(".users[\"{}\"].devices", user));
        let up = run_jq(&format!(".users[\"{}\"].statistic.bytes_usage.up", user)).parse::<i64>().unwrap_or(0);
        let down = run_jq(&format!(".users[\"{}\"].statistic.bytes_usage.down", user)).parse::<i64>().unwrap_or(0);

        if !active.is_empty() {
            let num_active = active.lines().count();
            println!("Username: {}", user);
            println!("Upload & Download: {} B / {} B", up, down);
            println!("Total Usage Bandwidth: {} B", up + down);
            println!("Total User Login Device: {} / {}", num_active, limit);
            println!("Hash Device Login:");
            println!("{}", active);
            println!("────────────────────────────────────────────────────");
        }
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn config_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("NoobzVPN Config Settings");
    println!("=======================");
    
    // Wrapper for config-noobz options
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("noobzvpns").arg("opts").status();
    }
    
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

// Option wrappers
pub fn cpu_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Rename NoobzVPN User");
    println!("=====================");
    let mut old_user = String::new();
    print!("Input Username to rename: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut old_user);
    let old_user = old_user.trim().to_string();

    let mut new_user = String::new();
    print!("Input New Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut new_user);
    let new_user = new_user.trim().to_string();

    if !old_user.is_empty() && !new_user.is_empty() {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("noobzvpns").args(&["rename", &old_user, &new_user]).status();
        }
        
        // Update .noobz.db
        let db_content = fs::read_to_string(DB_LOGIN_FILE).unwrap_or_default();
        let mut new_lines = Vec::new();
        for line in db_content.lines() {
            if line.contains(&old_user) {
                new_lines.push(line.replace(&old_user, &new_user));
            } else {
                new_lines.push(line.to_string());
            }
        }
        let _ = fs::write(DB_LOGIN_FILE, new_lines.join("\n") + "\n");
        
        println!("\n=============================");
        println!("Sukses Mengubah Username");
        println!("=============================");
        println!("Username Lama: {}@fn-project", old_user);
        println!("Username Baru: {}@fn-project", new_user);
        println!("=============================");
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cpw_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Change NoobzVPN Password");
    println!("=========================");
    let mut user = String::new();
    print!("Input Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    let mut new_pass = String::new();
    print!("Input New Password: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut new_pass);
    let new_pass = new_pass.trim().to_string();

    if !user.is_empty() && !new_pass.is_empty() {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("noobzvpns").args(&["edit", &user, "-p", &new_pass]).status();
        }
        
        // Update .noobz.db
        let db_content = fs::read_to_string(DB_LOGIN_FILE).unwrap_or_default();
        let mut new_lines = Vec::new();
        for line in db_content.lines() {
            if line.contains(&user) {
                // Format: #noobzvpns# user pass expi
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    new_lines.push(format!("{} {} {} {}", parts[0], parts[1], new_pass, parts[3]));
                    continue;
                }
            }
            new_lines.push(line.to_string());
        }
        let _ = fs::write(DB_LOGIN_FILE, new_lines.join("\n") + "\n");

        println!("\n=============================");
        println!("Success Change Password");
        println!("=============================");
        println!("Username: {}@fn-project", user);
        println!("New Password: {}", new_pass);
        println!("=============================");
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cpb_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Change NoobzVPN Bandwidth Limit");
    println!("==============================");
    let mut user = String::new();
    print!("Input Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    let mut new_limit = String::new();
    print!("Input New Bandwidth Limit (GB): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut new_limit);
    let new_limit = new_limit.trim().to_string();

    if !user.is_empty() && !new_limit.is_empty() {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("noobzvpns").args(&["edit", &user, "-b", &new_limit]).status();
        }
        println!("Bandwidth limit updated for {}.", user);
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cpi_noobz() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Change NoobzVPN Max Device Limit");
    println!("==============================");
    let mut user = String::new();
    print!("Input Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    let mut new_limit = String::new();
    print!("Input New Device Limit: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut new_limit);
    let new_limit = new_limit.trim().to_string();

    if !user.is_empty() && !new_limit.is_empty() {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("noobzvpns").args(&["edit", &user, "-d", &new_limit]).status();
        }
        println!("Device limit updated for {}.", user);
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
