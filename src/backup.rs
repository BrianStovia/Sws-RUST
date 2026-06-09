use std::io::{self, Write};
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::utils::{get_domain, get_ip, send_telegram_notification, run_bash_cmd};

pub fn run_backup() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Memulai Backup...");
    
    let tanggal = run_bash_cmd("date +%m-%d-%Y");
    let waktu = run_bash_cmd("date +%H-%M-%S");
    let random_code = run_bash_cmd("openssl rand -hex 4");
    
    let zip_file = format!("/root/Backup-{}-{}-{}.zip", random_code, tanggal, waktu);
    let backup_dir = "/root/backup";
    
    let _ = fs::remove_dir_all(backup_dir);
    let _ = fs::create_dir_all(backup_dir);
    
    println!("Copying files...");
    let _ = run_bash_cmd(&format!("cp /etc/passwd {}/", backup_dir));
    let _ = run_bash_cmd(&format!("cp /etc/group {}/", backup_dir));
    let _ = run_bash_cmd(&format!("cp /etc/shadow {}/", backup_dir));
    let _ = run_bash_cmd(&format!("cp /etc/gshadow {}/", backup_dir));
    
    let _ = fs::create_dir_all(format!("{}/v2ray", backup_dir));
    let _ = run_bash_cmd(&format!("cp -r /usr/local/etc/v2ray/* {}/v2ray/", backup_dir));
    
    let _ = fs::create_dir_all(format!("{}/noobzvpns", backup_dir));
    let _ = run_bash_cmd(&format!("cp /etc/noobzvpns/db_user.json {}/noobzvpns/", backup_dir));
    let _ = run_bash_cmd(&format!("cp /etc/noobzvpns/.noobz.db {}/noobzvpns/", backup_dir));
    
    println!("Creating encrypted zip...");
    let zip_status = Command::new("zip")
        .args(&["-rP", "Rerechan02", &zip_file, "backup"])
        .current_dir("/root")
        .status();
        
    let _ = fs::remove_dir_all(backup_dir);

    match zip_status {
        Ok(s) if s.success() => {}
        _ => {
            println!("Gagal membuat file ZIP.");
            return;
        }
    }

    println!("Uploading to Google Drive via rclone...");
    let _ = Command::new("rclone").args(&["copy", &zip_file, "rerechan:backup/"]).status();
    
    let file_name = Path::new(&zip_file).file_name().unwrap().to_string_lossy().to_string();
    let url = run_bash_cmd(&format!("rclone link rerechan:backup/{}", file_name));
    
    if url.is_empty() {
        println!("Gagal mendapatkan link backup.");
        return;
    }
    
    // Extract ID from url (e.g., id=xxx)
    let backup_id = if let Some(id_idx) = url.find("id=") {
        let after = &url[id_idx + 3..];
        if let Some(amp_idx) = after.find('&') {
            after[..amp_idx].to_string()
        } else {
            after.to_string()
        }
    } else {
        url.clone()
    };
    
    let download_link = format!("https://drive.google.com/u/4/uc?id={}&export=download", backup_id);
    let domain = get_domain();
    let ip = get_ip();
    
    let isp = run_bash_cmd(&format!("curl -s --max-time 5 http://ip-api.com/json/{}", ip));
    let mut isp_org = String::new();
    if let Some(org_idx) = isp.find("\"org\":\"") {
        let after = &isp[org_idx + 7..];
        if let Some(end_idx) = after.find('"') {
            isp_org = after[..end_idx].to_string();
        }
    }
    if isp_org.is_empty() {
        isp_org = "Unknown ISP".to_string();
    }
    
    let message = format!(
        "✅ <b>Backup Berhasil!</b>\n\n\
         🆔 <b>ID Backup:</b>\n<pre>{}</pre>\n\n\
         🖥 <b>Informasi Server:</b>\n\
         📛 <b>Nama Pengguna:</b> Rerechan02\n\
         🌐 <b>Domain:</b> {}\n\
         🏢 <b>ISP:</b> {}\n\
         🌍 <b>IP VPS:</b> <pre>{}</pre>\n\
         ⏰ <b>Waktu Backup:</b> {} pukul {}\n\n\
         📥 <b>Unduh Backup:</b>\n<pre>{}</pre>\n\n\
         🔹 <i>File backup telah tersimpan di Google Drive.</i>\n\
         🔹 <i>Simpan ID backup ini untuk referensi pemulihan di masa mendatang.</i>\n\n\
         🔒 <b>Keamanan:</b> File backup dilindungi dengan password.\n\
         🔄 Proses backup ini dilakukan secara otomatis untuk menjaga data Anda tetap aman.\n\n\
         ✨ <b>Terima kasih telah menggunakan layanan kami!</b>",
        backup_id, domain, isp_org, ip, tanggal, waktu, download_link
    );
    
    send_telegram_notification(&message);
    
    print!("\x1B[2J\x1B[1;1H");
    println!("Backup dan pengiriman pesan ke Telegram selesai.");
    println!("Detail Backup:");
    println!("🆔 ID Backup: {}", backup_id);
    println!("🖥 Informasi Server:");
    println!("┣ 📛 Nama Pengguna: Rerechan02");
    println!("┣ 🌐 Domain: {}", domain);
    println!("┣ 🏢 ISP: {}", isp_org);
    println!("┣ 🌍 IP VPS: {}", ip);
    println!("┣ ⏰ Waktu Backup: {} pukul {}", tanggal, waktu);
    println!("┗━━━━━━━━━━━━━━━━━");
    println!("📥 Unduh Backup: {}", download_link);
    println!("🔒 Keamanan: File backup dilindungi dengan password.");
    
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

