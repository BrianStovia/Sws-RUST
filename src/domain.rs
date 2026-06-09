use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, get_ip, run_bash_cmd};

fn get_acme_domain() {
    println!("\x1B[1;32m--->\x1B[0m    Start ");
    let interface = run_bash_cmd("ip route | grep default | awk '{print $5}'");
    let _ = &interface;
    
    #[cfg(target_os = "linux")]
    {
        if !interface.is_empty() {
            let _ = Command::new("iptables").args(&["-t", "nat", "-D", "PREROUTING", "-i", &interface, "-p", "tcp", "--dport", "80", "-j", "REDIRECT", "--to-port", "2080"]).status();
        }
        let _ = Command::new("iptables").args(&["-t", "nat", "-D", "PREROUTING", "-p", "tcp", "--dport", "80", "-j", "REDIRECT", "--to-port", "2080"]).status();
        let _ = Command::new("systemctl").args(&["stop", "nginx"]).status();
        let _ = run_bash_cmd("pkill sslh");
        let _ = Command::new("systemctl").args(&["stop", "sslh"]).status();
    }

    println!("\x1B[1;32m--->\x1B[0m    Starting renew cert ");
    let domain = get_domain();
    let _ = &domain;


    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("/root/.acme.sh/acme.sh").args(&["--upgrade", "--auto-upgrade"]).status();
        let _ = Command::new("/root/.acme.sh/acme.sh").args(&["--set-default-ca", "--server", "letsencrypt"]).status();
        let _ = Command::new("/root/.acme.sh/acme.sh").args(&["--issue", "-d", &domain, "--standalone", "-k", "ec-256"]).status();
        let _ = Command::new("/root/.acme.sh/acme.sh").args(&["--installcert", "-d", &domain, "--fullchainpath", "/usr/local/etc/v2ray/v2ray.crt", "--keypath", "/usr/local/etc/v2ray/v2ray.key", "--ecc"]).status();
        
        println!("\x1B[1;32m--->\x1B[0m    Renew cert done ");
        let _ = Command::new("systemctl").args(&["restart", "nginx"]).status();
        let _ = Command::new("systemctl").args(&["restart", "sslh"]).status();
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
        
        if !interface.is_empty() {
            let _ = Command::new("iptables").args(&["-t", "nat", "-A", "PREROUTING", "-i", &interface, "-p", "tcp", "--dport", "80", "-j", "REDIRECT", "--to-port", "2080"]).status();
        } else {
            let _ = Command::new("iptables").args(&["-t", "nat", "-A", "PREROUTING", "-p", "tcp", "--dport", "80", "-j", "REDIRECT", "--to-port", "2080"]).status();
        }
    }
    println!("\nCert Done.");
}

fn renew_domain() {
    let mut domain = String::new();
    print!("Input ur Domain/Host : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut domain);
    let domain = domain.trim().to_string();

    if domain.is_empty() {
        return;
    }

    let old_domain = get_domain();
    let _ = fs::rename("/usr/local/etc/v2ray/domain", "/usr/local/etc/v2ray/domain.old");
    let _ = fs::write("/usr/local/etc/v2ray/domain", &domain);

    // Update nginx configs
    if Path::new("/etc/nginx").exists() {
        let replace_cmd = format!("sed -i 's/{}/{}/g' /etc/nginx/nginx.conf 2>/dev/null", old_domain, domain);
        let _ = run_bash_cmd(&replace_cmd);
    }

    get_acme_domain();
}

pub fn run_change_domain() {
    print!("\x1B[2J\x1B[1;1H");
    let lastdomain = get_domain();
    let ip = get_ip();
    let ram = run_bash_cmd("grep 'MemTotal: ' /proc/meminfo | awk '{ print $2}'").parse::<i64>().unwrap_or(4096000) / 1024;
    
    println!("───────────────────────────");
    println!("Hostname   : {}", lastdomain);
    println!("Public ip  : {}", ip);
    println!("Total RAM  : {} MB", ram);
    println!("───────────────────────────");
    println!("1). MANUAL POINTING");
    println!("2). Renew Certificate");
    println!();
    
    let mut choice = String::new();
    print!("what do you choose[ 1 - 2 ] : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut choice);
    
    match choice.trim() {
        "1" => renew_domain(),
        "2" => get_acme_domain(),
        _ => println!("\x1B[1;31mYou wrong command !\x1B[0m"),
    }

    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
