use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

#[derive(Debug)]
struct TgUpdate {
    update_id: i64,
    chat_id: String,
    text: Option<String>,
    callback_id: Option<String>,
    callback_data: Option<String>,
}

fn get_bot_paths() -> (&'static str, &'static str, &'static str) {
    let key = "/usr/local/etc/v2ray/bot.key";
    let cid = "/usr/local/etc/v2ray/client.id";
    let sellers = "/etc/telegram_sellers.json";
    
    let key_path = if Path::new(key).exists() { key } else { "bot.key" };
    let cid_path = if Path::new(cid).exists() { cid } else { "client.id" };
    let sellers_path = if Path::new("/etc").exists() { sellers } else { "telegram_sellers.json" };
    
    (key_path, cid_path, sellers_path)
}

fn load_config(key_path: &str, cid_path: &str) -> (String, String) {
    let mut token = String::new();
    let mut owner_id = String::new();
    
    if let Ok(content) = fs::read_to_string(key_path) {
        token = content.trim().to_string();
    }
    if let Ok(content) = fs::read_to_string(cid_path) {
        owner_id = content.trim().to_string();
    }
    (token, owner_id)
}

fn parse_sellers_json(content: &str) -> Vec<String> {
    let mut sellers = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();
    let mut escaped = false;
    for c in content.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            if in_quotes {
                sellers.push(current.clone());
                current.clear();
                in_quotes = false;
            } else {
                in_quotes = true;
            }
        } else if in_quotes {
            current.push(c);
        }
    }
    sellers
}

fn load_sellers(path: &str) -> Vec<String> {
    if Path::new(path).exists() {
        if let Ok(content) = fs::read_to_string(path) {
            return parse_sellers_json(&content);
        }
    }
    Vec::new()
}

fn save_sellers(path: &str, sellers: &[String]) {
    let elements: Vec<String> = sellers.iter().map(|s| format!("\"{}\"", s)).collect();
    let json = format!("[\n  {}\n]", elements.join(",\n  "));
    let _ = fs::write(path, json);
}

