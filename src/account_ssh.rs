use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, get_ip, send_telegram_notification, run_bash_cmd};

// Check if username already exists
fn username_exists(username: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        Command::new("id").arg(username).output().map(|out| out.status.success()).unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = username;
        false
    }
}

pub fn add_ssh() {
    print!("\x1B[2J\x1B[1;1H");
    println!("———————————————————");
    println!(" Create SSH Account ");
    println!("———————————————————");

    let mut username = String::new();
    print!("Input Username: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut username);
    let username = username.trim().to_string();

    if username.is_empty() {
        println!("Error: Username cannot be empty.");
        return;
    }

    if username_exists(&username) {
        println!("\x1B[31m[404 Not Found]\x1B[0m Username '{}' already used.", username);
        return;
    }

    let mut password = String::new();
    print!("Input Password: ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut password);
    let password = password.trim().to_string();

    let mut limit_input = String::new();
    print!("Limit IP Login (Default 2): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut limit_input);
    let mut limit = limit_input.trim().to_string();
    if limit.is_empty() {
        limit = "2".to_string();
    }
    let _ = &limit; // Prevent unused variable warning on non-linux systems


    let mut days_str = String::new();
    print!("Expired ( Days ): ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut days_str);
    let days: i64 = days_str.trim().parse().unwrap_or(30);

    // Calculate expiration date
    let exp_date = run_bash_cmd(&format!("date +%F -d '{} days'", days));
    let exp_date = if exp_date.is_empty() { "2026-12-31".to_string() } else { exp_date };

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("useradd")
            .args(&[
                "-e", &exp_date,
                "-M", "-N",
                "-s", "/bin/false",
                "-c", &format!("limit={}", limit),
                &username
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                // Set password
                let chpass_cmd = format!("echo '{}:{}' | chpasswd", username, password);
                let _ = Command::new("bash").args(&["-c", &chpass_cmd]).status();
            }
            _ => {
                println!("Error: Failed to create system user.");
                return;
            }
        }
    }

    let domain = get_domain();
    let ip = get_ip();

    let nameserver = run_bash_cmd("cat /etc/slowdns/nameserver 2>/dev/null");
    let nameserver = if nameserver.is_empty() { "not set".to_string() } else { nameserver };

    let pubkey = run_bash_cmd("cat /etc/slowdns/server.pub 2>/dev/null");
    let pubkey = if pubkey.is_empty() { "not set".to_string() } else { pubkey };

    let payload = format!("GET / HTTP/1.1[crlf]Host: {}[crlf]Upgrade: websocket[crlf][crlf]", domain);

    let text_telegram = format!(
        "<b>Success Create SSH Account</b>\n\
         <b>———————————————————</b>\n\
         <b>Domain:</b> <code>{}</code> / <code>{}</code>\n\
         <b>Username:</b> <code>{}</code>\n\
         <b>Password:</b> <code>{}</code>\n\
         <b>———————————————————</b>\n\
         <b>Port OpenSSH:</b> <code>443</code>\n\
         <b>Port WS HTTP:</b> <code>80, 2082</code>\n\
         <b>Port WS TLS:</b> <code>443</code>\n\
         <b>Port SSL/TLS (Stunnel):</b> <code>222, 777, 990</code>\n\
         <b>Port Socks5:</b> <code>443, 1080</code>\n\
         <b>Port UDP Custom:</b> <code>1-65535</code> &amp; <code>36712</code>\n\
         <b>Port BadVPN:</b> <code>7300</code>\n\
         <b>———————————————————</b>\n\
         <b>DNS:</b> <code>1.1.1.1/8.8.8.8</code>\n\
         <b>Publik Key:</b> <code>{}</code>\n\
         <b>Nameserver:</b> <code>{}</code>\n\
         <b>———————————————————</b>\n\
         <b>Config HTTP Custom:</b> <code>{}:1-65535@{:?}:{}</code>\n\
         <b>———————————————————</b>\n\
         <b>Payload:</b> <code>{}</code>\n\
         <b>———————————————————</b>\n\
         <b>Expired:</b> <code>{}</code>\n\
         <b>———————————————————</b>",
        domain, ip, username, password, pubkey, nameserver, domain, username, password, payload, exp_date
    );

    send_telegram_notification(&text_telegram);

    print!("\x1B[2J\x1B[1;1H");
    println!(" Success Create SSH Account ");
    println!("———————————————————");
    println!("Domain: {} / {}", domain, ip);
    println!("Username: {}", username);
    println!("Password: {}", password);
    println!("———————————————————");
    println!("Port OpenSSH: 443");
    println!("Port WS HTTP: 80, 2082");
    println!("Port WS TLS: 443");
    println!("Port SSL/TLS (Stunnel): 222, 777, 990");
    println!("Port Socks5: 443, 1080");
    println!("Port UDP Custom: 1-65535 & 36712");
    println!("Port BadVPN: 7300");
    println!("Port Slowdns: 53, 5300");
    println!("———————————————————");
    println!("DNS: 1.1.1.1, 8.8.8.8");
    println!("Nameserver: {}", nameserver);
    println!("Publik Key: {}", pubkey);
    println!("———————————————————");
    println!("Config HTTP Custom: {}:1-65535@{:?}:{}", domain, username, password);
    println!("———————————————————");
    println!("Payload: {}", payload);
    println!("———————————————————");
    println!("Expired: {}", exp_date);
    println!("———————————————————");
    
    // Pause for user keypress before returning to menu
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn del_ssh() {
    print!("\x1B[2J\x1B[1;1H");
    println!("\x1B[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1B[0m");
    println!("\x1B[0;41;36m                 AKUN SSH                 \x1B[0m");
    println!("\x1B[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1B[0m");
    println!("USERNAME          EXP DATE          STATUS");
    println!("\x1B[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1B[0m");

    let passwd_content = fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut total_users = 0;
    
    for line in passwd_content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 3 {
            continue;
        }
        let username = parts[0];
        if username == "nobody" {
            continue;
        }
        if let Ok(uid) = parts[2].parse::<u32>() {
            if uid >= 1000 {
                total_users += 1;
                
                let exp = run_bash_cmd(&format!("chage -l {} | grep 'Account expires' | awk -F': ' '{{print $2}}'", username));
                let exp = if exp.is_empty() { "Never".to_string() } else { exp };
                
                let status_raw = run_bash_cmd(&format!("passwd -S {} | awk '{{print $2}}'", username));
                let status = if status_raw.starts_with('L') { "LOCKED" } else { "UNLOCKED" };
                
                println!("{:<17} {:<17} {}", username, exp, status);
            }
        }
    }

    println!("\x1B[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1B[0m");
    println!("Account number: {} user", total_users);
    println!("\x1B[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1B[0m");
    println!();

    let mut delete_user = String::new();
    print!("Username SSH to Delete : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut delete_user);
    let delete_user = delete_user.trim().to_string();

    if delete_user.is_empty() {
        return;
    }

    let limit_dir = format!("/etc/funny/limit/ssh/ip/{}", delete_user);
    if Path::new(&limit_dir).exists() {
        let _ = fs::remove_dir_all(&limit_dir);
    }

    #[cfg(target_os = "linux")]
    {
        if username_exists(&delete_user) {
            let _ = Command::new("userdel").args(&["--force", &delete_user]).status();
            println!("User {} was removed.", delete_user);
            let _ = Command::new("systemctl").args(&["restart", "ssh"]).status();
            let _ = Command::new("systemctl").args(&["restart", "udp-custom"]).status();
        } else {
            println!("Failure: User {} Not Exist.", delete_user);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("User {} was removed. (Mocked)", delete_user);
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn renew_ssh() {
    print!("\x1B[2J\x1B[1;1H");
    println!("━━━━━━━━━━━");
    println!("RENEW  USER ");
    println!("━━━━━━━━━━━");
    println!();

    let mut user = String::new();
    print!("Username : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut user);
    let user = user.trim().to_string();

    if user.is_empty() {
        return;
    }

    let exists = if cfg!(target_os = "linux") {
        username_exists(&user)
    } else {
        true
    };

    if exists {
        let mut days_str = String::new();
        print!("Day Extend : ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut days_str);
        let days: i64 = days_str.trim().parse().unwrap_or(30);

        // Check if we can get system time
        let today_secs = run_bash_cmd("date +%s").parse::<i64>().unwrap_or(1770000000);
        let expire_on = today_secs + (days * 86400);
        
        let expiration = run_bash_cmd(&format!("date -u --date='1970-01-01 {} sec GMT' +%Y/%m/%d", expire_on));
        let expiration = if expiration.is_empty() { "2026/12/31".to_string() } else { expiration };
        let _ = &expiration; // Prevent unused variable warning on non-linux systems
        
        let expiration_display = run_bash_cmd(&format!("date -u --date='1970-01-01 {} sec GMT' '+%d %b %Y'", expire_on));
        let expiration_display = if expiration_display.is_empty() { "31 Dec 2026".to_string() } else { expiration_display };

        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("passwd").args(&["-u", &user]).status();
            let _ = Command::new("usermod").args(&["-e", &expiration, &user]).status();
        }

        print!("\x1B[2J\x1B[1;1H");
        println!("━━━━━━━━━━━");
        println!("RENEW  USER ");
        println!("━━━━━━━━━━━");
        println!();
        println!(" Username : {}", user);
        println!(" Days Added : {} Days", days);
        println!(" Expires on :  {}", expiration_display);
        println!();
        println!("━━━━━━━━━━━");
    } else {
        print!("\x1B[2J\x1B[1;1H");
        println!("━━━━━━━━━━━");
        println!("RENEW  USER ");
        println!("━━━━━━━━━━━");
        println!();
        println!("   Username Doesnt Exist      ");
        println!();
        println!("━━━━━━━━━━━");
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn cek_ssh() {
    print!("\x1B[2J\x1B[1;1H");
    #[cfg(target_os = "linux")]
    {
        if Path::new("/usr/local/sbin/ssh-limit").exists() {
            let _ = Command::new("/usr/local/sbin/ssh-limit").arg("--check").status();
        } else {
            println!("Error: ssh-limit script not found in /usr/local/sbin/");
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("(Mocking ssh-limit check output for dev system)");
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
