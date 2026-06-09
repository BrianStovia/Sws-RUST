use std::io::{self, Write};
#[cfg(target_os = "linux")]
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::run_bash_cmd;

fn draw_line() {
    println!("\x1B[1;35m──────────────────────────────────────────────────\x1B[0m");
}

fn header() {
    print!("\x1B[2J\x1B[1;1H");
    draw_line();
    println!("\x1B[1;36m         Project Rerechan - SlowDNS Manager\x1B[0m");
    draw_line();
}

fn footer() {
    draw_line();
    println!("\x1B[1;33m  Thank you for using this script by rbstv\x1B[0m");
    draw_line();
}

fn show_status(status: &str) {
    if status == "enabled" {
        println!("\x1B[1;32m[✔] SlowDNS is ENABLED\x1B[0m");
    } else if status == "disabled" {
        println!("\x1B[1;31m[✘] SlowDNS is DISABLED\x1B[0m");
    } else {
        println!("\x1B[1;33m[!] SlowDNS status unknown\x1B[0m");
    }
}

fn show_dns_info() {
    let ns = fs::read_to_string("/etc/slowdns/nameserver").unwrap_or_default().trim().to_string();
    let pub_key = fs::read_to_string("/etc/slowdns/server.pub").unwrap_or_default().trim().to_string();
    println!("\x1B[1;36mNameserver :\x1B[0m \x1B[1;33m{}\x1B[0m", ns);
    println!("\x1B[1;36mPUB Key    :\x1B[0m \x1B[1;34m{}\x1B[0m", pub_key);
}