#[allow(dead_code)]
fn clean_ansi(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1B' || c == '\u{001B}' {
            if chars.peek() == Some(&'[') {
                let _ = chars.next();
                while let Some(&nc) = chars.peek() {
                    let _ = chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn urlencode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char);
            }
            b' ' => {
                encoded.push('+');
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

fn telegram_api_call(token: &str, method: &str, payload: &str, is_json: bool) -> Option<String> {
    let url = format!("https://api.telegram.org/bot{}/{}", token, method);
    let mut cmd = Command::new("curl");
    cmd.arg("-s")
       .arg("-X").arg("POST")
       .arg("--connect-timeout").arg("10")
       .arg("-m").arg("35")
       .arg(url);
    
    if is_json {
        cmd.arg("-H").arg("Content-Type: application/json")
           .arg("-d").arg(payload);
    } else {
        cmd.arg("-H").arg("Content-Type: application/x-www-form-urlencoded")
           .arg("-d").arg(payload);
    }
    
    let out = cmd.output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

fn send_message(token: &str, chat_id: &str, text: &str, parse_mode: &str, reply_markup: Option<&str>) {
    let mut params = format!(
        "chat_id={}&text={}&parse_mode={}",
        urlencode(chat_id),
        urlencode(text),
        urlencode(parse_mode)
    );
    if let Some(markup) = reply_markup {
        params.push_str(&format!("&reply_markup={}", urlencode(markup)));
    }
    
    if telegram_api_call(token, "sendMessage", &params, false).is_none() {
        println!("Failed to send message to {}", chat_id);
    }
}

fn answer_callback_query(token: &str, callback_query_id: &str) {
    let params = format!("callback_query_id={}", urlencode(callback_query_id));
    if telegram_api_call(token, "answerCallbackQuery", &params, false).is_none() {
        println!("Failed to answer callback query: {}", callback_query_id);
    }
}

fn set_bot_commands(token: &str) {
    let payload = r#"{
        "commands": [
            {"command": "menu", "description": "Tampilkan Menu Utama"},
            {"command": "status", "description": "Cek Status VPS"},
            {"command": "listssh", "description": "Daftar Akun SSH"},
            {"command": "listnoobz", "description": "Daftar Akun NoobzVPN"},
            {"command": "help", "description": "Bantuan & Format Perintah"}
        ]
    }"#;
    if let Some(res) = telegram_api_call(token, "setMyCommands", payload, true) {
        println!("Bot commands menu registered successfully: {}", res);
    } else {
        println!("Failed to set bot commands");
    }
}

fn send_menu(token: &str, chat_id: &str) {
    let menu_text = "\
        🤖 <b>VPN SELLER BOT MENU</b> 🤖\n\
        ───────────────────────\n\
        Silakan pilih opsi menu di bawah ini:";
    
    let keyboard = r#"{
        "inline_keyboard": [
            [
                {"text": "➕ Create SSH", "callback_data": "/addssh"},
                {"text": "➕ Create Vmess", "callback_data": "/addvmess"}
            ],
            [
                {"text": "➕ Create Vless", "callback_data": "/addvless"},
                {"text": "➕ Create Trojan", "callback_data": "/addtrojan"}
            ],
            [
                {"text": "➕ Create Noobz", "callback_data": "/addnoobz"},
                {"text": "📊 VPS Status", "callback_data": "/status"}
            ],
            [
                {"text": "👥 List SSH", "callback_data": "/listssh"},
                {"text": "👥 List Noobz", "callback_data": "/listnoobz"}
            ]
        ]
    }"#;
    
    send_message(token, chat_id, menu_text, "HTML", Some(keyboard));
}

fn run_bash_cmd(cmd_str: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        match Command::new("bash").arg("-c").arg(cmd_str).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                clean_ansi(&combined).trim().to_string()
            }
            Err(e) => format!("Error running bash command: {}", e),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        format!("(Mock command output for: {})", cmd_str)
    }
}

fn get_service_status(service: &str) -> &'static str {
    #[cfg(target_os = "linux")]
    {
        match Command::new("systemctl").args(&["is-active", service]).output() {
            Ok(output) => {
                let status = String::from_utf8_lossy(&output.stdout);
                if status.trim() == "active" {
                    "✅ ON"
                } else {
                    "❌ OFF"
                }
            }
            Err(_) => "❌ OFF",
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = service;
        "✅ ON (Mock)"
    }
}

fn extract_json_int(json: &str, key: &str) -> Option<i64> {
    if let Some(key_idx) = json.find(key) {
        let after = &json[key_idx + key.len()..];
        if let Some(colon_idx) = after.find(':') {
            let val_part = after[colon_idx + 1..].trim_start();
            let len = val_part.chars().take_while(|c| c.is_ascii_digit() || *c == '-').count();
            if len > 0 {
                return val_part[..len].parse::<i64>().ok();
            }
        }
    }
    None
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    if let Some(key_idx) = json.find(key) {
        let after = &json[key_idx + key.len()..];
        if let Some(colon_idx) = after.find(':') {
            let val_part = after[colon_idx + 1..].trim_start();
            if val_part.starts_with('"') {
                let mut current = String::new();
                let mut escaped = false;
                for c in val_part[1..].chars() {
                    if escaped {
                        current.push(c);
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        return Some(current);
                    } else {
                        current.push(c);
                    }
                }
            }
        }
    }
    None
}

fn extract_chat_id(update_block: &str) -> Option<String> {
    if let Some(chat_idx) = update_block.find("\"chat\"") {
        let after_chat = &update_block[chat_idx..];
        if let Some(id_idx) = after_chat.find("\"id\"") {
            let after_id = &after_chat[id_idx + 4..];
            if let Some(colon_idx) = after_id.find(':') {
                let val_part = after_id[colon_idx + 1..].trim_start();
                let len = val_part.chars().take_while(|c| c.is_ascii_digit() || *c == '-').count();
                if len > 0 {
                    return Some(val_part[..len].to_string());
                }
            }
        }
    }
    None
}

