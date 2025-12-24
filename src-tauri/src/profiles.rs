use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ========== Profile Data Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub url: Option<String>, // Subscription URL if any
    pub file_path: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesData {
    pub profiles: Vec<Profile>,
    pub active_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProxyNode {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cipher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Rule {
    pub rule_type: String,
    pub matcher: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MihomoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(rename = "socks-port", skip_serializing_if = "Option::is_none")]
    pub socks_port: Option<u16>,
    #[serde(rename = "mixed-port", skip_serializing_if = "Option::is_none")]
    pub mixed_port: Option<u16>,
    #[serde(rename = "allow-lan", skip_serializing_if = "Option::is_none")]
    pub allow_lan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(rename = "log-level", skip_serializing_if = "Option::is_none")]
    pub log_level: Option<String>,
    #[serde(
        rename = "external-controller",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_controller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxies: Option<Vec<serde_yaml::Value>>,
    #[serde(rename = "proxy-groups", skip_serializing_if = "Option::is_none")]
    pub proxy_groups: Option<Vec<serde_yaml::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

// ========== Helper Functions ==========

fn get_profiles_dir() -> PathBuf {
    let app_data = dirs::data_local_dir().unwrap_or_default();
    app_data.join("aqiu").join("profiles")
}

fn get_profiles_index_path() -> PathBuf {
    get_profiles_dir().join("profiles.json")
}

fn load_profiles_data() -> ProfilesData {
    let path = get_profiles_index_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str(&content) {
                return data;
            }
        }
    }
    ProfilesData::default()
}

fn save_profiles_data(data: &ProfilesData) -> Result<(), String> {
    let dir = get_profiles_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(get_profiles_index_path(), content).map_err(|e| e.to_string())?;

    Ok(())
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    format!("{:x}", now.as_millis())
}

fn get_current_time() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn is_proxy_mapping(value: &serde_yaml::Value) -> bool {
    let mapping = match value.as_mapping() {
        Some(m) => m,
        None => return false,
    };
    let name_key = serde_yaml::Value::String("name".to_string());
    let type_key = serde_yaml::Value::String("type".to_string());
    let server_key = serde_yaml::Value::String("server".to_string());
    let port_key = serde_yaml::Value::String("port".to_string());
    mapping.contains_key(&name_key)
        && mapping.contains_key(&type_key)
        && mapping.contains_key(&server_key)
        && mapping.contains_key(&port_key)
}

fn normalize_config_value(value: serde_yaml::Value) -> serde_yaml::Value {
    if let serde_yaml::Value::Sequence(items) = &value {
        let all_proxies = items.iter().all(is_proxy_mapping);
        if all_proxies {
            let mut root = serde_yaml::Mapping::new();
            root.insert(
                serde_yaml::Value::String("mixed-port".to_string()),
                serde_yaml::Value::Number(27890.into()),
            );
            root.insert(
                serde_yaml::Value::String("allow-lan".to_string()),
                serde_yaml::Value::Bool(false),
            );
            root.insert(
                serde_yaml::Value::String("mode".to_string()),
                serde_yaml::Value::String("Rule".to_string()),
            );
            root.insert(
                serde_yaml::Value::String("log-level".to_string()),
                serde_yaml::Value::String("info".to_string()),
            );
            root.insert(
                serde_yaml::Value::String("external-controller".to_string()),
                serde_yaml::Value::String("127.0.0.1:29090".to_string()),
            );
            root.insert(serde_yaml::Value::String("proxies".to_string()), value);
            root.insert(
                serde_yaml::Value::String("proxy-groups".to_string()),
                serde_yaml::Value::Sequence(vec![]),
            );
            root.insert(
                serde_yaml::Value::String("rules".to_string()),
                serde_yaml::Value::Sequence(vec![]),
            );
            return serde_yaml::Value::Mapping(root);
        }
    }
    if let serde_yaml::Value::Mapping(root) = value {
        return serde_yaml::Value::Mapping(root);
    }
    value
}

fn normalize_config_content(content: &str) -> Result<serde_yaml::Value, String> {
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;
    Ok(normalize_config_value(yaml))
}

fn create_profile_with_content(
    name: String,
    url: Option<String>,
    content: String,
) -> Result<Profile, String> {
    let mut data = load_profiles_data();
    let id = generate_id();
    let file_path = get_profiles_dir().join(format!("{}.yaml", id));

    fs::create_dir_all(get_profiles_dir()).map_err(|e| e.to_string())?;
    fs::write(&file_path, content).map_err(|e| e.to_string())?;

    let is_first = data.profiles.is_empty();
    let profile = Profile {
        id: id.clone(),
        name,
        url,
        file_path: file_path.to_string_lossy().to_string(),
        updated_at: get_current_time(),
        is_active: is_first,
    };

    if is_first {
        data.active_id = Some(id);
    }

    data.profiles.push(profile.clone());
    save_profiles_data(&data)?;

    Ok(profile)
}

