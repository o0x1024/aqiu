use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User configuration overrides that take precedence over profile settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfigOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(rename = "socks-port", skip_serializing_if = "Option::is_none")]
    pub socks_port: Option<u16>,
    #[serde(rename = "mixed-port", skip_serializing_if = "Option::is_none")]
    pub mixed_port: Option<u16>,
    #[serde(rename = "redir-port", skip_serializing_if = "Option::is_none")]
    pub redir_port: Option<u16>,
    #[serde(rename = "tproxy-port", skip_serializing_if = "Option::is_none")]
    pub tproxy_port: Option<u16>,
    #[serde(rename = "allow-lan", skip_serializing_if = "Option::is_none")]
    pub allow_lan: Option<bool>,
    #[serde(
        rename = "external-controller",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_controller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tun: Option<TunOverride>,
    /// Persisted core mode preference (macOS only: "user" or "service")
    #[serde(rename = "core-mode", skip_serializing_if = "Option::is_none")]
    pub core_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(rename = "device-id", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u16>,
    #[serde(rename = "strict-route", skip_serializing_if = "Option::is_none")]
    pub strict_route: Option<bool>,
    #[serde(rename = "auto-route", skip_serializing_if = "Option::is_none")]
    pub auto_route: Option<bool>,
    #[serde(
        rename = "auto-detect-interface",
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_detect_interface: Option<bool>,
    #[serde(rename = "dns-hijack", skip_serializing_if = "Option::is_none")]
    pub dns_hijack: Option<Vec<String>>,
}

impl TunOverride {
    fn has_effective_fields(&self) -> bool {
        self.enable.is_some()
            || self.stack.is_some()
            || self.device_id.is_some()
            || self.mtu.is_some()
            || self.strict_route.is_some()
            || self.auto_route.is_some()
            || self.auto_detect_interface.is_some()
            || self.dns_hijack.is_some()
    }
}

fn get_overrides_path() -> PathBuf {
    let app_data = dirs::data_local_dir().unwrap_or_default();
    app_data.join("aqiu").join("user_overrides.json")
}

pub fn load_overrides() -> UserConfigOverrides {
    let path = get_overrides_path();
    if !path.exists() {
        return UserConfigOverrides::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UserConfigOverrides::default(),
    }
}

pub fn save_overrides(overrides: &UserConfigOverrides) -> Result<(), String> {
    let path = get_overrides_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(overrides)
        .map_err(|e| format!("Failed to serialize overrides: {}", e))?;

    fs::write(&path, content).map_err(|e| format!("Failed to write overrides: {}", e))?;

    Ok(())
}