fn extract_callback_id(update_block: &str) -> Option<String> {
    if let Some(cb_idx) = update_block.find("\"callback_query\"") {
        let after_cb = &update_block[cb_idx..];
        return extract_json_string(after_cb, "\"id\"");
    }
    None
}

fn extract_callback_data(update_block: &str) -> Option<String> {
    if let Some(cb_idx) = update_block.find("\"callback_query\"") {
        let after_cb = &update_block[cb_idx..];
        return extract_json_string(after_cb, "\"data\"");
    }
    None
}

fn extract_message_text(update_block: &str) -> Option<String> {
    if let Some(msg_idx) = update_block.find("\"message\"") {
        return extract_json_string(&update_block[msg_idx..], "\"text\"");
    }
    None
}

fn parse_updates(json: &str) -> Vec<TgUpdate> {
    let mut updates = Vec::new();
    if let Some(result_idx) = json.find("\"result\"") {
        let mut search_str = &json[result_idx..];
        let mut indices = Vec::new();
        let mut offset = 0;
        
        while let Some(idx) = search_str.find("\"update_id\"") {
            let before = &search_str[..idx];
            if let Some(brace_idx) = before.rfind('{') {
                indices.push(offset + brace_idx);
            }
            let next_start = idx + 11;
            search_str = &search_str[next_start..];
            offset += next_start;
        }
        
        for i in 0..indices.len() {
            let start = indices[i];
            let end = if i + 1 < indices.len() {
                indices[i + 1]
            } else {
                json.len()
            };
            let block = &json[start..end];
            
            if let Some(update_id) = extract_json_int(block, "\"update_id\"") {
                if let Some(chat_id) = extract_chat_id(block) {
                    let text = extract_message_text(block);
                    let callback_id = extract_callback_id(block);
                    let callback_data = extract_callback_data(block);
                    
                    updates.push(TgUpdate {
                        update_id,
                        chat_id,
                        text,
                        callback_id,
                        callback_data,
                    });
                }
            }
        }
    }
    updates
}

