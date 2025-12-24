# CI 配置说明

## 当前状态

CI/CD 配置已更新为**仅编译 macOS 平台**。

### 已注释的内容

以下 Windows 相关配置已被注释:

1. **构建矩阵** (`.github/workflows/ci.yml:83-88`)
   ```yaml
   # Windows 编译暂时注释
   # - name: Windows (x64)
   #   os: windows-latest
   #   target: x86_64-pc-windows-msvc
   #   bundles: msi,nsis
   #   rust_target: x86_64-pc-windows-msvc
   ```

2. **Windows 依赖安装** (`.github/workflows/ci.yml:126-131`)
   ```yaml
   # Windows 特定设置 (暂时注释)
   # - name: Install Windows dependencies
   #   if: matrix.platform.os == 'windows-latest'
   #   run: |
   #     echo "WebView2 will be installed via Tauri"
   ```

3. **Windows 产物上传** (`.github/workflows/ci.yml:162-170`)
   ```yaml
   # Windows 产物上传 (暂时注释)
   # - name: Upload artifacts (Windows)
   #   if: matrix.platform.os == 'windows-latest'
   #   uses: actions/upload-artifact@v4
   #   with:
   #     name: aqiu-windows-x64
   #     path: |
   #       src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/*.msi
   #       src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe
   ```

4. **发布步骤中的 Windows 产物** (`.github/workflows/ci.yml:203-204`)
   ```yaml
   # Windows 产物暂时注释
   # artifacts/**/*.msi
   # artifacts/**/*.exe
   ```

### 当前编译平台

- ✅ **macOS (Apple Silicon)** - aarch64-apple-darwin
  - 产物: DMG 和 APP

### 如何恢复 Windows 编译

如需恢复 Windows 编译,只需取消注释以上四处代码即可:

1. 取消注释构建矩阵中的 Windows 配置
2. 取消注释 Windows 依赖安装步骤
3. 取消注释 Windows 产物上传步骤
4. 取消注释发布步骤中的 Windows 产物

### 代码状态

虽然 CI 中注释了 Windows 编译,但 Windows 平台的代码实现仍然保留:

- ✅ Windows 系统代理实现 (`src-tauri/src/core/windows.rs`)
- ✅ Windows IPC 通信 (`crates/aqiu-service-ipc/src/client.rs`)
- ✅ 跨平台抽象层

这些代码在本地开发时仍然可用,只是 CI 暂时不会构建 Windows 版本。

### 本地开发

在 Windows 上进行本地开发时,代码仍然可以正常编译和运行:

```bash
# Windows 本地开发
cargo check --manifest-path=./src-tauri/Cargo.toml
bun run tauri dev
bun run tauri build
```

### 注意事项

- CI 现在只会在 macOS 上运行测试和构建
- 发布时只会包含 macOS 产物
- Windows 代码实现保持不变,随时可以恢复编译