fn is_proxy_url(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("ss://")
        || trimmed.starts_with("vmess://")
        || trimmed.starts_with("vless://")
        || trimmed.starts_with("trojan://")
        || trimmed.starts_with("ssr://")
}

fn extract_proxy_list(content: &str) -> Option<Vec<String>> {
    let mut urls = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !is_proxy_url(trimmed) {
            return None;
        }
        urls.push(trimmed.to_string());
    }
    if urls.is_empty() {
        None
    } else {
        Some(urls)
    }
}

#[derive(Debug)]
struct ParsedUrl {
    scheme: String,
    userinfo: Option<String>,
    host: String,
    port: u16,
    query: HashMap<String, String>,
    name: Option<String>,
}

fn parse_query_map(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let raw_key = parts.next().unwrap_or("");
        let raw_value = parts.next().unwrap_or("");
        let key = urlencoding::decode(raw_key)
            .unwrap_or_else(|_| raw_key.into())
            .into_owned();
        let value = urlencoding::decode(raw_value)
            .unwrap_or_else(|_| raw_value.into())
            .into_owned();
        if !key.is_empty() {
            map.insert(key, value);
        }
    }
    map
}

fn decode_base64_string(input: &str) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let cleaned = input.trim();
    let decoded = general_purpose::STANDARD
        .decode(cleaned)
        .or_else(|_| general_purpose::URL_SAFE.decode(cleaned))
        .map_err(|e| format!("Failed to decode base64: {}", e))?;
    String::from_utf8(decoded).map_err(|e| e.to_string())
}

fn parse_standard_url(url: &str) -> Result<ParsedUrl, String> {
    let (before_fragment, name) = if let Some(pos) = url.find('#') {
        let (left, right) = url.split_at(pos);
        let name = right.trim_start_matches('#');
        let name = if name.is_empty() {
            None
        } else {
            Some(
                urlencoding::decode(name)
                    .unwrap_or_else(|_| name.into())
                    .into_owned(),
            )
        };
        (left, name)
    } else {
        (url, None)
    };

    let scheme_pos = before_fragment
        .find("://")
        .ok_or("Invalid URL: missing scheme")?;
    let scheme = before_fragment[..scheme_pos].to_lowercase();
    let mut rest = &before_fragment[scheme_pos + 3..];

    let (authority, query) = if let Some(pos) = rest.find('?') {
        let (left, right) = rest.split_at(pos);
        rest = left;
        (rest, Some(&right[1..]))
    } else {
        (rest, None)
    };

    let (userinfo, hostport) = if let Some(pos) = authority.rfind('@') {
        let (left, right) = authority.split_at(pos);
        let info = left.trim();
        let info = if info.is_empty() {
            None
        } else {
            Some(
                urlencoding::decode(info)
                    .unwrap_or_else(|_| info.into())
                    .into_owned(),
            )
        };
        (info, &right[1..])
    } else {
        (None, authority)
    };

    let hostport = hostport.split('/').next().unwrap_or(hostport).trim();
    if hostport.is_empty() {
        return Err("Invalid URL: missing host".to_string());
    }

    let (host, port) = if hostport.starts_with('[') {
        let end = hostport.find(']').ok_or("Invalid URL: invalid IPv6 host")?;
        let host = hostport[1..end].to_string();
        let port = hostport[end + 1..].trim_start_matches(':');
        if port.is_empty() {
            return Err("Invalid URL: missing port".to_string());
        }
        (host, port.parse::<u16>().map_err(|e| e.to_string())?)
    } else {
        let mut parts = hostport.rsplitn(2, ':');
        let port_str = parts.next().unwrap_or("");
        let host_str = parts.next().unwrap_or("");
        if host_str.is_empty() || port_str.is_empty() {
            return Err("Invalid URL: missing host or port".to_string());
        }
        (
            host_str.to_string(),
            port_str.parse::<u16>().map_err(|e| e.to_string())?,
        )
    };

    let query = query.map(parse_query_map).unwrap_or_default();

    Ok(ParsedUrl {
        scheme,
        userinfo,
        host,
        port,
        query,
        name,
    })
}

fn set_bool(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, value: &str) {
    let normalized = value.trim().to_lowercase();
    let parsed = matches!(normalized.as_str(), "1" | "true" | "yes" | "y");
    map.insert(key.to_string(), serde_json::Value::Bool(parsed));
}

