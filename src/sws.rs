mod utils;
mod menu;
mod account_ssh;
mod account_xray;
mod account_noobz;
mod account_wg;
mod xp;
mod dns;
mod backup;
mod domain;
mod api;

use std::path::Path;
use std::io::{self, Write};
use std::fs;
#[cfg(target_os = "linux")]
use std::process::Command;

fn get_command_name() -> String {
    let args: Vec<String> = std::env::args().collect();
    if let Some(exe_path) = args.get(0) {
        if let Some(filename) = Path::new(exe_path).file_name() {
            let name = filename.to_string_lossy().to_string();
            let name_without_ext = name.strip_suffix(".exe").unwrap_or(&name).to_string();
            if name_without_ext != "sws" {
                return name_without_ext;
            }
        }
    }
    if args.len() > 1 {
        return args[1].clone();
    }
    "menu".to_string()
}

fn is_api_call() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if let Some(exe_path) = args.get(0) {
        if exe_path.contains("api/") || exe_path.contains("/api") {
            return true;
        }
    }
    false
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}

fn check_service(service: &str) -> &'static str {
    #[cfg(target_os = "linux")]
    {
        let res = Command::new("systemctl")
            .args(&["is-active", "--quiet", service])
            .status();
        match res {
            Ok(status) if status.success() => "\x1B[1;32mON\x1B[0m",
            _ => "\x1B[1;31mOFF\x1B[0m",
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = service;
        "\x1B[1;32mON (Mock)\x1B[0m"
    }
}

fn get_ssh_count() -> usize {
    let path = "/etc/passwd";
    if !Path::new(path).exists() {
        return 0;
    }
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .filter(|line| {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        return uid >= 1000 && parts[0] != "nobody";
                    }
                }
                false
            })
            .count()
    } else {
        0
    }
}

fn get_config_client_count(tag: &str) -> usize {
    let path = "/usr/local/etc/v2ray/config.json";
    if !Path::new(path).exists() {
        return 0;
    }
    if let Ok(content) = fs::read_to_string(path) {
        content.lines().filter(|line| line.trim().starts_with(tag)).count()
    } else {
        0
    }
}

fn get_noobz_count() -> usize {
    let path = "/etc/noobzvpns/.noobz.db";
    if !Path::new(path).exists() {
        return 0;
    }
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .filter(|line| line.starts_with("#noobzvpns#"))
            .count()
    } else {
        0
    }
}

fn show_main_menu() {
    loop {
        clear_screen();
        
        let domain = get_domain();
            
        #[cfg(target_os = "linux")]
        let os_version = match Command::new("lsb_release").arg("-ds").output() {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            Err(_) => "Unknown OS".to_string(),
        };
        #[cfg(not(target_os = "linux"))]
        let os_version = "Unknown OS".to_string();
        
        println!("─────────────────────────────────────────────────────");
        println!("[                 Autoscript By rbstv               ]");
        println!("─────────────────────────────────────────────────────");
        println!(" OS             : {}", os_version);
        println!(" Domain         : {}", domain);
        println!("─────────────────────────────────────────────────────");
        println!("Total Accounts:                                       ");
        println!(" SSH            : {}", get_ssh_count());
        println!(" Noobz VPN      : {}", get_noobz_count());
        println!(" Vmess          : {}", get_config_client_count("###"));
        println!(" Vless          : {}", get_config_client_count("#&"));
        println!(" Trojan         : {}", get_config_client_count("#!"));
        println!("─────────────────────────────────────────────────────");
        println!("VPN Menus:                                            ");
        println!(" 01. Menu SSH                                        ");
        println!(" 02. Menu Noobz VPN                                  ");
        println!(" 03. Menu V2ray Vmess                                ");
        println!(" 04. Menu V2ray Vless                                ");
        println!(" 05. Menu V2ray Trojan                               ");
        println!(" 06. Menu WireGuard                                  ");
        println!(" 07. Menu Xray Reality                               ");
        println!("─────────────────────────────────────────────────────");
        println!("Other Options:                                          ");
        println!(" 08. Menu Bot Telegram                               ");
        println!(" 09. Backup Database                                 ");
        println!(" 10. Restore Database                                ");
        println!(" 11. Menu Domain & Certificate                       ");
        println!(" 12. Restart All Services                             ");
        println!(" 13. Menu Rest API Create Account                    ");
        println!(" 14. Setting SlowDNS                                  ");
        println!(" 00. Exit                                            ");
        println!("─────────────────────────────────────────────────────");
        println!(
            "NoobzVPN: {} | V2ray: {} | Nginx: {} | SSLH: {}",
            check_service("noobzvpns"),
            check_service("v2ray"),
            check_service("nginx"),
            check_service("sslh")
        );
        println!("─────────────────────────────────────────────────────");
        
        print!("Input Option: ");
        let _ = io::stdout().flush();
        
        let mut opt = String::new();
        let _ = io::stdin().read_line(&mut opt);
        let choice = opt.trim();
        
        if choice == "0" || choice == "00" {
            break;
        }
        
        match choice {
            "1" | "01" => menu::menu_ssh(),
            "2" | "02" => menu::menu_noobz(),
            "3" | "03" => menu::menu_vmess(),
            "4" | "04" => menu::menu_vless(),
            "5" | "05" => menu::menu_trojan(),
            "6" | "06" => menu::menu_wg(),
            "7" | "07" => menu::menu_reality(),
            "8" | "08" => menu::menu_bot(),
            "9" | "09" => backup::run_backup(),
            "10" => backup::run_restore(),
            "11" => domain::run_change_domain(),
            "12" => {
                clear_screen();
                println!("Restarting all services...");
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").arg("daemon-reload").status();
                    let _ = Command::new("pkill").arg("sslh").status();
                    let _ = Command::new("systemctl")
                        .args(&[
                            "restart", "v2ray", "proxy", "danted", "badvpn",
                            "udp-custom", "nginx", "sslh", "wg-quick@wg0"
                        ])
                        .status();
                }
                println!("Successfully restarted all services.");
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            "13" => menu::menu_api(),
            "14" => dns::run_slowdns(),
            _ => {}
        }
    }
}