fn handle_command(
    token: &str,
    owner_id: &str,
    sellers_path: &str,
    chat_id: &str,
    text: &str,
    is_auth: bool,
) {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    
    let cmd = parts[0].to_lowercase();
    
    if cmd == "/start" || cmd == "/menu" {
        if !is_auth {
            send_message(token, chat_id, "❌ <b>Akses Ditolak:</b> ID Anda belum terdaftar sebagai Seller.", "HTML", None);
            return;
        }
        send_menu(token, chat_id);
        return;
    }
    
    if cmd == "/help" {
        if !is_auth {
            send_message(token, chat_id, "❌ <b>Akses Ditolak:</b> ID Anda belum terdaftar sebagai Seller.", "HTML", None);
            return;
        }
        let mut help_text = "\
            🤖 <b>VPN Seller Telegram Bot Help</b> 🤖\n\
            ───────────────────────\n\
            <b>Daftar Perintah Pembuatan Akun:</b>\n\
            • <code>/addssh &lt;user&gt; &lt;pass&gt; &lt;days&gt;</code>\n\
            • <code>/addvmess &lt;user&gt; &lt;days&gt;</code>\n\
            • <code>/addvless &lt;user&gt; &lt;days&gt;</code>\n\
            • <code>/addtrojan &lt;user&gt; &lt;days&gt;</code>\n\
            • <code>/addnoobz &lt;user&gt; &lt;limit_device&gt; &lt;bandwidth_gb&gt; &lt;days&gt;</code>\n\
            ───────────────────────\n\
            <b>Daftar Perintah Manajemen & Status:</b>\n\
            • <code>/status</code> - Cek status resource & service VPS\n\
            • <code>/listssh</code> - Daftar akun SSH aktif\n\
            • <code>/listnoobz</code> - Daftar akun NoobzVPN\n\
            ───────────────────────\n".to_string();
            
        if chat_id == owner_id {
            help_text.push_str("\
                <b>Perintah Owner Only:</b>\n\
                • <code>/addseller &lt;chat_id&gt;</code> - Tambah seller baru\n\
                • <code>/delseller &lt;chat_id&gt;</code> - Hapus seller\n\
                • <code>/listsellers</code> - Daftar semua seller\n\
                ───────────────────────\n");
        }
        send_message(token, chat_id, &help_text, "HTML", None);
        return;
    }
    
    if !is_auth {
        send_message(token, chat_id, "❌ <b>Akses Ditolak:</b> ID Anda belum terdaftar sebagai Seller.", "HTML", None);
        return;
    }
    
    match cmd.as_str() {
        "/addssh" => {
            if parts.len() < 4 {
                send_message(token, chat_id, "💡 Format: <code>/addssh &lt;user&gt; &lt;pass&gt; &lt;days&gt;</code>", "HTML", None);
                return;
            }
            let user = parts[1];
            let password = parts[2];
            let days = parts[3];
            send_message(token, chat_id, &format!("⏳ Sedang membuat akun SSH untuk <code>{}</code>...", user), "HTML", None);
            let cmd_str = format!("printf \"{}\\n{}\\n{}\\n\" | bash /usr/local/sbin/add-ssh", user, password, days);
            let out = run_bash_cmd(&cmd_str);
            send_message(token, chat_id, &format!("<pre>{}</pre>", out), "HTML", None);
        }
        "/addvmess" => {
            if parts.len() < 3 {
                send_message(token, chat_id, "💡 Format: <code>/addvmess &lt;user&gt; &lt;days&gt;</code>", "HTML", None);
                return;
            }
            let user = parts[1];
            let days = parts[2];
            send_message(token, chat_id, &format!("⏳ Sedang membuat akun Vmess untuk <code>{}</code>...", user), "HTML", None);
            let cmd_str = format!("printf \"{}\\n{}\\n\\n\" | bash /usr/local/sbin/add-vmess", user, days);
            let out = run_bash_cmd(&cmd_str);
            send_message(token, chat_id, &format!("<pre>{}</pre>", out), "HTML", None);
        }
        "/addvless" => {
            if parts.len() < 3 {
                send_message(token, chat_id, "💡 Format: <code>/addvless &lt;user&gt; &lt;days&gt;</code>", "HTML", None);
                return;
            }
            let user = parts[1];
            let days = parts[2];
            send_message(token, chat_id, &format!("⏳ Sedang membuat akun Vless untuk <code>{}</code>...", user), "HTML", None);
            let cmd_str = format!("printf \"{}\\n{}\\n\\n\" | bash /usr/local/sbin/add-vless", user, days);
            let out = run_bash_cmd(&cmd_str);
            send_message(token, chat_id, &format!("<pre>{}</pre>", out), "HTML", None);
        }
        "/addtrojan" => {
            if parts.len() < 3 {
                send_message(token, chat_id, "💡 Format: <code>/addtrojan &lt;user&gt; &lt;days&gt;</code>", "HTML", None);
                return;
            }
            let user = parts[1];
            let days = parts[2];
            send_message(token, chat_id, &format!("⏳ Sedang membuat akun Trojan untuk <code>{}</code>...", user), "HTML", None);
            let cmd_str = format!("printf \"{}\\n{}\\n\\n\" | bash /usr/local/sbin/add-tr", user, days);
            let out = run_bash_cmd(&cmd_str);
            send_message(token, chat_id, &format!("<pre>{}</pre>", out), "HTML", None);
        }
        "/addnoobz" => {
            if parts.len() < 5 {
                send_message(token, chat_id, "💡 Format: <code>/addnoobz &lt;user&gt; &lt;limit_device&gt; &lt;bandwidth_gb&gt; &lt;days&gt;</code>", "HTML", None);
                return;
            }
            let user = parts[1];
            let device = parts[2];
            let bw = parts[3];
            let days = parts[4];
            send_message(token, chat_id, &format!("⏳ Sedang membuat akun NoobzVPN untuk <code>{}</code>...", user), "HTML", None);
            let cmd_str = format!("printf \"{}\\n{}\\n{}\\n{}\\n\" | bash /usr/local/sbin/add-noobz", user, device, bw, days);
            let out = run_bash_cmd(&cmd_str);
            send_message(token, chat_id, &format!("<pre>{}</pre>", out), "HTML", None);
        }
        "/status" => {
            send_message(token, chat_id, "⏳ Mengambil status VPS...", "HTML", None);
            let v2ray_st = get_service_status("v2ray");
            let nginx_st = get_service_status("nginx");
            let sslh_st = get_service_status("sslh");
            let noobz_st = get_service_status("noobzvpns");
            let udp_st = get_service_status("udp-custom");
            let proxy_st = get_service_status("proxy");
            
            let mem_out = run_bash_cmd("free -h | awk 'NR==2 {print $3 \" / \" $2}'");
            let cpu_out = run_bash_cmd("top -bn1 | grep 'Cpu(s)' | awk '{print $2}'");
            let disk_out = run_bash_cmd("df -h / | awk 'NR==2 {print $3 \" / \" $2 \" ( \" $5 \" )\"}'");
            let uptime_out = run_bash_cmd("uptime -p");
            
            let status_text = format!(
                "⚙️ <b>VPS STATUS INFO</b> ⚙️\n\
                 ───────────────────────\n\
                 <b>CPU Usage  :</b> <code>{}%</code>\n\
                 <b>Memory     :</b> <code>{}</code>\n\
                 <b>Disk Space :</b> <code>{}</code>\n\
                 <b>Uptime     :</b> <code>{}</code>\n\
                 ───────────────────────\n\
                 <b>SERVICES STATUS:</b>\n\
                 • <b>V2Ray Core  :</b> {}\n\
                 • <b>Nginx Server :</b> {}\n\
                 • <b>SSLH Proxy   :</b> {}\n\
                 • <b>NoobzVPN     :</b> {}\n\
                 • <b>UDP Custom   :</b> {}\n\
                 • <b>Proxy SSHWS  :</b> {}\n\
                 ───────────────────────\n",
                cpu_out, mem_out, disk_out, uptime_out,
                v2ray_st, nginx_st, sslh_st, noobz_st, udp_st, proxy_st
            );
            send_message(token, chat_id, &status_text, "HTML", None);
        }
        "/listssh" => {
            send_message(token, chat_id, "⏳ Mengambil daftar akun SSH...", "HTML", None);
            let out = run_bash_cmd("awk -F: '$3 >= 1000 && $1 != \"nobody\" {print $1}' /etc/passwd");
            let display_out = if out.is_empty() { "Tidak ada akun".to_string() } else { out };
            send_message(token, chat_id, &format!("👥 <b>Daftar Akun SSH Aktif:</b>\n<pre>{}</pre>", display_out), "HTML", None);
        }
        "/listnoobz" => {
            send_message(token, chat_id, "⏳ Mengambil daftar akun NoobzVPN...", "HTML", None);
            let out = run_bash_cmd("noobzvpns print-all | grep -E '^[0-9]+\\.'");
            let display_out = if out.is_empty() { "Tidak ada akun".to_string() } else { out };
            send_message(token, chat_id, &format!("👥 <b>Daftar Akun NoobzVPN Aktif:</b>\n<pre>{}</pre>", display_out), "HTML", None);
        }
        "/addseller" => {
            if chat_id != owner_id {
                send_message(token, chat_id, "❌ Perintah ini hanya khusus untuk Owner!", "HTML", None);
                return;
            }
            if parts.len() < 2 {
                send_message(token, chat_id, "💡 Format: <code>/addseller &lt;chat_id&gt;</code>", "HTML", None);
                return;
            }
            let new_seller = parts[1].to_string();
            let mut sellers_list = load_sellers(sellers_path);
            if !sellers_list.contains(&new_seller) {
                sellers_list.push(new_seller.clone());
                save_sellers(sellers_path, &sellers_list);
                send_message(token, chat_id, &format!("✅ Sukses menambahkan <code>{}</code> sebagai Seller baru.", new_seller), "HTML", None);
            } else {
                send_message(token, chat_id, &format!("ℹ️ ID <code>{}</code> sudah terdaftar.", new_seller), "HTML", None);
            }
        }
        "/delseller" => {
            if chat_id != owner_id {
                send_message(token, chat_id, "❌ Perintah ini hanya khusus untuk Owner!", "HTML", None);
                return;
            }
            if parts.len() < 2 {
                send_message(token, chat_id, "💡 Format: <code>/delseller &lt;chat_id&gt;</code>", "HTML", None);
                return;
            }
            let del_seller = parts[1];
            let mut sellers_list = load_sellers(sellers_path);
            if sellers_list.contains(&del_seller.to_string()) {
                sellers_list.retain(|s| s != del_seller);
                save_sellers(sellers_path, &sellers_list);
                send_message(token, chat_id, &format!("✅ Sukses menghapus <code>{}</code> dari daftar Seller.", del_seller), "HTML", None);
            } else {
                send_message(token, chat_id, &format!("ℹ️ ID <code>{}</code> tidak ditemukan.", del_seller), "HTML", None);
            }
        }
        "/listsellers" => {
            if chat_id != owner_id {
                send_message(token, chat_id, "❌ Perintah ini hanya khusus untuk Owner!", "HTML", None);
                return;
            }
            let sellers_list = load_sellers(sellers_path);
            let out_str = if sellers_list.is_empty() {
                "Tidak ada seller lain".to_string()
            } else {
                sellers_list.iter().map(|s| format!("• <code>{}</code>", s)).collect::<Vec<String>>().join("\n")
            };
            send_message(token, chat_id, &format!("👤 <b>Daftar Seller Terdaftar:</b>\n{}", out_str), "HTML", None);
        }
        _ => {}
    }
}