fn set_string(map: &mut serde_json::Map<String, serde_json::Value>, key: &str, value: &str) {
    map.insert(
        key.to_string(),
        serde_json::Value::String(value.to_string()),
    );
}

fn set_number_or_string(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: &str,
) {
    if let Ok(num) = value.parse::<u64>() {
        map.insert(key.to_string(), serde_json::Value::Number(num.into()));
    } else {
        set_string(map, key, value);
    }
}

fn apply_common_query(
    map: &mut serde_json::Map<String, serde_json::Value>,
    query: &HashMap<String, String>,
) {
    if let Some(value) = query
        .get("sni")
        .or_else(|| query.get("servername"))
        .or_else(|| query.get("peer"))
    {
        set_string(map, "sni", value);
    }

    if let Some(value) = query.get("alpn") {
        let list: Vec<serde_json::Value> = value
            .split(',')
            .map(|v| serde_json::Value::String(v.trim().to_string()))
            .filter(|v| v.as_str().map(|s| !s.is_empty()).unwrap_or(false))
            .collect();
        if !list.is_empty() {
            map.insert("alpn".to_string(), serde_json::Value::Array(list));
        }
    }

    if let Some(value) = query.get("udp") {
        set_bool(map, "udp", value);
    }

    if let Some(value) = query.get("tls") {
        set_bool(map, "tls", value);
    }

    if let Some(value) = query.get("insecure") {
        set_bool(map, "skip-cert-verify", value);
    }

    if let Some(value) = query
        .get("fp")
        .or_else(|| query.get("fingerprint"))
        .or_else(|| query.get("client-fingerprint"))
    {
        set_string(map, "client-fingerprint", value);
    }

    if let Some(value) = query.get("type").or_else(|| query.get("network")) {
        set_string(map, "network", value);
    }

    if let Some(value) = query.get("path") {
        set_string(map, "path", value);
    }

    if let Some(value) = query.get("host") {
        set_string(map, "host", value);
    }
}

fn build_base_proxy(
    name: Option<String>,
    proxy_type: &str,
    host: &str,
    port: u16,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    let proxy_name =
        name.unwrap_or_else(|| format!("{}-{}-{}", proxy_type.to_uppercase(), host, port));
    map.insert("name".to_string(), serde_json::Value::String(proxy_name));
    map.insert(
        "type".to_string(),
        serde_json::Value::String(proxy_type.to_string()),
    );
    map.insert(
        "server".to_string(),
        serde_json::Value::String(host.to_string()),
    );
    map.insert("port".to_string(), serde_json::Value::Number(port.into()));
    map.insert("udp".to_string(), serde_json::Value::Bool(true));
    map
}

fn parse_ssr_url(url: &str) -> Result<serde_json::Value, String> {
    let without_prefix = url.trim().strip_prefix("ssr://").ok_or("Invalid SSR URL")?;
    let decoded = decode_base64_string(without_prefix)?;
    let (main_part, params_part) = if let Some(pos) = decoded.find("/?") {
        (&decoded[..pos], Some(&decoded[pos + 2..]))
    } else {
        (decoded.as_str(), None)
    };

    let parts: Vec<&str> = main_part.split(':').collect();
    if parts.len() < 6 {
        return Err("Invalid SSR URL: missing fields".to_string());
    }

    let server = parts[0].to_string();
    let port = parts[1].parse::<u16>().map_err(|e| e.to_string())?;
    let protocol = parts[2].to_string();
    let method = parts[3].to_string();
    let obfs = parts[4].to_string();
    let password = decode_base64_string(parts[5])?;

    let mut map = serde_json::Map::new();
    map.insert(
        "name".to_string(),
        serde_json::Value::String(format!("SSR-{}-{}", server, port)),
    );
    map.insert(
        "type".to_string(),
        serde_json::Value::String("ssr".to_string()),
    );
    map.insert("server".to_string(), serde_json::Value::String(server));
    map.insert("port".to_string(), serde_json::Value::Number(port.into()));
    map.insert("protocol".to_string(), serde_json::Value::String(protocol));
    map.insert("cipher".to_string(), serde_json::Value::String(method));
    map.insert("obfs".to_string(), serde_json::Value::String(obfs));
    map.insert("password".to_string(), serde_json::Value::String(password));
    map.insert("udp".to_string(), serde_json::Value::Bool(true));

    if let Some(params) = params_part {
        let params_map = parse_query_map(params);
        if let Some(value) = params_map.get("obfsparam") {
            if let Ok(decoded) = decode_base64_string(value) {
                set_string(&mut map, "obfs-param", &decoded);
            }
        }
        if let Some(value) = params_map.get("protoparam") {
            if let Ok(decoded) = decode_base64_string(value) {
                set_string(&mut map, "protocol-param", &decoded);
            }
        }
        if let Some(value) = params_map.get("remarks") {
            if let Ok(decoded) = decode_base64_string(value) {
                set_string(&mut map, "name", &decoded);
            }
        }
    }

    Ok(serde_json::Value::Object(map))
}