fn get_domain() -> String {
    utils::get_domain()
}

fn main() {
    let cmd_name = get_command_name();
    let is_api = is_api_call();
    
    if is_api {
        match cmd_name.as_str() {
            "addssh" => api::handle_api_addssh(),
            "add-vmess" => api::handle_api_add_vmess(),
            "add-vless" => api::handle_api_add_vless(),
            "add-trojan" => api::handle_api_add_trojan(),
            "add-noobz" => api::handle_api_add_noobz(),
            "renew-vmess" => api::handle_api_renew_vmess(),
            "renew-vless" => api::handle_api_renew_vless(),
            "renew-trojan" => api::handle_api_renew_trojan(),
            "renew-noobz" => api::handle_api_renew_noobz(),
            _ => println!("{{\"status\": \"false\", \"message\": \"Unknown API path\"}}"),
        }
        return;
    }
    
    match cmd_name.as_str() {
        "menu" => show_main_menu(),
        "menu-ssh" => menu::menu_ssh(),
        "menu-noobz" => menu::menu_noobz(),
        "menu-vmess" => menu::menu_vmess(),
        "menu-vless" => menu::menu_vless(),
        "menu-trojan" => menu::menu_trojan(),
        "menu-wg" => menu::menu_wg(),
        "menu-reality" => menu::menu_reality(),
        "menu-api" => menu::menu_api(),
        "bot-menu" => menu::menu_bot(),
        "slowdns" => dns::run_slowdns(),
        
        "add-ssh" | "addssh" => account_ssh::add_ssh(),
        "del-ssh" => account_ssh::del_ssh(),
        "renew-ssh" => account_ssh::renew_ssh(),
        "cek-ssh" => account_ssh::cek_ssh(),
        
        "add-vmess" | "add-vmess-gege" => account_xray::add_xray("vmess"),
        "del-vmess" => account_xray::del_xray("vmess"),
        "renew-vmess" => account_xray::renew_xray("vmess"),
        "cek-vmess" => account_xray::cek_xray("vmess"),
        "trafik-vmess" => account_xray::trafik_xray("vmess"),
        
        "add-vless" | "add-vless-gege" => account_xray::add_xray("vless"),
        "del-vless" => account_xray::del_xray("vless"),
        "renew-vless" => account_xray::renew_xray("vless"),
        "cek-vless" => account_xray::cek_xray("vless"),
        "trafik-vless" => account_xray::trafik_xray("vless"),
        
        "add-tr" | "add-trojan-gege" => account_xray::add_xray("trojan"),
        "del-tr" => account_xray::del_xray("trojan"),
        "renew-tr" => account_xray::renew_xray("trojan"),
        "cek-tr" => account_xray::cek_xray("trojan"),
        "trafik-tr" => account_xray::trafik_xray("trojan"),
        
        "add-reality" => account_xray::add_xray("reality"),
        "del-reality" => account_xray::del_xray("reality"),
        "renew-reality" => account_xray::renew_xray("reality"),
        "cek-reality" => account_xray::cek_xray("reality"),
        
        "add-wg" => account_wg::add_wg(),
        "del-wg" => account_wg::del_wg(),
        "renew-wg" => account_wg::renew_wg(),
        "cek-wg" => account_wg::cek_wg(),
        
        "add-noobz" | "add-noobz-gege" => account_noobz::add_noobz(),
        "renew-noobz" | "ex-noobz" => account_noobz::ex_noobz(),
        "list-noobz" => account_noobz::list_noobz(),
        "cek-login-noobz" => account_noobz::cek_login_noobz(),
        "config-noobz" => account_noobz::config_noobz(),
        "cpb-noobz" => account_noobz::cpb_noobz(),
        "cpi-noobz" => account_noobz::cpi_noobz(),
        "cpu-noobz" => account_noobz::cpu_noobz(),
        "cpw-noobz" => account_noobz::cpw_noobz(),
        "rbd-noobz" => account_noobz::rbd_noobz(),
        "ubl-noobz" => account_noobz::ubl_noobz(),
        
        "xp" => xp::run_xp(),
        "backup" => backup::run_backup(),
        "restore" => backup::run_restore(),
        "change-domain" => domain::run_change_domain(),
        _ => show_main_menu(),
    }
}