/// Apply user overrides to a YAML config
pub fn apply_overrides_to_yaml(
    yaml: &mut serde_yaml::Value,
    overrides: &UserConfigOverrides,
) -> Result<(), String> {
    let root = yaml
        .as_mapping_mut()
        .ok_or("Config root must be a mapping")?;

    // Apply port overrides
    if let Some(port) = overrides.port {
        root.insert(
            serde_yaml::Value::String("port".to_string()),
            serde_yaml::Value::Number(port.into()),
        );
    }

    if let Some(socks_port) = overrides.socks_port {
        root.insert(
            serde_yaml::Value::String("socks-port".to_string()),
            serde_yaml::Value::Number(socks_port.into()),
        );
    }

    if let Some(mixed_port) = overrides.mixed_port {
        root.insert(
            serde_yaml::Value::String("mixed-port".to_string()),
            serde_yaml::Value::Number(mixed_port.into()),
        );
    }

    if let Some(redir_port) = overrides.redir_port {
        root.insert(
            serde_yaml::Value::String("redir-port".to_string()),
            serde_yaml::Value::Number(redir_port.into()),
        );
    }

    if let Some(tproxy_port) = overrides.tproxy_port {
        root.insert(
            serde_yaml::Value::String("tproxy-port".to_string()),
            serde_yaml::Value::Number(tproxy_port.into()),
        );
    }

    // Apply network overrides
    if let Some(allow_lan) = overrides.allow_lan {
        root.insert(
            serde_yaml::Value::String("allow-lan".to_string()),
            serde_yaml::Value::Bool(allow_lan),
        );
    }

    if let Some(ref external_controller) = overrides.external_controller {
        root.insert(
            serde_yaml::Value::String("external-controller".to_string()),
            serde_yaml::Value::String(external_controller.clone()),
        );
    }

    // Apply TUN overrides
    if let Some(ref tun_override) = overrides.tun {
        if tun_override.has_effective_fields() {
            let tun_key = serde_yaml::Value::String("tun".to_string());
            let mut tun_value = root
                .get(&tun_key)
                .cloned()
                .unwrap_or_else(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

            if let serde_yaml::Value::Mapping(ref mut map) = tun_value {
                if let Some(enable) = tun_override.enable {
                    println!("apply_overrides_to_yaml: Setting TUN enable to {}", enable);
                    map.insert(
                        serde_yaml::Value::String("enable".to_string()),
                        serde_yaml::Value::Bool(enable),
                    );
                } else {
                    println!("apply_overrides_to_yaml: WARNING - TUN override enable is None!");
                }
                if let Some(ref stack) = tun_override.stack {
                    map.insert(
                        serde_yaml::Value::String("stack".to_string()),
                        serde_yaml::Value::String(stack.clone()),
                    );
                }
                if let Some(ref device_id) = tun_override.device_id {
                    map.insert(
                        serde_yaml::Value::String("device-id".to_string()),
                        serde_yaml::Value::String(device_id.clone()),
                    );
                }
                if let Some(mtu) = tun_override.mtu {
                    map.insert(
                        serde_yaml::Value::String("mtu".to_string()),
                        serde_yaml::Value::Number(mtu.into()),
                    );
                }
                if let Some(strict_route) = tun_override.strict_route {
                    map.insert(
                        serde_yaml::Value::String("strict-route".to_string()),
                        serde_yaml::Value::Bool(strict_route),
                    );
                }
                if let Some(auto_route) = tun_override.auto_route {
                    map.insert(
                        serde_yaml::Value::String("auto-route".to_string()),
                        serde_yaml::Value::Bool(auto_route),
                    );
                }
                if let Some(auto_detect) = tun_override.auto_detect_interface {
                    map.insert(
                        serde_yaml::Value::String("auto-detect-interface".to_string()),
                        serde_yaml::Value::Bool(auto_detect),
                    );
                }
                if let Some(ref hijack_list) = tun_override.dns_hijack {
                    let mut seq = serde_yaml::Sequence::new();
                    for entry in hijack_list {
                        seq.push(serde_yaml::Value::String(entry.clone()));
                    }
                    map.insert(
                        serde_yaml::Value::String("dns-hijack".to_string()),
                        serde_yaml::Value::Sequence(seq),
                    );
                }
            }

            root.insert(tun_key, tun_value);
        }
    }

    // --- Ensure DNS config if TUN is enabled ---
    // Calculate effective TUN enable status
    let tun_key = serde_yaml::Value::String("tun".to_string());
    let enable_key = serde_yaml::Value::String("enable".to_string());

    let original_tun_enable = root
        .get(&tun_key)
        .and_then(|v| v.get(&enable_key))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let override_tun_enable = overrides.tun.as_ref().and_then(|t| t.enable);

    let effective_tun_enable = override_tun_enable.unwrap_or(original_tun_enable);

    if effective_tun_enable {
        // First, ensure TUN section has dns-hijack configured
        if let Some(serde_yaml::Value::Mapping(ref mut tun_map)) = root.get_mut(&tun_key) {
            let hijack_key = serde_yaml::Value::String("dns-hijack".to_string());
            if tun_map.get(&hijack_key).is_none() {
                let mut hijack_seq = serde_yaml::Sequence::new();
                hijack_seq.push(serde_yaml::Value::String("any:53".to_string()));
                hijack_seq.push(serde_yaml::Value::String("tcp://any:53".to_string()));
                tun_map.insert(hijack_key, serde_yaml::Value::Sequence(hijack_seq));
                println!("TUN mode: Added default dns-hijack configuration");
            }
        }

        let dns_key = serde_yaml::Value::String("dns".to_string());
        let mut dns_value = root
            .get(&dns_key)
            .cloned()
            .unwrap_or_else(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

        if !matches!(dns_value, serde_yaml::Value::Mapping(_)) {
            dns_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }

        if let serde_yaml::Value::Mapping(ref mut map) = dns_value {
            // Ensure enable is true
            map.insert(enable_key.clone(), serde_yaml::Value::Bool(true));

            macro_rules! ensure_string {
                ($key:expr, $val:expr) => {
                    let k = serde_yaml::Value::String($key.to_string());
                    if map.get(&k).is_none() {
                        map.insert(k, serde_yaml::Value::String($val.to_string()));
                    }
                };
            }

            macro_rules! ensure_bool {
                ($key:expr, $val:expr) => {
                    let k = serde_yaml::Value::String($key.to_string());
                    if map.get(&k).is_none() {
                        map.insert(k, serde_yaml::Value::Bool($val));
                    }
                };
            }

            macro_rules! ensure_sequence {
                ($key:expr, $vals:expr) => {
                    let k = serde_yaml::Value::String($key.to_string());
                    if map.get(&k).is_none() {
                        let mut seq = serde_yaml::Sequence::new();
                        for v in $vals {
                            seq.push(serde_yaml::Value::String(v.to_string()));
                        }
                        map.insert(k, serde_yaml::Value::Sequence(seq));
                    }
                };
            }

            // DNS configuration based on user-provided working config
            // Basic settings
            ensure_bool!("ipv6", false);

            // Enhanced mode and fake-ip settings
            ensure_string!("enhanced-mode", "fake-ip");
            ensure_string!("fake-ip-range", "198.18.0.1/16");

            // Ensure local DNS listener is present when TUN is enabled.
            // Without `dns.listen`, `tun.dns-hijack` may redirect queries to nowhere, causing:
            //   dns resolve failed: couldn't find ip
            {
                let k = serde_yaml::Value::String("listen".to_string());
                if map.get(&k).is_none() {
                    // Service Mode runs as root on macOS, so :53 is OK.
                    // Keep it loopback-only to reduce surface area.
                    map.insert(k, serde_yaml::Value::String("127.0.0.1:53".to_string()));
                    println!("DNS: Added dns.listen=127.0.0.1:53 for TUN mode");
                }
            }

            // Default nameserver (for resolving DoH/DoT DNS addresses)
            //
            // IMPORTANT:
            // `dns resolve failed: couldn't find ip` on the proxy server domain is very often caused by
            // a DNS circular dependency (DNS query tries to go through the proxy, but the proxy server
            // itself needs DNS to connect).
            //
            // We hard-set these to plain IP resolvers to avoid bootstrap issues under TUN.
            {
                let k = serde_yaml::Value::String("default-nameserver".to_string());
                let mut seq = serde_yaml::Sequence::new();
                for v in ["223.5.5.5", "119.29.29.29", "114.114.114.114"] {
                    seq.push(serde_yaml::Value::String(v.to_string()));
                }
                map.insert(k, serde_yaml::Value::Sequence(seq));
            }

            // Primary nameservers
            //
            // IMPORTANT:
            // In TUN + Fake-IP scenarios, using DoH here often requires a bootstrap DNS step and can
            // fail in some networks, leading to:
            //   connect error: dns resolve failed: couldn't find ip
            //
            // Prefer plain IP resolvers here. Avoid relying on DoH bootstrap or system resolvers.
            {
                let k = serde_yaml::Value::String("nameserver".to_string());
                let mut seq = serde_yaml::Sequence::new();
                // Prefer TCP to avoid UDP/53 being blocked in some networks.
                // Keep UDP as fallback.
                for v in [
                    "tcp://223.5.5.5",
                    "tcp://119.29.29.29",
                    "223.5.5.5",
                    "119.29.29.29",
                    "114.114.114.114",
                ] {
                    seq.push(serde_yaml::Value::String(v.to_string()));
                }
                map.insert(k, serde_yaml::Value::Sequence(seq));
            }

            // Nameservers for resolving proxy server domains.
            // This is critical for domains like `api-us-dc1.seckv.com` used by proxies.
            //
            // IMPORTANT: In TUN mode, `dns-hijack: any:53` intercepts ALL port 53 traffic.
            // Using tcp://xxx or plain IP DNS here will be hijacked and cause circular dependency.
            // We MUST use DoH (port 443) or DoT (port 853) which bypass dns-hijack.
            {
                let k = serde_yaml::Value::String("proxy-server-nameserver".to_string());
                let mut seq = serde_yaml::Sequence::new();
                // Use DoH/DoT to avoid dns-hijack interception
                for v in [
                    "https://doh.pub/dns-query",
                    "https://dns.alidns.com/dns-query",
                    "tls://223.5.5.5:853",
                ] {
                    seq.push(serde_yaml::Value::String(v.to_string()));
                }
                map.insert(k, serde_yaml::Value::Sequence(seq));
            }

            // Direct nameserver for bypass/direct rules
            // Use DoH/DoT to avoid dns-hijack interception in TUN mode
            {
                let k = serde_yaml::Value::String("direct-nameserver".to_string());
                let mut seq = serde_yaml::Sequence::new();
                for v in [
                    "https://doh.pub/dns-query",
                    "https://dns.alidns.com/dns-query",
                    "tls://223.5.5.5:853",
                ] {
                    seq.push(serde_yaml::Value::String(v.to_string()));
                }
                map.insert(k, serde_yaml::Value::Sequence(seq));
            }

            // Avoid DNS -> proxy circular dependency.
            // If `respect-rules` is true, DNS may follow proxy rules and try to resolve the proxy server
            // domain via proxy, which is impossible before the proxy is connected.
            map.insert(
                serde_yaml::Value::String("respect-rules".to_string()),
                serde_yaml::Value::Bool(false),
            );

            // Fallback nameservers (DoH/DoT for reliability)
            ensure_sequence!(
                "fallback",
                &[
                    "https://doh.dns.sb/dns-query",
                    "https://dns.cloudflare.com/dns-query",
                    "https://dns.twnic.tw/dns-query",
                    "tls://8.8.4.4:853"
                ]
            );

            // Fallback filter
            let fallback_filter_key = serde_yaml::Value::String("fallback-filter".to_string());
            if map.get(&fallback_filter_key).is_none() {
                let mut filter_map = serde_yaml::Mapping::new();

                filter_map.insert(
                    serde_yaml::Value::String("geoip".to_string()),
                    serde_yaml::Value::Bool(true),
                );

                let mut ipcidr_seq = serde_yaml::Sequence::new();
                ipcidr_seq.push(serde_yaml::Value::String("240.0.0.0/4".to_string()));
                ipcidr_seq.push(serde_yaml::Value::String("0.0.0.0/32".to_string()));
                filter_map.insert(
                    serde_yaml::Value::String("ipcidr".to_string()),
                    serde_yaml::Value::Sequence(ipcidr_seq),
                );

                map.insert(fallback_filter_key, serde_yaml::Value::Mapping(filter_map));
            }

            // --- Critical: prevent proxy server domains from being mapped to Fake-IP ---
            //
            // Symptom:
            //   connect failed: dial tcp 198.18.0.x:443: i/o timeout
            //
            // This usually means the proxy server domain (e.g. api-us-dc1.seckv.com) was resolved
            // to a Fake-IP (198.18.0.0/16). Then the dial tries to connect to a non-existent host.
            //
            // To avoid that, we automatically add all `server` domains from `proxies` into
            // dns.fake-ip-filter, so they always resolve to REAL IPs.
            fn is_ip_literal(s: &str) -> bool {
                s.parse::<std::net::IpAddr>().is_ok()
            }

            fn looks_like_domain(s: &str) -> bool {
                // Heuristic: contains at least one dot and at least one letter.
                s.contains('.') && s.chars().any(|c| c.is_ascii_alphabetic())
            }

            fn collect_proxy_server_domains(root: &serde_yaml::Mapping) -> Vec<String> {
                let mut out = Vec::new();
                let proxies_key = serde_yaml::Value::String("proxies".to_string());
                let Some(serde_yaml::Value::Sequence(items)) = root.get(&proxies_key) else {
                    return out;
                };

                for item in items {
                    let Some(m) = item.as_mapping() else {
                        continue;
                    };
                    let server_key = serde_yaml::Value::String("server".to_string());
                    let Some(server) = m
                        .get(&server_key)
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                    else {
                        continue;
                    };

                    if is_ip_literal(server) {
                        continue;
                    }
                    if looks_like_domain(server) {
                        out.push(server.to_string());
                        // Also add subdomain wildcard form to cover common patterns.
                        out.push(format!("+.{}", server));
                    }
                }

                out.sort();
                out.dedup();
                out
            }

            // Add proxy server domains to fake-ip-filter to prevent them from being resolved to Fake-IPs
            // This is CRITICAL: if proxy server domains get fake-ip, the proxy connection will fail!
            let proxy_domains = collect_proxy_server_domains(root);
            if !proxy_domains.is_empty() {
                let filter_key = serde_yaml::Value::String("fake-ip-filter".to_string());
                let mut seq = match map.get(&filter_key).cloned() {
                    Some(serde_yaml::Value::Sequence(s)) => s,
                    _ => serde_yaml::Sequence::new(),
                };

                let mut existing: std::collections::HashSet<String> = seq
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                let mut added = 0usize;
                for d in proxy_domains {
                    if existing.insert(d.clone()) {
                        seq.push(serde_yaml::Value::String(d));
                        added += 1;
                    }
                }

                if added > 0 {
                    println!(
                        "DNS: Added {} proxy server entries to fake-ip-filter to avoid Fake-IP dialing",
                        added
                    );
                    map.insert(filter_key, serde_yaml::Value::Sequence(seq));
                }
            }
        }
        root.insert(dns_key, dns_value);

        // Set allow-lan: true for TUN mode
        let allow_lan_key = serde_yaml::Value::String("allow-lan".to_string());
        if root.get(&allow_lan_key).is_none() {
            root.insert(allow_lan_key, serde_yaml::Value::Bool(true));
        }

        // Set bind-address: '*' for TUN mode
        let bind_key = serde_yaml::Value::String("bind-address".to_string());
        if root.get(&bind_key).is_none() {
            root.insert(bind_key, serde_yaml::Value::String("*".to_string()));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn set_user_override(key: String, value: serde_json::Value) -> Result<(), String> {
    let mut overrides = load_overrides();

    match key.as_str() {
        "port" => {
            overrides.port = value.as_u64().map(|v| v as u16);
        }
        "socks-port" => {
            overrides.socks_port = value.as_u64().map(|v| v as u16);
        }
        "mixed-port" => {
            overrides.mixed_port = value.as_u64().map(|v| v as u16);
        }
        "redir-port" => {
            overrides.redir_port = value.as_u64().map(|v| v as u16);
        }
        "tproxy-port" => {
            overrides.tproxy_port = value.as_u64().map(|v| v as u16);
        }
        "allow-lan" => {
            overrides.allow_lan = value.as_bool();
        }
        "external-controller" => {
            overrides.external_controller = value.as_str().map(|s| s.to_string());
        }
        key if key.starts_with("tun.") => {
            if overrides.tun.is_none() {
                overrides.tun = Some(TunOverride::default());
            }
            let field = &key[4..];
            let tun = overrides.tun.as_mut().unwrap();
            match field {
                "enable" => {
                    if value.is_null() {
                        tun.enable = None;
                    } else {
                        tun.enable = value.as_bool();
                    }
                }
                "stack" => {
                    if value.is_null() {
                        tun.stack = None;
                    } else if let Some(val) = value.as_str() {
                        tun.stack = Some(val.to_string());
                    } else {
                        return Err("tun.stack expects a string".to_string());
                    }
                }
                "device-id" => {
                    if value.is_null() {
                        tun.device_id = None;
                    } else if let Some(val) = value.as_str() {
                        tun.device_id = Some(val.to_string());
                    } else {
                        return Err("tun.device-id expects a string".to_string());
                    }
                }
                "mtu" => {
                    if value.is_null() {
                        tun.mtu = None;
                    } else if let Some(num) = value.as_u64() {
                        if num > u16::MAX as u64 {
                            return Err("tun.mtu must be <= 65535".to_string());
                        }
                        tun.mtu = Some(num as u16);
                    } else {
                        return Err("tun.mtu expects a positive integer".to_string());
                    }
                }
                "strict-route" => {
                    if value.is_null() {
                        tun.strict_route = None;
                    } else if let Some(val) = value.as_bool() {
                        tun.strict_route = Some(val);
                    } else {
                        return Err("tun.strict-route expects a boolean".to_string());
                    }
                }
                "auto-route" => {
                    if value.is_null() {
                        tun.auto_route = None;
                    } else if let Some(val) = value.as_bool() {
                        tun.auto_route = Some(val);
                    } else {
                        return Err("tun.auto-route expects a boolean".to_string());
                    }
                }
                "auto-detect-interface" => {
                    if value.is_null() {
                        tun.auto_detect_interface = None;
                    } else if let Some(val) = value.as_bool() {
                        tun.auto_detect_interface = Some(val);
                    } else {
                        return Err("tun.auto-detect-interface expects a boolean".to_string());
                    }
                }
                "dns-hijack" => {
                    if value.is_null() {
                        tun.dns_hijack = None;
                    } else if let Some(entries) = value.as_array() {
                        let mut list = Vec::with_capacity(entries.len());
                        for entry in entries {
                            if let Some(val) = entry.as_str() {
                                list.push(val.to_string());
                            } else {
                                return Err("tun.dns-hijack entries must be strings".to_string());
                            }
                        }
                        tun.dns_hijack = Some(list);
                    } else {
                        return Err("tun.dns-hijack expects an array of strings".to_string());
                    }
                }
                _ => return Err(format!("Unknown TUN override key: {}", key)),
            }
        }
        _ => return Err(format!("Unknown override key: {}", key)),
    }

    save_overrides(&overrides)?;
    Ok(())
}

#[tauri::command]
pub fn get_user_overrides() -> Result<UserConfigOverrides, String> {
    Ok(load_overrides())
}

#[tauri::command]
pub fn clear_user_overrides() -> Result<(), String> {
    save_overrides(&UserConfigOverrides::default())
}

/// Persist the latest TUN enable preference so UI stays consistent with runtime changes
pub fn persist_tun_override(enable: bool) -> Result<(), String> {
    println!("persist_tun_override: Setting TUN enable to {}", enable);
    let mut overrides = load_overrides();
    if overrides.tun.is_none() {
        println!("persist_tun_override: Creating new TUN override");
        overrides.tun = Some(TunOverride::default());
    }
    if let Some(ref mut tun) = overrides.tun {
        tun.enable = Some(enable);
        println!("persist_tun_override: TUN enable set to {:?}", tun.enable);

        // When enabling TUN, ensure essential parameters are set for it to work
        if enable {
            // macOS: align with clash-verge defaults for stability
            // - gvisor stack is generally more reliable than system stack
            // - strict-route may break LAN/DIRECT flows in some setups
            tun.stack = Some("Mixed".to_string());
            // auto-route: must be true for traffic to go through TUN
            if tun.auto_route.is_none() {
                tun.auto_route = Some(true);
            }
            // auto-detect-interface: helps with network interface detection
            if tun.auto_detect_interface.is_none() {
                tun.auto_detect_interface = Some(true);
            }
            tun.strict_route = Some(false);
            // dns-hijack: required for DNS resolution through TUN
            if tun.dns_hijack.is_none() {
                tun.dns_hijack = Some(vec!["any:53".to_string(), "tcp://any:53".to_string()]);
                println!("TUN mode: Setting default dns-hijack: any:53, tcp://any:53");
            }
        }
    }
    let result = save_overrides(&overrides);
    if result.is_ok() {
        println!("persist_tun_override: Successfully saved overrides to disk");
    } else {
        println!("persist_tun_override: ERROR saving overrides: {:?}", result);
    }
    result
}

/// Persist core mode preference ("user" or "service") for next app launch
pub fn persist_core_mode(mode: &str) -> Result<(), String> {
    let mut overrides = load_overrides();
    overrides.core_mode = Some(mode.to_string());
    save_overrides(&overrides)
}

/// Get persisted core mode preference
pub fn get_persisted_core_mode() -> Option<String> {
    load_overrides().core_mode
}