fn parse_proxy_url_value(url: &str) -> Result<serde_json::Value, String> {
    let url = url.trim();

    if url.starts_with("ss://") {
        // Basic SS URL parsing: ss://base64(method:password)@host:port#name
        // Or ss://base64(method:password)@host:port/?plugin=...#name

        let without_prefix = &url[5..];
        let (main_part, name) = if let Some(pos) = without_prefix.find('#') {
            (&without_prefix[..pos], Some(&without_prefix[pos + 1..]))
        } else {
            (without_prefix, None)
        };

        let (auth_part, server_part) = if let Some(pos) = main_part.find('@') {
            (&main_part[..pos], &main_part[pos + 1..])
        } else {
            return Err("Invalid SS URL: missing @".to_string());
        };

        use base64::{engine::general_purpose, Engine as _};
        let auth_decoded = general_purpose::STANDARD
            .decode(auth_part)
            .or_else(|_| general_purpose::URL_SAFE.decode(auth_part))
            .map_err(|e| format!("Failed to decode SS auth: {}", e))?;

        let auth_str = String::from_utf8(auth_decoded).map_err(|e| e.to_string())?;
        let auth_parts: Vec<&str> = auth_str.splitn(2, ':').collect();
        if auth_parts.len() < 2 {
            return Err("Invalid SS auth: missing colon".to_string());
        }

        let method = auth_parts[0];
        let password = auth_parts[1];

        let server_parts: Vec<&str> = server_part.splitn(2, ':').collect();
        if server_parts.len() < 2 {
            return Err("Invalid SS server: missing port".to_string());
        }

        let server = server_parts[0];
        let port_str = server_parts[1].split('/').next().unwrap_or(server_parts[1]);
        let port = port_str.parse::<u16>().map_err(|e| e.to_string())?;

        let name = name
            .map(|n| urlencoding::decode(n).unwrap_or(n.into()).into_owned())
            .unwrap_or_else(|| format!("SS-{}-{}", server, port));

        return Ok(serde_json::json!({
            "name": name,
            "type": "ss",
            "server": server,
            "port": port,
            "password": password,
            "cipher": method,
            "udp": true
        }));
    } else if url.starts_with("vmess://") {
        let without_prefix = &url[8..];
        use base64::{engine::general_purpose, Engine as _};
        let decoded = general_purpose::STANDARD
            .decode(without_prefix)
            .map_err(|e| format!("Failed to decode vmess: {}", e))?;

        let vmess_json: serde_json::Value =
            serde_json::from_slice(&decoded).map_err(|e| format!("Invalid vmess JSON: {}", e))?;

        return Ok(serde_json::json!({
            "name": vmess_json["ps"].as_str().unwrap_or("VMess"),
            "type": "vmess",
            "server": vmess_json["add"].as_str().unwrap_or(""),
            "port": vmess_json["port"].as_u64().unwrap_or(0) as u16,
            "uuid": vmess_json["id"].as_str().unwrap_or(""),
            "alterId": vmess_json["aid"].as_u64().unwrap_or(0),
            "cipher": "auto",
            "tls": vmess_json["tls"].as_str() == Some("tls"),
            "network": vmess_json["net"].as_str().unwrap_or("tcp"),
            "udp": true
        }));
    } else if url.starts_with("trojan://") {
        // trojan://password@host:port#name
        let without_prefix = &url[9..];
        let (main_part, name) = if let Some(pos) = without_prefix.find('#') {
            (&without_prefix[..pos], Some(&without_prefix[pos + 1..]))
        } else {
            (without_prefix, None)
        };

        let (password, server_part) = if let Some(pos) = main_part.find('@') {
            (&main_part[..pos], &main_part[pos + 1..])
        } else {
            return Err("Invalid Trojan URL: missing @".to_string());
        };

        let server_parts: Vec<&str> = server_part.splitn(2, ':').collect();
        if server_parts.len() < 2 {
            return Err("Invalid Trojan server: missing port".to_string());
        }

        let server = server_parts[0];
        let port = server_parts[1].parse::<u16>().map_err(|e| e.to_string())?;

        let name = name
            .map(|n| urlencoding::decode(n).unwrap_or(n.into()).into_owned())
            .unwrap_or_else(|| format!("Trojan-{}-{}", server, port));

        return Ok(serde_json::json!({
            "name": name,
            "type": "trojan",
            "server": server,
            "port": port,
            "password": password,
            "udp": true,
            "sni": server
        }));
    } else if url.starts_with("ssr://") {
        return parse_ssr_url(url);
    } else if url.starts_with("vless://")
        || url.starts_with("socks5://")
        || url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("hysteria://")
        || url.starts_with("hysteria2://")
        || url.starts_with("hy2://")
        || url.starts_with("tuic://")
        || url.starts_with("wireguard://")
        || url.starts_with("wg://")
    {
        let parsed = parse_standard_url(url)?;
        let mut map = build_base_proxy(parsed.name, &parsed.scheme, &parsed.host, parsed.port);

        match parsed.scheme.as_str() {
            "vless" => {
                let uuid = parsed.userinfo.ok_or("Invalid VLESS URL: missing uuid")?;
                set_string(&mut map, "uuid", &uuid);
                if let Some(value) = parsed.query.get("encryption") {
                    set_string(&mut map, "encryption", value);
                }
                if let Some(value) = parsed.query.get("flow") {
                    set_string(&mut map, "flow", value);
                }
                if let Some(value) = parsed.query.get("security") {
                    if value == "tls" || value == "reality" {
                        map.insert("tls".to_string(), serde_json::Value::Bool(true));
                    }
                    if value == "reality" {
                        let mut reality_opts = serde_json::Map::new();
                        if let Some(pbk) = parsed
                            .query
                            .get("pbk")
                            .or_else(|| parsed.query.get("publickey"))
                            .or_else(|| parsed.query.get("public_key"))
                        {
                            set_string(&mut reality_opts, "public-key", pbk);
                        }
                        if let Some(sid) = parsed
                            .query
                            .get("sid")
                            .or_else(|| parsed.query.get("shortid"))
                            .or_else(|| parsed.query.get("short_id"))
                        {
                            set_string(&mut reality_opts, "short-id", sid);
                        }
                        if let Some(spx) = parsed
                            .query
                            .get("spx")
                            .or_else(|| parsed.query.get("spiderx"))
                            .or_else(|| parsed.query.get("spider_x"))
                        {
                            set_string(&mut reality_opts, "spider-x", spx);
                        }
                        if !reality_opts.is_empty() {
                            map.insert(
                                "reality-opts".to_string(),
                                serde_json::Value::Object(reality_opts),
                            );
                        }
                    }
                }
                apply_common_query(&mut map, &parsed.query);
            }
            "socks5" => {
                if let Some(info) = parsed.userinfo {
                    let mut parts = info.splitn(2, ':');
                    let username = parts.next().unwrap_or("").to_string();
                    let password = parts.next().unwrap_or("").to_string();
                    if !username.is_empty() {
                        set_string(&mut map, "username", &username);
                    }
                    if !password.is_empty() {
                        set_string(&mut map, "password", &password);
                    }
                }
                apply_common_query(&mut map, &parsed.query);
            }
            "http" | "https" => {
                map.insert(
                    "type".to_string(),
                    serde_json::Value::String("http".to_string()),
                );
                if parsed.scheme == "https" {
                    map.insert("tls".to_string(), serde_json::Value::Bool(true));
                }
                if let Some(info) = parsed.userinfo {
                    let mut parts = info.splitn(2, ':');
                    let username = parts.next().unwrap_or("").to_string();
                    let password = parts.next().unwrap_or("").to_string();
                    if !username.is_empty() {
                        set_string(&mut map, "username", &username);
                    }
                    if !password.is_empty() {
                        set_string(&mut map, "password", &password);
                    }
                }
                apply_common_query(&mut map, &parsed.query);
            }
            "hysteria" => {
                if let Some(info) = parsed.userinfo {
                    set_string(&mut map, "auth-str", &info);
                }
                if let Some(value) = parsed
                    .query
                    .get("auth")
                    .or_else(|| parsed.query.get("auth_str"))
                {
                    set_string(&mut map, "auth-str", value);
                }
                if let Some(value) = parsed.query.get("up") {
                    set_number_or_string(&mut map, "up", value);
                }
                if let Some(value) = parsed.query.get("down") {
                    set_number_or_string(&mut map, "down", value);
                }
                if let Some(value) = parsed.query.get("obfs") {
                    set_string(&mut map, "obfs", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("obfs-password")
                    .or_else(|| parsed.query.get("obfs_password"))
                {
                    set_string(&mut map, "obfs-password", value);
                }
                apply_common_query(&mut map, &parsed.query);
            }
            "hysteria2" | "hy2" => {
                map.insert(
                    "type".to_string(),
                    serde_json::Value::String("hysteria2".to_string()),
                );
                if let Some(info) = parsed.userinfo {
                    set_string(&mut map, "password", &info);
                }
                if let Some(value) = parsed.query.get("password") {
                    set_string(&mut map, "password", value);
                }
                if let Some(value) = parsed.query.get("obfs") {
                    set_string(&mut map, "obfs", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("obfs-password")
                    .or_else(|| parsed.query.get("obfs_password"))
                {
                    set_string(&mut map, "obfs-password", value);
                }
                apply_common_query(&mut map, &parsed.query);
            }
            "tuic" => {
                if let Some(info) = parsed.userinfo {
                    let mut parts = info.splitn(2, ':');
                    let uuid = parts.next().unwrap_or("").to_string();
                    let password = parts.next().unwrap_or("").to_string();
                    if !uuid.is_empty() {
                        set_string(&mut map, "uuid", &uuid);
                    }
                    if !password.is_empty() {
                        set_string(&mut map, "password", &password);
                    }
                }
                if let Some(value) = parsed.query.get("uuid") {
                    set_string(&mut map, "uuid", value);
                }
                if let Some(value) = parsed.query.get("password") {
                    set_string(&mut map, "password", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("congestion_control")
                    .or_else(|| parsed.query.get("congestion-controller"))
                {
                    set_string(&mut map, "congestion-controller", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("udp_relay_mode")
                    .or_else(|| parsed.query.get("udp-relay-mode"))
                {
                    set_string(&mut map, "udp-relay-mode", value);
                }
                apply_common_query(&mut map, &parsed.query);
            }
            "wireguard" | "wg" => {
                map.insert(
                    "type".to_string(),
                    serde_json::Value::String("wireguard".to_string()),
                );
                if let Some(info) = parsed.userinfo {
                    set_string(&mut map, "private-key", &info);
                }
                if let Some(value) = parsed
                    .query
                    .get("private_key")
                    .or_else(|| parsed.query.get("private-key"))
                {
                    set_string(&mut map, "private-key", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("public_key")
                    .or_else(|| parsed.query.get("public-key"))
                {
                    set_string(&mut map, "public-key", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("preshared_key")
                    .or_else(|| parsed.query.get("pre_shared_key"))
                    .or_else(|| parsed.query.get("pre-shared-key"))
                {
                    set_string(&mut map, "pre-shared-key", value);
                }
                if let Some(value) = parsed.query.get("reserved") {
                    set_string(&mut map, "reserved", value);
                }
                if let Some(value) = parsed.query.get("mtu") {
                    set_number_or_string(&mut map, "mtu", value);
                }
                if let Some(value) = parsed
                    .query
                    .get("address")
                    .or_else(|| parsed.query.get("ip"))
                {
                    set_string(&mut map, "ip", value);
                }
                apply_common_query(&mut map, &parsed.query);
            }
            _ => {}
        }

        return Ok(serde_json::Value::Object(map));
    }

    Err("Unsupported proxy URL format".to_string())
}

fn build_config_from_proxy_urls(urls: &[String]) -> Result<serde_yaml::Value, String> {
    let mut proxies_yaml = Vec::new();
    let mut proxy_names = Vec::new();

    for url in urls {
        let proxy_json = parse_proxy_url_value(url)?;
        let proxy_yaml: serde_yaml::Value =
            serde_json::from_value(proxy_json).map_err(|e| format!("Invalid proxy data: {}", e))?;
        if let Some(name) = proxy_yaml
            .as_mapping()
            .and_then(|m| m.get(&serde_yaml::Value::String("name".to_string())))
            .and_then(|v| v.as_str())
        {
            proxy_names.push(name.to_string());
        }
        proxies_yaml.push(proxy_yaml);
    }

    let mut root = serde_yaml::Mapping::new();
    root.insert(
        serde_yaml::Value::String("mixed-port".to_string()),
        serde_yaml::Value::Number(7890.into()),
    );
    root.insert(
        serde_yaml::Value::String("allow-lan".to_string()),
        serde_yaml::Value::Bool(false),
    );
    root.insert(
        serde_yaml::Value::String("mode".to_string()),
        serde_yaml::Value::String("Rule".to_string()),
    );
    root.insert(
        serde_yaml::Value::String("log-level".to_string()),
        serde_yaml::Value::String("info".to_string()),
    );
    root.insert(
        serde_yaml::Value::String("external-controller".to_string()),
        serde_yaml::Value::String("127.0.0.1:29090".to_string()),
    );
    root.insert(
        serde_yaml::Value::String("proxies".to_string()),
        serde_yaml::Value::Sequence(proxies_yaml),
    );

    let mut group = serde_yaml::Mapping::new();
    group.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String("Proxy".to_string()),
    );
    group.insert(
        serde_yaml::Value::String("type".to_string()),
        serde_yaml::Value::String("select".to_string()),
    );
    let mut group_proxies = vec![serde_yaml::Value::String("DIRECT".to_string())];
    group_proxies.extend(proxy_names.into_iter().map(serde_yaml::Value::String));
    group.insert(
        serde_yaml::Value::String("proxies".to_string()),
        serde_yaml::Value::Sequence(group_proxies),
    );

    root.insert(
        serde_yaml::Value::String("proxy-groups".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::Mapping(group)]),
    );
    root.insert(
        serde_yaml::Value::String("rules".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("MATCH,Proxy".to_string())]),
    );

    Ok(serde_yaml::Value::Mapping(root))
}

// ========== Commands ==========

#[tauri::command]
pub fn list_profiles() -> Result<Vec<Profile>, String> {
    let data = load_profiles_data();
    Ok(data.profiles)
}

#[tauri::command]
pub fn get_active_profile() -> Result<Option<Profile>, String> {
    let data = load_profiles_data();
    if let Some(active_id) = &data.active_id {
        return Ok(data.profiles.into_iter().find(|p| &p.id == active_id));
    }
    Ok(None)
}

#[tauri::command]
pub fn create_profile(name: String, url: Option<String>) -> Result<Profile, String> {
    // Create empty config file
    let default_config = r#"mixed-port: 27890
allow-lan: false
mode: Rule
log-level: info
external-controller: 127.0.0.1:29090

proxies: []

proxy-groups: []

rules:
  - MATCH,DIRECT
"#;

    create_profile_with_content(name, url, default_config.to_string())
}

#[tauri::command]
pub fn create_profile_from_path(
    name: String,
    path: String,
    url: Option<String>,
) -> Result<Profile, String> {
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let normalized = normalize_config_content(&content)?;
    let new_content = serde_yaml::to_string(&normalized).map_err(|e| e.to_string())?;
    create_profile_with_content(name, url, new_content)
}

#[tauri::command]
pub fn delete_profile(id: String) -> Result<(), String> {
    let mut data = load_profiles_data();

    if let Some(pos) = data.profiles.iter().position(|p| p.id == id) {
        let profile = &data.profiles[pos];

        // Delete file
        let _ = fs::remove_file(&profile.file_path);

        data.profiles.remove(pos);

        if data.active_id.as_ref() == Some(&id) {
            data.active_id = data.profiles.first().map(|p| p.id.clone());
        }

        save_profiles_data(&data)?;
    }

    Ok(())
}

#[tauri::command]
pub fn set_active_profile(id: String) -> Result<(), String> {
    let mut data = load_profiles_data();

    // Verify profile exists
    if !data.profiles.iter().any(|p| p.id == id) {
        return Err("Profile not found".to_string());
    }

    // Update is_active flags
    for p in &mut data.profiles {
        p.is_active = p.id == id;
    }

    data.active_id = Some(id);
    save_profiles_data(&data)?;

    Ok(())
}

#[tauri::command]
pub fn get_profile_content(id: String) -> Result<String, String> {
    let data = load_profiles_data();

    let profile = data
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or("Profile not found")?;

    fs::read_to_string(&profile.file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_profile_content(id: String, content: String) -> Result<(), String> {
    let mut data = load_profiles_data();

    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found")?;

    let normalized = normalize_config_content(&content)?;
    let new_content = serde_yaml::to_string(&normalized).map_err(|e| e.to_string())?;
    fs::write(&profile.file_path, &new_content).map_err(|e| e.to_string())?;

    profile.updated_at = get_current_time();
    save_profiles_data(&data)?;

    Ok(())
}

#[tauri::command]
pub fn rename_profile(id: String, new_name: String) -> Result<(), String> {
    let mut data = load_profiles_data();

    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found")?;

    profile.name = new_name;
    save_profiles_data(&data)?;

    Ok(())
}

#[tauri::command]
pub async fn update_profile_from_url(id: String) -> Result<String, String> {
    let mut data = load_profiles_data();

    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found")?;

    let url = profile
        .url
        .clone()
        .ok_or("No subscription URL for this profile")?;

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "clash-verge/1.0.0") // Use a common user agent
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let mut content = response.text().await.map_err(|e| e.to_string())?;

    // Try to parse as YAML first
    let mut is_valid_yaml = serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok();
    let mut proxy_list = extract_proxy_list(&content);

    // If not valid YAML or it's a proxy list, try base64 decoding
    if !is_valid_yaml || proxy_list.is_some() {
        use base64::{engine::general_purpose, Engine as _};
        if let Ok(decoded_bytes) =
            general_purpose::STANDARD.decode(content.trim().replace("\r\n", "").replace("\n", ""))
        {
            if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                if serde_yaml::from_str::<serde_yaml::Value>(&decoded_str).is_ok() {
                    content = decoded_str;
                    is_valid_yaml = true;
                    proxy_list = None;
                } else {
                    content = decoded_str;
                    proxy_list = extract_proxy_list(&content);
                }
            }
        }
    }

    if !is_valid_yaml {
        if let Some(urls) = proxy_list {
            let config = build_config_from_proxy_urls(&urls)?;
            content = serde_yaml::to_string(&config).map_err(|e| e.to_string())?;
            is_valid_yaml = true;
        }
    }

    if !is_valid_yaml {
        return Err("Invalid config (not valid YAML or base64-encoded YAML/URL list)".to_string());
    }

    let normalized = normalize_config_content(&content)?;
    let new_content = serde_yaml::to_string(&normalized).map_err(|e| e.to_string())?;
    fs::write(&profile.file_path, &new_content).map_err(|e| e.to_string())?;

    profile.updated_at = get_current_time();
    save_profiles_data(&data)?;

    Ok("Updated successfully".to_string())
}

#[tauri::command]
pub fn parse_proxy_url(url: String) -> Result<serde_json::Value, String> {
    parse_proxy_url_value(&url)
}

#[tauri::command]
pub fn add_proxy_to_profile(id: String, proxy: serde_json::Value) -> Result<(), String> {
    let mut data = load_profiles_data();
    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found")?;

    let content = fs::read_to_string(&profile.file_path).map_err(|e| e.to_string())?;
    let mut config: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Invalid YAML in profile: {}", e))?;

    let proxy_yaml: serde_yaml::Value =
        serde_json::from_value(proxy).map_err(|e| format!("Invalid proxy data: {}", e))?;

    if let Some(config_obj) = config.as_mapping_mut() {
        let proxies_key = serde_yaml::Value::String("proxies".to_string());
        if !config_obj.contains_key(&proxies_key) {
            config_obj.insert(proxies_key.clone(), serde_yaml::Value::Sequence(vec![]));
        }

        if let Some(proxies) = config_obj
            .get_mut(&proxies_key)
            .and_then(|v| v.as_sequence_mut())
        {
            proxies.push(proxy_yaml.clone());
        }

        // Also add to default proxy group if it exists
        let groups_key = serde_yaml::Value::String("proxy-groups".to_string());
        if let Some(groups) = config_obj
            .get_mut(&groups_key)
            .and_then(|v| v.as_sequence_mut())
        {
            for group in groups {
                if let Some(group_obj) = group.as_mapping_mut() {
                    let name_key = serde_yaml::Value::String("name".to_string());
                    if let Some(name) = group_obj.get(&name_key).and_then(|v| v.as_str()) {
                        if name == "Proxy" || name == "节点选择" {
                            let proxies_in_group_key =
                                serde_yaml::Value::String("proxies".to_string());
                            if let Some(proxies_in_group) = group_obj
                                .get_mut(&proxies_in_group_key)
                                .and_then(|v| v.as_sequence_mut())
                            {
                                if let Some(proxy_name) =
                                    proxy_yaml.as_mapping().and_then(|m| m.get(&name_key))
                                {
                                    proxies_in_group.insert(0, proxy_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let new_content = serde_yaml::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(&profile.file_path, new_content).map_err(|e| e.to_string())?;

    profile.updated_at = get_current_time();
    save_profiles_data(&data)?;

    Ok(())
}

#[tauri::command]
pub fn parse_config(content: String) -> Result<serde_json::Value, String> {
    let normalized = normalize_config_content(&content)?;
    serde_json::to_value(normalized).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config_obj(id: String, config: serde_json::Value) -> Result<(), String> {
    let mut data = load_profiles_data();
    let profile = data
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Profile not found")?;

    let yaml_value: serde_yaml::Value =
        serde_json::from_value(config).map_err(|e| format!("Invalid config data: {}", e))?;

    let content = serde_yaml::to_string(&yaml_value).map_err(|e| e.to_string())?;
    fs::write(&profile.file_path, content).map_err(|e| e.to_string())?;

    profile.updated_at = get_current_time();
    save_profiles_data(&data)?;

    Ok(())
}

#[tauri::command]
pub fn get_active_profile_path() -> Result<Option<String>, String> {
    let data = load_profiles_data();

    if let Some(active_id) = &data.active_id {
        if let Some(profile) = data.profiles.iter().find(|p| &p.id == active_id) {
            return Ok(Some(profile.file_path.clone()));
        }
    }

    Ok(None)
}
