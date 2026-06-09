use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
#[allow(unused_imports)]
use std::process::Command;

fn get_paths() -> (&'static str, &'static str) {
    let conf = "/usr/local/etc/v2ray/reality.conf";
    let json = "/usr/local/etc/v2ray/config.json";
    if Path::new("/usr/local/etc/v2ray").exists() {
        (conf, json)
    } else {
        ("reality.conf", "config.json")
    }
}

fn load_reality_conf(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if Path::new(path).exists() {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(idx) = trimmed.find('=') {
                    let k = trimmed[..idx].trim().to_string();
                    let v = trimmed[idx + 1..].trim().to_string();
                    map.insert(k, v);
                }
            }
        }
    }
    map
}

fn save_reality_conf(path: &str, conf: &HashMap<String, String>) -> io::Result<()> {
    let mut content = String::new();
    for (k, v) in conf {
        content.push_str(&format!("{}={}\n", k, v));
    }
    fs::write(path, content)
}

fn find_reality_settings_block(json: &str) -> Option<(usize, usize)> {
    if let Some(rs_idx) = json.find("\"realitySettings\"") {
        let after_rs = &json[rs_idx..];
        if let Some(brace_start) = after_rs.find('{') {
            let absolute_start = rs_idx + brace_start;
            let mut nesting = 0;
            for (offset, c) in json[absolute_start..].char_indices() {
                if c == '{' {
                    nesting += 1;
                } else if c == '}' {
                    nesting -= 1;
                    if nesting == 0 {
                        let absolute_end = absolute_start + offset + 1;
                        return Some((rs_idx, absolute_end));
                    }
                }
            }
        }
    }
    None
}

fn json_val_end(s: &str) -> Option<usize> {
    let mut chars = s.char_indices();
    let mut escaped = false;
    while let Some((idx, c)) = chars.next() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(idx);
        }
    }
    None
}

fn replace_json_string_key(block: &str, key: &str, new_val: &str) -> String {
    if let Some(key_idx) = block.find(key) {
        let after_key = &block[key_idx + key.len()..];
        if let Some(colon_idx) = after_key.find(':') {
            let after_colon = &after_key[colon_idx + 1..];
            if let Some(quote_start) = after_colon.find('"') {
                let val_start = key_idx + key.len() + colon_idx + 1 + quote_start + 1;
                if let Some(quote_end) = json_val_end(&block[val_start..]) {
                    let mut new_block = String::new();
                    new_block.push_str(&block[..val_start]);
                    new_block.push_str(new_val);
                    new_block.push_str(&block[val_start + quote_end..]);
                    return new_block;
                }
            }
        }
    }
    block.to_string()
}

fn replace_server_names(block: &str, new_sni_list: &[&str]) -> String {
    if let Some(sn_idx) = block.find("\"serverNames\"") {
        let after_sn = &block[sn_idx + 13..];
        if let Some(colon_idx) = after_sn.find(':') {
            let after_colon = &after_sn[colon_idx + 1..];
            if let Some(bracket_start) = after_colon.find('[') {
                if let Some(bracket_end) = after_colon[bracket_start..].find(']') {
                    let start_idx = sn_idx + 13 + colon_idx + 1 + bracket_start + 1;
                    let end_idx = sn_idx + 13 + colon_idx + 1 + bracket_start + bracket_end;
                    
                    let array_elements: Vec<String> = new_sni_list
                        .iter()
                        .map(|s| format!("\"{}\"", s.trim()))
                        .collect();
                    let new_array_content = array_elements.join(", ");
                    
                    let mut new_block = String::new();
                    new_block.push_str(&block[..start_idx]);
                    new_block.push_str(&new_array_content);
                    new_block.push_str(&block[end_idx..]);
                    return new_block;
                }
            }
        }
    }
    block.to_string()
}

fn main() {
    let (conf_path, config_json_path) = get_paths();
    
    let mut reality_conf = load_reality_conf(conf_path);
    let _curr_dest = reality_conf.get("REALITY_DEST").cloned().unwrap_or_else(|| "yahoo.com:443".to_string());
    let curr_sni = reality_conf.get("REALITY_SNI").cloned().unwrap_or_else(|| "yahoo.com,www.yahoo.com".to_string());
    
    println!("──────────────────────────────────────────");
    println!("          Change Xray Reality SNI         ");
    println!("──────────────────────────────────────────");
    println!(" Current SNI List    : {}", curr_sni);
    println!("──────────────────────────────────────────");
    
    print!(" New SNIs (comma separated, e.g. yahoo.com,www.yahoo.com) [Leave blank to keep current]: ");
    io::stdout().flush().unwrap();
    
    let mut input_buffer = String::new();
    io::stdin().read_line(&mut input_buffer).unwrap();
    let mut new_sni = input_buffer.trim().to_string();
    
    if new_sni.is_empty() {
        new_sni = curr_sni;
    }
    
    let first_sni = match new_sni.split(',').next() {
        Some(s) => s.trim().to_string(),
        None => "yahoo.com".to_string(),
    };
    let new_dest = format!("{}:443", first_sni);
    
    reality_conf.insert("REALITY_DEST".to_string(), new_dest.clone());
    reality_conf.insert("REALITY_SNI".to_string(), new_sni.clone());
    
    if let Err(e) = save_reality_conf(conf_path, &reality_conf) {
        eprintln!("Error saving reality.conf: {}", e);
    }
    
    if Path::new(config_json_path).exists() {
        if let Ok(mut config_content) = fs::read_to_string(config_json_path) {
            if let Some((start, end)) = find_reality_settings_block(&config_content) {
                let block = &config_content[start..end];
                let mut new_block = replace_json_string_key(block, "\"dest\"", &new_dest);
                
                let snis: Vec<&str> = new_sni.split(',').collect();
                new_block = replace_server_names(&new_block, &snis);
                
                config_content.replace_range(start..end, &new_block);
                
                if let Err(e) = fs::write(config_json_path, config_content) {
                    eprintln!("Error writing config.json: {}", e);
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("systemctl").args(&["restart", "v2ray"]).status();
    }
    
    println!("\nReality SNI successfully updated!");
    println!("New SNI list   : {}", new_sni);
    println!("New Destination: {}", new_dest);
    println!("");
    
    print!("Press Enter to go back...");
    io::stdout().flush().unwrap();
    let mut temp = String::new();
    io::stdin().read_line(&mut temp).unwrap();
    
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("menu-reality").status();
    }
}