fn get_updates(token: &str, offset: i64) -> Option<String> {
    let params = format!("offset={}&timeout=30", offset);
    telegram_api_call(token, "getUpdates", &params, false)
}

fn main() {
    let (key_path, cid_path, sellers_path) = get_bot_paths();
    let (token, owner_id) = load_config(key_path, cid_path);
    
    if token.is_empty() {
        eprintln!("Error: Bot Token not found in {}", key_path);
        std::process::exit(1);
    }
    
    println!("Telegram Bot daemon started successfully!");
    set_bot_commands(&token);
    
    let mut offset: i64 = 0;
    loop {
        match get_updates(&token, offset) {
            Some(res) => {
                let updates = parse_updates(&res);
                let mut max_id = offset;
                for update in &updates {
                    if update.update_id >= max_id {
                        max_id = update.update_id + 1;
                    }
                    let sellers = load_sellers(sellers_path);
                    let is_auth = update.chat_id == owner_id || sellers.contains(&update.chat_id);
                    
                    if let Some(ref text) = update.text {
                        handle_command(&token, &owner_id, sellers_path, &update.chat_id, text, is_auth);
                    } else if let Some(ref callback_data) = update.callback_data {
                        if let Some(ref cb_id) = update.callback_id {
                            answer_callback_query(&token, cb_id);
                        }
                        handle_command(&token, &owner_id, sellers_path, &update.chat_id, callback_data, is_auth);
                    }
                }
                offset = max_id;
            }
            None => {
                println!("Loop error fetching updates. Sleeping for 5s...");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
