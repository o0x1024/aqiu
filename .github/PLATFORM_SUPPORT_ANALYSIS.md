# AQiu 平台支持分析

## 概述

本文档分析 AQiu 应用在不同平台上的可用性状态,并提供完善 Windows 和 iOS 平台功能的实施计划。

## 当前平台支持状态

### ✅ macOS (完全支持)

**状态**: 完全可用,所有功能已实现

**已实现功能**:
- ✅ 基础核心进程管理(启动/停止/重启)
- ✅ 系统代理设置
- ✅ TUN 模式(需要特权)
- ✅ 特权服务模式(LaunchDaemon)
- ✅ 孤儿进程恢复
- ✅ 配置文件管理
- ✅ 实时流量监控
- ✅ 连接管理
- ✅ 日志查看
- ✅ 托盘图标和菜单
- ✅ 自动启动

**特有功能**:
- LaunchDaemon 服务模式(以 root 权限运行)
- 特权助手安装/卸载
- macOS 系统代理 API 集成
- 孤儿进程检测和恢复

---

### ⚠️ Windows (部分支持 - 需要完善)

**状态**: 基础功能可用,但缺少关键平台特定功能

**已实现功能**:
- ✅ 基础核心进程管理(用户模式)
- ✅ 配置文件管理
- ✅ 实时流量监控
- ✅ 连接管理
- ✅ 日志查看
- ✅ 托盘图标和菜单

**缺失功能**:
- ❌ **系统代理设置** - 需要实现 Windows Registry API
- ❌ **TUN 模式支持** - 需要 Windows 驱动或 WinTun
- ❌ **Windows 服务模式** - 需要实现 Windows Service
- ❌ **IPC 通信** - 当前仅支持 Unix Socket
- ❌ **端口清理** - 当前实现仅支持 macOS/Linux
- ❌ **进程检测** - 需要 Windows 特定实现

**技术限制**:
1. **IPC 通信**: 当前使用 Unix Socket,Windows 需要使用 Named Pipes 或 TCP
2. **系统代理**: 需要修改 Windows Registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`)
3. **TUN 模式**: 需要集成 WinTun 驱动或使用 Windows TAP 驱动
4. **服务模式**: 需要实现 Windows Service,使用 `windows-service` crate
5. **进程管理**: 需要使用 Windows API 检测进程和端口占用

---

### ❌ iOS (不可用 - 需要重大架构调整)

**状态**: 编译可能成功,但核心功能无法运行

**技术限制**:
1. **无法运行外部进程**: iOS 沙盒不允许 `std::process::Command`
2. **无网络扩展权限**: TUN/VPN 需要 Network Extension
3. **无系统代理 API**: iOS 不提供系统级代理设置
4. **无后台服务**: iOS 不支持传统的后台守护进程
5. **应用沙盒限制**: 无法访问系统级配置

**可行的替代方案**:
- **Network Extension**: 使用 iOS Network Extension Framework
- **内嵌核心**: 将 Mihomo 编译为库而非独立进程
- **VPN 配置**: 通过 NEVPNManager 配置 VPN
- **本地 HTTP 代理**: 仅提供应用内代理,无系统级支持

**实施难度**: 极高 - 需要完全重写核心架构

---

## 实施计划

### 阶段 1: Windows 平台完善 (优先级: 高)

#### 1.1 IPC 通信层 (Windows Named Pipes)

**文件**: `crates/aqiu-service-ipc/src/`

**任务**:
- [ ] 实现 Windows Named Pipes 客户端
- [ ] 实现 Windows Named Pipes 服务端
- [ ] 统一 IPC 接口,支持条件编译

**实现示例**:
```rust
#[cfg(windows)]
use tokio::net::windows::named_pipe;

#[cfg(windows)]
pub async fn connect() -> IpcResult<NamedPipeClient> {
    let pipe_name = r"\\.\pipe\aqiu-service";
    // 实现 Windows Named Pipe 连接
}
```

#### 1.2 系统代理设置 (Windows Registry)

**文件**: `src-tauri/src/core/proxy_and_mode.rs`

**任务**:
- [ ] 实现 Windows Registry 读写
- [ ] 设置 `ProxyEnable` 和 `ProxyServer`
- [ ] 通知系统代理设置已更改

**实现示例**:
```rust
#[cfg(target_os = "windows")]
pub async fn set_system_proxy(
    _app: tauri::AppHandle,
    enabled: bool,
    proxy_server: Option<String>,
) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_WRITE,
    ).map_err(|e| e.to_string())?;
    
    if enabled {
        let proxy = proxy_server.unwrap_or_else(|| "127.0.0.1:10809".to_string());
        internet_settings.set_value("ProxyEnable", &1u32).map_err(|e| e.to_string())?;
        internet_settings.set_value("ProxyServer", &proxy).map_err(|e| e.to_string())?;
    } else {
        internet_settings.set_value("ProxyEnable", &0u32).map_err(|e| e.to_string())?;
    }
    
    // 通知系统刷新代理设置
    unsafe {
        winapi::um::wininet::InternetSetOptionW(
            std::ptr::null_mut(),
            winapi::um::wininet::INTERNET_OPTION_SETTINGS_CHANGED,
            std::ptr::null_mut(),
            0,
        );
    }
    
    Ok(())
}
```

#### 1.3 进程和端口管理 (Windows API)

**文件**: `src-tauri/src/core/base.rs`

**任务**:
- [ ] 实现 Windows 进程检测
- [ ] 实现端口占用检测
- [ ] 实现端口清理功能

**实现示例**:
```rust
#[cfg(target_os = "windows")]
fn is_port_in_use(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
}

