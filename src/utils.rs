use std::fs;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Command;
use std::time::SystemTime;

// Pure Rust Base64 Encoder
pub fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut chunks = data.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        result.push(CHARSET[(n & 63) as usize] as char);
    }
    let remainder = chunks.remainder();
    if remainder.len() == 1 {
        let n = (remainder[0] as u32) << 16;
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push('=');
        result.push('=');
    } else if remainder.len() == 2 {
        let n = ((remainder[0] as u32) << 16) | ((remainder[1] as u32) << 8);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        result.push('=');
    }
    result
}

// Get domain configured in /usr/local/etc/v2ray/domain
pub fn get_domain() -> String {
    let path = "/usr/local/etc/v2ray/domain";
    if Path::new(path).exists() {
        if let Ok(content) = fs::read_to_string(path) {
            return content.trim().to_string();
        }
    }
    "not.configured.com".to_string()
}

// Get public IP
pub fn get_ip() -> String {
    let path = "/root/.ip";
    if Path::new(path).exists() {
        if let Ok(content) = fs::read_to_string(path) {
            return content.trim().to_string();
        }
    }
    // Fallback on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("hostname").arg("-I").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_ip) = stdout.split_whitespace().next() {
                return first_ip.to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

// Get telegram configuration
pub fn get_telegram_keys() -> (String, String) {
    let token_path = "/usr/local/etc/v2ray/bot.key";
    let chat_path = "/usr/local/etc/v2ray/client.id";
    
    let token = fs::read_to_string(token_path).unwrap_or_default().trim().to_string();
    let chat_id = fs::read_to_string(chat_path).unwrap_or_default().trim().to_string();
    
    (token, chat_id)
}

// Send telegram alert notification via curl process
pub fn send_telegram_notification(text: &str) {
    let (token, chat_id) = get_telegram_keys();
    if token.is_empty() || chat_id.is_empty() {
        return;
    }
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("curl")
            .args(&[
                "-s",
                "-X", "POST",
                &format!("https://api.telegram.org/bot{}/sendMessage", token),
                "-d", &format!("chat_id={}", chat_id),
                "-d", "parse_mode=HTML",
                "--data-urlencode", &format!("text={}", text)
            ])
            .output();
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("Mock Telegram sending text:\n{}", text);
    }
}

// Helper to run bash commands on Linux or print Mock on Windows
pub fn run_bash_cmd(cmd_str: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        match Command::new("bash").args(&["-c", cmd_str]).output() {
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

// Clean ANSI colors/codes
#[allow(dead_code)]
pub fn clean_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut in_escape = false;
    let mut in_bracket = false;
    for c in input.chars() {
        if c == '\x1B' {
            in_escape = true;
        } else if in_escape && c == '[' {
            in_bracket = true;
            in_escape = false;
        } else if in_bracket {
            if c.is_ascii_alphabetic() {
                in_bracket = false;
            }
        } else {
            output.push(c);
        }
    }
    output
}

// UUID generator
pub fn generate_uuid() -> String {
    let mut bytes = [0u8; 16];
    
    #[allow(unused_mut)]
    let mut read_success = false;
    #[cfg(target_os = "linux")]
    {
        use std::io::Read;
        if let Ok(mut f) = fs::File::open("/dev/urandom") {
            if f.read_exact(&mut bytes).is_ok() {
                read_success = true;
            }
        }
    }
    
    if !read_success {
        // Simple fallback seed
        let seed = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => d.as_nanos(),
            Err(_) => 123456789,
        };
        for i in 0..16 {
            bytes[i] = ((seed >> (i * 8)) & 0xFF) as u8;
        }
    }
    
    // Set UUID v4 variant/version bits
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 10xxxxxx
    
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

// Extract string value from simple JSON input
pub fn get_json_string_field(json: &str, key: &str) -> String {
    let key_pattern = format!("\"{}\"", key);
    if let Some(key_idx) = json.find(&key_pattern) {
        let after = &json[key_idx + key_pattern.len()..];
        if let Some(colon_idx) = after.find(':') {
            let after_colon = &after[colon_idx + 1..];
            if let Some(start_quote) = after_colon.find('"') {
                let after_quote = &after_colon[start_quote + 1..];
                if let Some(end_quote) = after_quote.find('"') {
                    return after_quote[..end_quote].to_string();
                }
            } else {
                let trimmed = after_colon.trim();
                let len = trimmed.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.').count();
                return trimmed[..len].to_string();
            }
        }
    }
    String::new()
}
