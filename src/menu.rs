use std::io::{self, Write};
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, run_bash_cmd, generate_uuid};

use crate::account_ssh::{add_ssh, del_ssh, renew_ssh, cek_ssh};
use crate::account_xray::{add_xray, del_xray, renew_xray, cek_xray, trafik_xray};
use crate::account_noobz::{
    add_noobz, list_noobz, ex_noobz, ubl_noobz, rbd_noobz, config_noobz, 
    cek_login_noobz, cpu_noobz, cpw_noobz, cpb_noobz, cpi_noobz, delete_noobz
};
use crate::account_wg::{add_wg, del_wg, renew_wg, cek_wg};

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}

pub fn menu_ssh() {
    loop {
        clear_screen();
        println!("────────────────────────────");
        println!("      Menu SSH WebSocket      ");
        println!("────────────────────────────");
        println!("1. Create SSH Account ");
        println!("2. Renew SSH Account ");
        println!("3. Cek    SSH Login ");
        println!("4. Delete SSH Account ");
        println!("────────────────────────────");
        println!("0. Back To Main Menu ");
        println!("────────────────────────────");
        
        let mut choice = String::new();
        print!("Input Options: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_ssh(),
            "2" => renew_ssh(),
            "3" => cek_ssh(),
            "4" => del_ssh(),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_vmess() {
    loop {
        clear_screen();
        println!("────────────────────────────");
        println!("      Menu V2ray Vmess      ");
        println!("────────────────────────────");
        println!("1. Create Vmess Account ");
        println!("2. Cek    Vmess Login ");
        println!("3. Delete Vmess Account ");
        println!("4. Renew  Vmess Account ");
        println!("5. Trafik Vmess Account ");
        println!("────────────────────────────");
        println!("0. Back To Main Menu ");
        println!("────────────────────────────");
        
        let mut choice = String::new();
        print!("Input Options: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_xray("vmess"),
            "2" => cek_xray("vmess"),
            "3" => del_xray("vmess"),
            "4" => renew_xray("vmess"),
            "5" => trafik_xray("vmess"),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_vless() {
    loop {
        clear_screen();
        println!("────────────────────────────");
        println!("      Menu V2ray Vless      ");
        println!("────────────────────────────");
        println!("1. Create Vless Account ");
        println!("2. Cek    Vless Login ");
        println!("3. Delete Vless Account ");
        println!("4. Renew  Vless Account ");
        println!("5. Trafik Vless Account ");
        println!("────────────────────────────");
        println!("0. Back To Main Menu ");
        println!("────────────────────────────");
        
        let mut choice = String::new();
        print!("Input Options: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_xray("vless"),
            "2" => cek_xray("vless"),
            "3" => del_xray("vless"),
            "4" => renew_xray("vless"),
            "5" => trafik_xray("vless"),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_trojan() {
    loop {
        clear_screen();
        println!("────────────────────────────");
        println!("      Menu V2ray Trojan      ");
        println!("────────────────────────────");
        println!("1. Create Trojan Account ");
        println!("2. Cek    Trojan Login ");
        println!("3. Delete Trojan Account ");
        println!("4. Renew  Trojan Account ");
        println!("5. Trafik Trojan Account ");
        println!("────────────────────────────");
        println!("0. Back To Main Menu ");
        println!("────────────────────────────");
        
        let mut choice = String::new();
        print!("Input Options: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_xray("trojan"),
            "2" => cek_xray("trojan"),
            "3" => del_xray("trojan"),
            "4" => renew_xray("trojan"),
            "5" => trafik_xray("trojan"),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_wg() {
    loop {
        clear_screen();
        println!("────────────────────────────");
        println!("      Menu WireGuard       ");
        println!("────────────────────────────");
        println!("1. Create WireGuard Account ");
        println!("2. Cek    WireGuard Login ");
        println!("3. Delete WireGuard Account ");
        println!("4. Renew  WireGuard Account ");
        println!("────────────────────────────");
        println!("0. Back To Main Menu ");
        println!("────────────────────────────");
        
        let mut choice = String::new();
        print!("Input Options: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_wg(),
            "2" => cek_wg(),
            "3" => del_wg(),
            "4" => renew_wg(),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_reality() {
    loop {
        clear_screen();
        println!("────────────────────────────");
        println!("      Menu Xray Reality     ");
        println!("────────────────────────────");
        println!("1. Create Reality Account ");
        println!("2. Cek    Reality Login ");
        println!("3. Delete Reality Account ");
        println!("4. Renew  Reality Account ");
        println!("5. Trafik Reality Account ");
        println!("────────────────────────────");
        println!("0. Back To Main Menu ");
        println!("────────────────────────────");
        
        let mut choice = String::new();
        print!("Input Options: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_xray("reality"),
            "2" => cek_xray("reality"),
            "3" => del_xray("reality"),
            "4" => renew_xray("reality"),
            "5" => trafik_xray("reality"),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_noobz() {
    loop {
        clear_screen();
        let status = if run_bash_cmd("systemctl is-active noobzvpns").trim() == "active" {
            "\x1B[1;32mON\x1B[0m"
        } else {
            "\x1B[1;31mOFF\x1B[0m"
        };
        
        println!("════════════════════════════════");
        println!("[ <== NOOBZVPN ==> ]");
        println!("════════════════════════════════");
        println!("Noobz: {}", status);
        println!("\n\
                  1.  Add     NoobzVPN   Account\n\
                  2.  Change  Password   Account\n\
                  3.  Change  Username   Account\n\
                  4.  Change  Bandwidth  Limit Account\n\
                  5.  Change  Device     Limit Account\n\
                  6.  Extend  Expired    Account\n\
                  7.  Unblock & Block    Account\n\
                  8.  Delete  & Remove   Account\n\
                  9.  Reset   Bandwidth & Device Login\n\
                  10. List    Total      Account\n\
                  11. Check   Config     Account\n\
                  12. Check   User       Login");
        println!("════════════════════════════════");
        println!("0. Back To Main Menu ");
        println!("════════════════════════════════");
        
        let mut choice = String::new();
        print!("Input Option: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => add_noobz(),
            "2" => cpw_noobz(),
            "3" => cpu_noobz(),
            "4" => cpb_noobz(),
            "5" => cpi_noobz(),
            "6" => ex_noobz(),
            "7" => ubl_noobz(),
            "8" => delete_noobz(),
            "9" => rbd_noobz(),
            "10" => list_noobz(),
            "11" => config_noobz(),
            "12" => cek_login_noobz(),
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_bot() {
    loop {
        clear_screen();
        println!("======================");
        println!("[      Bot Telegram      ]");
        println!("======================");
        println!("\n\
                  1. Setup Bot Notification\n\
                  2. Setup Bot Panel All Menu\n\
                  3. Setup Bot WhatsApp Panel (Coming Soon)\n\
                  4. Setup Bot Reseller / Store VPN (Coming Soon)\n\
                  0. Back To Menu System");
        println!("======================");
        
        let mut choice = String::new();
        print!("Input Option: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => {
                clear_screen();
                println!("===================");
                println!("[ Setup Bot Notification ]");
                println!("===================");
                let mut api = String::new();
                print!("API Key Bot: ");
                let _ = io::stdout().flush();
                let _ = io::stdin().read_line(&mut api);
                
                let mut itd = String::new();
                print!("Your Chat ID: ");
                let _ = io::stdout().flush();
                let _ = io::stdin().read_line(&mut itd);
                
                let api = api.trim().to_string();
                let itd = itd.trim().to_string();
                
                if !api.is_empty() && !itd.is_empty() {
                    let _ = fs::create_dir_all("/usr/local/etc/v2ray");
                    let _ = fs::write("/usr/local/etc/v2ray/bot.key", &api);
                    let _ = fs::write("/usr/local/etc/v2ray/client.id", &itd);
                    
                    println!("\nYour Data Bot Notification Saved successfully.");
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
            "2" => {
                clear_screen();
                println!("==================================");
                println!("     Setup Bot Panel Telegram     ");
                println!("==================================");
                if !Path::new("/usr/local/etc/v2ray/bot.key").exists() {
                    println!("\x1B[1;31mError: Bot Notification belum di-setup!\x1B[0m");
                    println!("Silakan pilih opsi 1 terlebih dahulu untuk memasukkan API Bot Key.");
                    pause();
                    continue;
                }
                
                println!("1. Aktifkan / Restart Bot Panel");
                println!("2. Matikan / Nonaktifkan Bot Panel");
                println!("0. Kembali");
                println!("==================================");
                
                let mut opt_panel = String::new();
                print!("Pilihan Anda: ");
                let _ = io::stdout().flush();
                let _ = io::stdin().read_line(&mut opt_panel);
                
                match opt_panel.trim() {
                    "1" => {
                        println!("Memasang Telegram Bot Panel...");
                        let _ = fs::create_dir_all("/usr/bin");
                        let _ = run_bash_cmd("cp /usr/local/sbin/vpn-telegram-bot /usr/bin/vpn-telegram-bot 2>/dev/null || cp ./vpn-telegram-bot /usr/bin/vpn-telegram-bot 2>/dev/null");
                        let _ = Command::new("chmod").args(&["+x", "/usr/bin/vpn-telegram-bot"]).status();
                        
                        let service_content = 
                            "[Unit]\n\
                             Description=VPN Seller Telegram Bot Daemon\n\
                             After=network.target\n\n\
                             [Service]\n\
                             Type=simple\n\
                             User=root\n\
                             WorkingDirectory=/root\n\
                             ExecStart=/usr/bin/vpn-telegram-bot\n\
                             Restart=always\n\
                             RestartSec=5\n\n\
                             [Install]\n\
                             WantedBy=multi-user.target\n";
                             
                        let _ = fs::write("/etc/systemd/system/vpn-bot.service", service_content);
                        #[cfg(target_os = "linux")]
                        {
                            let _ = Command::new("systemctl").arg("daemon-reload").status();
                            let _ = Command::new("systemctl").arg("enable").arg("vpn-bot").status();
                            let _ = Command::new("systemctl").arg("restart").arg("vpn-bot").status();
                        }
                        
                        println!("\x1B[1;32mSuccess: Bot Panel berhasil diaktifkan!\x1B[0m");
                        println!("Silakan buka Telegram, lalu ketik /start pada Bot Anda.");
                        pause();
                    }
                    "2" => {
                        println!("Menonaktifkan Bot Panel...");
                        #[cfg(target_os = "linux")]
                        {
                            let _ = Command::new("systemctl").arg("stop").arg("vpn-bot").status();
                            let _ = Command::new("systemctl").arg("disable").arg("vpn-bot").status();
                        }
                        let _ = fs::remove_file("/etc/systemd/system/vpn-bot.service");
                        #[cfg(target_os = "linux")]
                        {
                            let _ = Command::new("systemctl").arg("daemon-reload").status();
                        }
                        println!("\x1B[1;32mSuccess: Bot Panel berhasil dinonaktifkan.\x1B[0m");
                        pause();
                    }
                    _ => {}
                }
            }
            "0" => break,
            _ => {}
        }
    }
}

pub fn menu_api() {
    loop {
        clear_screen();
        let service_running = if run_bash_cmd("systemctl is-active server").trim() == "active" {
            "\x1B[1;32m[ ON ]\x1B[0m"
        } else {
            "\x1B[1;31m[ OFF ]\x1B[0m"
        };
        let domain = get_domain();
        
        println!("<= Menu Web API =>");
        println!("==================");
        println!("With Port:");
        println!("- http://{}:9000/path", domain);
        println!("==================");
        println!("Default Port:");
        println!("- http://{}/api/path", domain);
        println!("- https://{}/api/path", domain);
        println!("==================");
        println!("Status: {}", service_running);
        println!();
        println!("1. Generate New Key Token");
        println!("2. Change Manual Key Token");
        println!("3. Add Key Token API");
        println!("4. Enable API");
        println!("5. Restart API");
        println!("6. Disable API");
        println!("0. Back To Default Menu");
        println!("==================");
        
        let mut choice = String::new();
        print!("Input Option: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut choice);
        
        match choice.trim() {
            "1" => {
                let uuid = generate_uuid();
                let _ = fs::create_dir_all("/etc/api");
                let _ = fs::write("/etc/api/key", format!("{}\n", uuid));
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").arg("daemon-reload").status();
                    let _ = Command::new("systemctl").args(&["enable", "server"]).status();
                    let _ = Command::new("systemctl").args(&["restart", "server"]).status();
                }
                println!("\nSuccess Generate New Key");
                println!("========================");
                println!("Your API Token: {}", uuid);
                println!("========================");
                pause();
            }
            "2" => {
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("nano").arg("/etc/api/key").status();
                }
            }
            "3" => {
                let mut token = String::new();
                print!("Input Token: ");
                let _ = io::stdout().flush();
                let _ = io::stdin().read_line(&mut token);
                let token = token.trim().to_string();
                if !token.is_empty() {
                    let _ = fs::create_dir_all("/etc/api");
                    let mut key_content = fs::read_to_string("/etc/api/key").unwrap_or_default();
                    key_content.push_str(&format!("{}\n", token));
                    let _ = fs::write("/etc/api/key", key_content);
                    
                    #[cfg(target_os = "linux")]
                    {
                        let _ = Command::new("systemctl").arg("daemon-reload").status();
                        let _ = Command::new("systemctl").args(&["enable", "server"]).status();
                        let _ = Command::new("systemctl").args(&["restart", "server"]).status();
                    }
                    println!("\nSuccess Add New Key API");
                }
                pause();
            }
            "4" => {
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").arg("daemon-reload").status();
                    let _ = Command::new("systemctl").args(&["enable", "server"]).status();
                    let _ = Command::new("systemctl").args(&["start", "server"]).status();
                    let _ = Command::new("systemctl").args(&["restart", "server"]).status();
                }
                println!("API Enabled.");
                pause();
            }
            "5" => {
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").arg("daemon-reload").status();
                    let _ = Command::new("systemctl").args(&["restart", "server"]).status();
                }
                println!("API Restarted.");
                pause();
            }
            "6" => {
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").args(&["stop", "server"]).status();
                    let _ = Command::new("systemctl").args(&["disable", "server"]).status();
                }
                println!("API Disabled.");
                pause();
            }
            "0" => break,
            _ => {}
        }
    }
}

fn pause() {
    print!("Press Enter to continue...");
    let _ = io::stdout().flush();
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