#[cfg(target_os = "windows")]
fn find_mihomo_pid_by_port(port: u16) -> Option<u32> {
    // 使用 netstat 或 Windows API 查找进程
    let output = std::process::Command::new("netstat")
        .args(["-ano"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(&format!(":{}", port)) && line.contains("LISTENING") {
            // 解析 PID
            if let Some(pid_str) = line.split_whitespace().last() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }
    None
}
```

#### 1.4 TUN 模式支持 (WinTun)

**文件**: `src-tauri/src/core/tun.rs`

**任务**:
- [ ] 集成 WinTun 驱动
- [ ] 实现 TUN 设备创建和配置
- [ ] 实现路由表管理

**依赖**:
```toml
[target.'cfg(windows)'.dependencies]
wintun = "0.4"
```

#### 1.5 Windows 服务模式 (可选)

**文件**: `crates/aqiu-service/`

**任务**:
- [ ] 使用 `windows-service` crate 实现 Windows Service
- [ ] 实现服务安装/卸载
- [ ] 实现服务启动/停止

---

### 阶段 2: iOS 平台评估 (优先级: 低)

#### 2.1 可行性分析

**问题**:
1. iOS 不允许运行外部进程
2. 需要 Network Extension 权限
3. 需要 Apple Developer Program 账号
4. 需要重写核心架构

**建议**:
- **短期**: 暂不支持 iOS,专注于 macOS 和 Windows
- **长期**: 考虑创建独立的 iOS 版本,使用 Network Extension

#### 2.2 替代方案 (如果必须支持)

**方案 A: Network Extension**
- 使用 Swift 编写 Network Extension
- 将 Mihomo 编译为静态库
- 通过 XPC 与主应用通信

**方案 B: 仅配置管理**
- iOS 版本仅用于管理配置
- 实际代理运行在远程服务器
- 通过 API 控制远程代理

---

## 依赖项更新

### Windows 平台新增依赖

```toml
[target.'cfg(windows)'.dependencies]
winreg = "0.52"
winapi = { version = "0.3", features = ["wininet", "winsock2"] }
wintun = "0.4"  # TUN 模式
windows-service = "0.7"  # 服务模式
```

### 跨平台 IPC 依赖

```toml
[dependencies]
# Unix
[target.'cfg(unix)'.dependencies]
tokio = { version = "1", features = ["net"] }

# Windows
[target.'cfg(windows)'.dependencies]
tokio = { version = "1", features = ["net", "io-util"] }
```

---

## 测试计划

### Windows 测试

- [ ] 基础核心启动/停止
- [ ] 系统代理开启/关闭
- [ ] 配置文件加载
- [ ] 托盘菜单功能
- [ ] 进程清理
- [ ] TUN 模式(如果实现)

### iOS 测试 (如果实现)

- [ ] 应用启动
- [ ] UI 渲染
- [ ] 配置管理
- [ ] Network Extension 连接

---

## 时间估算

| 任务 | 优先级 | 估算时间 | 难度 |
|------|--------|----------|------|
| Windows IPC (Named Pipes) | 高 | 3-5 天 | 中 |
| Windows 系统代理 | 高 | 2-3 天 | 低 |
| Windows 进程管理 | 高 | 2-3 天 | 中 |
| Windows 端口清理 | 中 | 1-2 天 | 低 |
| Windows TUN 模式 | 中 | 5-7 天 | 高 |
| Windows 服务模式 | 低 | 5-7 天 | 高 |
| iOS 可行性评估 | 低 | 2-3 天 | - |
| iOS Network Extension | 低 | 14-21 天 | 极高 |

**Windows 完整支持总计**: 约 18-27 天  
**iOS 基础支持总计**: 约 16-24 天

---

## 建议

1. **优先完善 Windows 平台**: Windows 用户基数大,实施难度适中
2. **暂缓 iOS 支持**: iOS 需要重大架构调整,投入产出比低
3. **保持 macOS 稳定性**: macOS 是当前最成熟的平台,继续优化
4. **分阶段实施**: 先实现 Windows 基础功能,再考虑高级功能

---

## 结论

- **macOS**: ✅ 完全可用,无需额外工作
- **Windows**: ⚠️ 需要完善,但可行且值得投入
- **iOS**: ❌ 不建议支持,除非有特殊需求和资源

建议优先完成 Windows 平台的核心功能(IPC、系统代理、进程管理),使其达到与 macOS 相当的可用性水平。
