# DNS 配置说明

## 支持的 DNS 类型

### UDP
- `223.5.5.5`
- `udp://223.5.5.5`

### TCP
- `tcp://8.8.8.8`

### DNS over TLS (DoT)
- `tls://1.1.1.1`
- `tls://8.8.4.4`

### DNS over HTTPS (DoH)
- `https://doh.pub/dns-query`
- `https://dns.alidns.com/dns-query`
- `https://1.1.1.1/dns-query`

### DNS over QUIC (DoQ)
- `quic://dns.adguard.com:784`

### System DNS
- `system://`
- `system`

### DHCP DNS
- `dhcp://en0`
- `dhcp://system` (仅限 cmfa)

### RCode (用于测试)
- `rcode://success` - No error
- `rcode://format_error` - Format error
- `rcode://server_failure` - Server failure
- `rcode://name_error` - Non-existent domain
- `rcode://not_implemented` - Not implemented
- `rcode://refused` - Query refused

## TUN 模式下的 DNS 配置

本应用在启用 TUN 模式时，会自动应用以下 DNS 配置：

### 基础设置
- `enable: true` - 启用 DNS 模块
- `listen: 0.0.0.0:1053` - DNS 服务监听地址
- `cache-algorithm: arc` - 使用 ARC 缓存算法
- `prefer-h3: false` - 不优先使用 HTTP/3
- `use-hosts: true` - 使用 hosts 文件
- `use-system-hosts: true` - 使用系统 hosts
- `respect-rules: false` - 不遵循规则集的 DNS 设置
- `ipv6: false` - 禁用 IPv6 (提高解析速度)

### Enhanced Mode (增强模式)
- `enhanced-mode: fake-ip` - 使用 Fake-IP 模式
- `fake-ip-range: 198.18.0.1/16` - Fake-IP 地址池
- `fake-ip-filter-mode: blacklist` - 黑名单模式

### DNS 服务器配置

#### default-nameserver (默认域名服务器)
用于解析 DoH/DoT 等加密 DNS 的地址：
- `223.5.5.5` - 阿里 DNS
- `119.29.29.29` - 腾讯 DNS
- `1.1.1.1` - Cloudflare DNS

#### nameserver (主域名服务器)
使用 DoH 提供隐私保护：
- `https://doh.pub/dns-query` - 腾讯 DoH
- `https://dns.alidns.com/dns-query` - 阿里 DoH

#### fallback (备用域名服务器)
使用 DoT 提供可靠性：
- `tls://8.8.4.4` - Google DoT
- `tls://1.1.1.1` - Cloudflare DoT

#### proxy-server-nameserver (代理服务器专用)
用于解析代理服务器地址：
- `https://doh.pub/dns-query`

#### direct-nameserver (直连专用)
用于直连域名解析：
- `system` - 使用系统 DNS

### Fallback Filter (备用 DNS 触发条件)
当主 DNS 返回以下情况时，使用备用 DNS：
- `geoip: true` - 启用 GeoIP 判断
- `geoip-code: CN` - 国内 IP 不使用备用
- `geosite: [gfw]` - GFW 列表中的域名使用备用
- `ipcidr: [240.0.0.0/4]` - 保留地址段使用备用
- `domain: [+.google.com, +.facebook.com, +.youtube.com]` - 特定域名使用备用

### Fake-IP Filter (不使用 Fake-IP 的域名)
以下域名使用真实 IP：
- `*.lan` - 局域网域名
- `*.local` - 本地域名
- `localhost.ptlogin2.qq.com` - QQ 登录
- `+.stun.*.*` - STUN 服务器
- `*.msftconnecttest.com` - Windows 网络检测
- `*.msftncsi.com` - Windows 网络状态指示器
- `WORKGROUP` - Windows 工作组

## 配置优势

1. **隐私保护**：使用 DoH/DoT 加密 DNS 查询
2. **智能分流**：国内域名使用国内 DNS，国外域名使用国外 DNS
3. **高可用性**：多组 DNS 互为备份
4. **低延迟**：优先使用国内 DNS 服务器