pub fn run_restore() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Memulai proses restore...");
    
    // Find zip file in /root
    let mut backup_file = String::new();
    if let Ok(entries) = fs::read_dir("/root") {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "zip" {
                    backup_file = entry.path().to_string_lossy().to_string();
                    break;
                }
            }
        }
    }
    
    if !backup_file.is_empty() {
        println!("File backup ditemukan: {}", backup_file);
    } else {
        // Prompt for ID or URL
        let mut input = String::new();
        print!("Tidak ditemukan file backup! Masukkan ID Backup atau URL: ");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut input);
        let input = input.trim().to_string();
        
        if input.is_empty() {
            println!("Tidak ada input ID Backup atau URL! Proses restore dibatalkan.");
            return;
        }
        
        let backup_url = if input.starts_with("http") {
            input
        } else {
            format!("https://drive.google.com/uc?id={}&export=download", input)
        };
        let _ = &backup_url;
        
        // Ensure gdown is installed
        #[cfg(target_os = "linux")]
        {
            if !Command::new("gdown").arg("--version").status().map(|s| s.success()).unwrap_or(false) {
                println!("Installing Python3 pip and gdown...");
                let _ = Command::new("apt-get").arg("update").status();
                let _ = Command::new("apt-get").args(&["install", "python3-pip", "-y"]).status();
                let _ = Command::new("pip3").args(&["install", "--no-cache-dir", "gdown"]).status();
            }
            
            println!("Downloading backup file using gdown...");
            let download_status = Command::new("gdown")
                .args(&["--fuzzy", "-O", "/root/rebackup.zip", &backup_url])
                .status();
                
            match download_status {
                Ok(s) if s.success() => {
                    backup_file = "/root/rebackup.zip".to_string();
                }
                _ => {
                    println!("Gagal mengunduh file backup!");
                    return;
                }
            }
        }
    }
    
    println!("Extracting backup file...");
    let extract_status = Command::new("unzip")
        .args(&["-P", "Rerechan02", "-o", &backup_file, "-d", "/root"])
        .status();
        
    match extract_status {
        Ok(s) if s.success() => {}
        _ => {
            println!("Gagal mengekstrak file backup!");
            return;
        }
    }
    
    println!("Restoring database configurations...");
    let _ = run_bash_cmd("cp -r /root/backup/v2ray/* /usr/local/etc/v2ray/ 2>/dev/null");
    let _ = run_bash_cmd("cp -r /root/backup/noobzvpns/* /etc/noobzvpns/ 2>/dev/null");
    let _ = run_bash_cmd("cp /root/backup/passwd /etc/ 2>/dev/null");
    let _ = run_bash_cmd("cp /root/backup/group /etc/ 2>/dev/null");
    let _ = run_bash_cmd("cp /root/backup/shadow /etc/ 2>/dev/null");
    let _ = run_bash_cmd("cp /root/backup/gshadow /etc/ 2>/dev/null");
    
    println!("Cleaning up temporary files...");
    let _ = fs::remove_dir_all("/root/backup");
    if backup_file == "/root/rebackup.zip" {
        let _ = fs::remove_file(&backup_file);
    }
    
    #[cfg(target_os = "linux")]
    {
        println!("Restarting services...");
        let _ = Command::new("systemctl").arg("daemon-reload").status();
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
        let _ = Command::new("systemctl").args(&["restart", "noobzvpns"]).status();
        let _ = Command::new("systemctl").args(&["restart", "ssh"]).status();
        let _ = Command::new("systemctl").args(&["restart", "sshd"]).status();
    }
    
    println!("Proses restore selesai dan semua file sementara telah dihapus.");
    println!("\nPress Enter to return to menu...");
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}