fn pause() {
    print!("Press Enter to continue...");
    let _ = io::stdout().flush();
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

fn get_core_dns() {
    header();
    println!("\x1B[1;33mChecking and installing dependencies...\x1B[0m");
    let packages = [
        "curl", "wget", "dnsutils", "git", "screen", "whois", "pwgen", "python3", "jq", 
        "fail2ban", "sudo", "gnutls-bin", "mlocate", "dh-make", "libaudit-dev", "build-essential", 
        "dos2unix", "debconf-utils"
    ];
    let _ = &packages;
    
    #[cfg(target_os = "linux")]
    {
        for pkg in &packages {
            let check = Command::new("dpkg-query").args(&["-W", "--showformat='${Status}\\n'", pkg]).output();
            if let Ok(out) = check {
                let status = String::from_utf8_lossy(&out.stdout);
                if !status.contains("install ok installed") {
                    let _ = Command::new("apt-get").args(&["-qq", "install", pkg, "-y"]).status();
                }
            }
        }

        println!("\x1B[1;33mInstalling Go...\x1B[0m");
        let _ = fs::remove_file("/usr/bin/go");
        let _ = Command::new("wget").args(&["-q", "https://go.dev/dl/go1.22.0.linux-amd64.tar.gz"]).status();
        let _ = Command::new("sudo").args(&["tar", "-C", "/usr/local", "-xzf", "go1.22.0.linux-amd64.tar.gz"]).status();
        let _ = fs::remove_file("go1.22.0.linux-amd64.tar.gz");
        
        println!("\x1B[1;33mBuilding dnstt-server from source...\x1B[0m");
        let _ = Command::new("rm").args(&["-rf", "/etc/slowdns", "/root/dnstt"]).status();
        let _ = Command::new("git").args(&["clone", "https://www.bamsoftware.com/git/dnstt.git", "/root/dnstt"]).status();
        
        let build_status = Command::new("go")
            .args(&["build"])
            .current_dir("/root/dnstt/dnstt-server")
            .env("PATH", "/usr/local/go/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
            .status();
            
        let _ = fs::create_dir_all("/etc/slowdns");
        if let Ok(status) = build_status {
            if status.success() {
                let _ = fs::rename("/root/dnstt/dnstt-server/dnstt-server", "/usr/sbin/dns-server");
            }
        }
        
        if !Path::new("/usr/sbin/dns-server").exists() {
            println!("\x1B[1;31mBuild failed, downloading prebuilt binary...\x1B[0m");
            let _ = Command::new("wget")
                .args(&["-q", "-O", "/usr/sbin/dns-server", "https://github.com/powermx/dnstt/raw/refs/heads/main/dns-server"])
                .status();
            let _ = Command::new("chmod").args(&["+x", "/usr/sbin/dns-server"]).status();
        }

        println!("\x1B[1;32mGenerating SlowDNS keys...\x1B[0m");
        let _ = Command::new("/usr/sbin/dns-server")
            .args(&["-gen-key", "-privkey-file", "/etc/slowdns/server.key", "-pubkey-file", "/etc/slowdns/server.pub"])
            .status();
            
        println!("\x1B[1;32mSlowDNS core installed successfully!\x1B[0m");
    }
    pause();
}

fn del_iptables_openvpn() {
    #[cfg(target_os = "linux")]
    {
        while Command::new("iptables").args(&["-C", "INPUT", "-p", "udp", "--dport", "25000", "-j", "ACCEPT"]).output().map(|o| o.status.success()).unwrap_or(false) {
            let _ = Command::new("iptables").args(&["-D", "INPUT", "-p", "udp", "--dport", "25000", "-j", "ACCEPT"]).status();
        }
        
        let interface = run_bash_cmd("ip route | grep default | awk '{print $5}'");
        if !interface.is_empty() {
            while Command::new("iptables").args(&["-t", "nat", "-C", "PREROUTING", "-i", &interface, "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "25000"]).output().map(|o| o.status.success()).unwrap_or(false) {
                let _ = Command::new("iptables").args(&["-t", "nat", "-D", "PREROUTING", "-i", &interface, "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "25000"]).status();
            }
        }
        
        while Command::new("iptables").args(&["-t", "nat", "-C", "PREROUTING", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "25000"]).output().map(|o| o.status.success()).unwrap_or(false) {
            let _ = Command::new("iptables").args(&["-t", "nat", "-D", "PREROUTING", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "25000"]).status();
        }
    }
}

fn del_iptables_ssh() {
    #[cfg(target_os = "linux")]
    {
        while Command::new("iptables").args(&["-C", "INPUT", "-p", "udp", "--dport", "5300", "-j", "ACCEPT"]).output().map(|o| o.status.success()).unwrap_or(false) {
            let _ = Command::new("iptables").args(&["-D", "INPUT", "-p", "udp", "--dport", "5300", "-j", "ACCEPT"]).status();
        }
        
        let interface = run_bash_cmd("ip route | grep default | awk '{print $5}'");
        if !interface.is_empty() {
            while Command::new("iptables").args(&["-t", "nat", "-C", "PREROUTING", "-i", &interface, "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5300"]).output().map(|o| o.status.success()).unwrap_or(false) {
                let _ = Command::new("iptables").args(&["-t", "nat", "-D", "PREROUTING", "-i", &interface, "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5300"]).status();
            }
        }
        
        while Command::new("iptables").args(&["-t", "nat", "-C", "PREROUTING", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5300"]).output().map(|o| o.status.success()).unwrap_or(false) {
            let _ = Command::new("iptables").args(&["-t", "nat", "-D", "PREROUTING", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5300"]).status();
        }
        
        let _ = Command::new("systemctl").args(&["stop", "dnstt"]).status();
        let _ = Command::new("systemctl").args(&["disable", "dnstt"]).status();
    }
}

fn add_iptables_ssh() {
    del_iptables_openvpn();
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("iptables").args(&["-I", "INPUT", "-p", "udp", "--dport", "5300", "-j", "ACCEPT"]).status();
        let interface = run_bash_cmd("ip route | grep default | awk '{print $5}'");
        if !interface.is_empty() {
            let _ = Command::new("iptables").args(&["-t", "nat", "-I", "PREROUTING", "-i", &interface, "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5300"]).status();
        } else {
            let _ = Command::new("iptables").args(&["-t", "nat", "-I", "PREROUTING", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "5300"]).status();
        }

        let ns = fs::read_to_string("/etc/slowdns/nameserver").unwrap_or_default().trim().to_string();
        
        let service_content = format!(
            "[Unit]\n\
             Description=SlowDNS rbstv Autoscript Service\n\
             After=network.target nss-lookup.target\n\n\
             [Service]\n\
             Type=simple\n\
             User=root\n\
             CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_BIND_SERVICE\n\
             AmbientCapabilities=CAP_NET_ADMIN CAP_NET_BIND_SERVICE\n\
             NoNewPrivileges=true\n\
             ExecStart=/usr/sbin/dns-server -udp :5300 -privkey-file /etc/slowdns/server.key {} 127.0.0.1:111\n\
             Restart=on-failure\n\n\
             [Install]\n\
             WantedBy=multi-user.target\n",
            ns
        );
        
        let _ = fs::write("/etc/systemd/system/dnstt.service", service_content);
        let _ = Command::new("systemctl").arg("daemon-reload").status();
        let _ = Command::new("systemctl").args(&["enable", "dnstt"]).status();
        let _ = Command::new("systemctl").args(&["start", "dnstt"]).status();
    }
}

fn add_iptables_openvpn() {
    del_iptables_ssh();
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("iptables").args(&["-I", "INPUT", "-p", "udp", "--dport", "25000", "-j", "ACCEPT"]).status();
        let interface = run_bash_cmd("ip route | grep default | awk '{print $5}'");
        if !interface.is_empty() {
            let _ = Command::new("iptables").args(&["-t", "nat", "-I", "PREROUTING", "-i", &interface, "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "25000"]).status();
        } else {
            let _ = Command::new("iptables").args(&["-t", "nat", "-I", "PREROUTING", "-p", "udp", "--dport", "53", "-j", "REDIRECT", "--to-ports", "25000"]).status();
        }
    }
}

fn ask_nameserver() {
    header();
    let mut ns = String::new();
    print!("Your Nameserver : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut ns);
    let ns = ns.trim().to_string();
    if !ns.is_empty() {
        let _ = fs::write("/etc/slowdns/nameserver", ns);
    }
}

fn change_nameserver() {
    header();
    let current_ns = fs::read_to_string("/etc/slowdns/nameserver").unwrap_or_default().trim().to_string();
    println!("\x1B[1;33mCurrent Nameserver:\x1B[0m \x1B[1;36m{}\x1B[0m", current_ns);
    let mut ns = String::new();
    print!("New Nameserver : ");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut ns);
    let ns = ns.trim().to_string();
    
    if !ns.is_empty() {
        let _ = fs::write("/etc/slowdns/nameserver", &ns);
        
        let svc_file = "/etc/systemd/system/dnstt.service";
        if Path::new(svc_file).exists() {
            let service_content = format!(
                "[Unit]\n\
                 Description=SlowDNS rbstv Autoscript Service\n\
                 After=network.target nss-lookup.target\n\n\
                 [Service]\n\
                 Type=simple\n\
                 User=root\n\
                 CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_BIND_SERVICE\n\
                 AmbientCapabilities=CAP_NET_ADMIN CAP_NET_BIND_SERVICE\n\
                 NoNewPrivileges=true\n\
                 ExecStart=/usr/sbin/dns-server -udp :5300 -privkey-file /etc/slowdns/server.key {} 127.0.0.1:111\n\
                 Restart=on-failure\n\n\
                 [Install]\n\
                 WantedBy=multi-user.target\n",
                ns
            );
            let _ = fs::write(svc_file, service_content);
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("systemctl").arg("daemon-reload").status();
                let _ = Command::new("systemctl").args(&["restart", "dnstt"]).status();
            }
        }
        println!("\x1B[1;32mNameserver updated successfully!\x1B[0m");
    }
    pause();
}

pub fn run_slowdns() {
    if !Path::new("/usr/sbin/dns-server").exists() {
        get_core_dns();
        // Setup initial resolvs
        let _ = run_bash_cmd("echo 'nameserver 1.1.1.1' >> /etc/resolv.conf");
        let _ = fs::create_dir_all("/etc/slowdns");
        let _ = fs::write("/etc/slowdns/server.key", "79165a5f041150b665db82f16d33be2664749ea5dd0e90c62c1ff99de02a375d");
        let _ = fs::write("/etc/slowdns/server.pub", "5bb04eb5c1d8e8ced2feefd2a3b7e4d57cf648dce0d5a225ac62197729336f50");
        ask_nameserver();
    }

    loop {
        let iptables_rules = run_bash_cmd("iptables -t nat -L");
        let port_openvpn = iptables_rules.contains("25000");
        let port_slowdns = iptables_rules.contains("5300");

        header();
        if port_openvpn && port_slowdns {
            println!("\x1B[1;31mDetected both SlowDNS SSH and DNS OpenVPN rules active!\x1B[0m");
            println!("Cleaning up conflicting rules...");
            del_iptables_ssh();
            del_iptables_openvpn();
            pause();
            continue;
        }

        let mut choice = String::new();
        if !port_openvpn && !port_slowdns {
            println!("   1.)  Start SlowDNS SSH");
            println!("   2.)  Start DNS OpenVPN");
            println!("   3.)  Change Nameserver");
            println!("   x.)  Exit");
            draw_line();
            print!(" Select from options [1-3 or x] : ");
            let _ = io::stdout().flush();
            let _ = io::stdin().read_line(&mut choice);
            match choice.trim() {
                "1" => {
                    if fs::read_to_string("/etc/slowdns/nameserver").unwrap_or_default().trim().is_empty() {
                        ask_nameserver();
                    }
                    add_iptables_ssh();
                    header();
                    show_status("enabled");
                    show_dns_info();
                    footer();
                    pause();
                }
                "2" => {
                    add_iptables_openvpn();
                    header();
                    println!("\x1B[1;32m[✔] DNS OpenVPN is ENABLED\x1B[0m");
                    footer();
                    pause();
                }
                "3" => change_nameserver(),
                "x" | "X" => break,
                _ => {}
            }
        } else if port_slowdns && !port_openvpn {
            println!("   1.)  Stop SlowDNS SSH");
            println!("   2.)  Change SSH to OpenVPN");
            println!("   3.)  Change Nameserver");
            println!("   x.)  Exit");
            draw_line();
            print!(" Select from options [1-3 or x] : ");
            let _ = io::stdout().flush();
            let _ = io::stdin().read_line(&mut choice);
            match choice.trim() {
                "1" => {
                    del_iptables_ssh();
                    header();
                    show_status("disabled");
                    footer();
                    pause();
                }
                "2" => {
                    del_iptables_ssh();
                    add_iptables_openvpn();
                    header();
                    println!("\x1B[1;32m[✔] DNS OpenVPN is ENABLED\x1B[0m");
                    footer();
                    pause();
                }
                "3" => change_nameserver(),
                "x" | "X" => break,
                _ => {}
            }
        } else if port_openvpn && !port_slowdns {
            println!("   1.)  Stop DNS OpenVPN");
            println!("   2.)  Change OpenVPN to SSH");
            println!("   3.)  Change Nameserver");
            println!("   x.)  Exit");
            draw_line();
            print!(" Select from options [1-3 or x] : ");
            let _ = io::stdout().flush();
            let _ = io::stdin().read_line(&mut choice);
            match choice.trim() {
                "1" => {
                    del_iptables_openvpn();
                    header();
                    println!("\x1B[1;31m[✘] DNS OpenVPN is DISABLED\x1B[0m");
                    footer();
                    pause();
                }
                "2" => {
                    del_iptables_openvpn();
                    if fs::read_to_string("/etc/slowdns/nameserver").unwrap_or_default().trim().is_empty() {
                        ask_nameserver();
                    }
                    add_iptables_ssh();
                    header();
                    show_status("enabled");
                    show_dns_info();
                    footer();
                    pause();
                }
                "3" => change_nameserver(),
                "x" | "X" => break,
                _ => {}
            }
        }
    }
}
