# GitHub Actions Secrets 配置指南

本文档说明如何配置 GitHub Actions 所需的 Secrets,以便在 CI/CD 中构建和签名应用。

## 必需的 Secrets

### 1. APPLE_CERTIFICATE (必需,用于 macOS 签名)

**说明**: Base64 编码的 Apple 开发者证书 (.p12 文件)

**获取步骤**:
```bash
# 1. 将证书转换为 Base64
base64 -i ~/Documents/o0x1024_apple_dev_cert.p12 | pbcopy

# 2. 内容已复制到剪贴板,直接粘贴到 GitHub Secrets
```

**GitHub 设置路径**:
1. 打开仓库: https://github.com/你的用户名/aqiu
2. Settings → Secrets and variables → Actions
3. New repository secret
4. Name: `APPLE_CERTIFICATE`
5. Value: 粘贴 Base64 内容

---

### 2. APPLE_CERTIFICATE_PASSWORD (必需)

**说明**: 导出 .p12 证书时设置的密码

**获取**: 这是你在导出证书时设置的密码,如果忘记了需要重新导出证书

**GitHub 设置**:
- Name: `APPLE_CERTIFICATE_PASSWORD`
- Value: 你的证书密码

---

### 3. APPLE_SIGNING_IDENTITY (必需)

**说明**: 用于签名的证书身份名称

**获取**:
```bash
security find-identity -v -p codesigning
```

**从你的输出中选择**:
- 推荐使用: `Apple Development: o0x1024@gmail.com (5AGZ74SY29)`
- 或使用 SHA-1: `63A56C5369A4D012909EF64BAB734719CA20E924`

**GitHub 设置**:
- Name: `APPLE_SIGNING_IDENTITY`
- Value: `Apple Development: o0x1024@gmail.com (5AGZ74SY29)`

---

### 4. APPLE_TEAM_ID (必需)

**说明**: Apple Developer Team ID

**从你的证书中提取**:
- 从 `(5AGZ74SY29)` 或 `(F4CQNHZG62)` 中选择
- 推荐使用: `5AGZ74SY29`

**GitHub 设置**:
- Name: `APPLE_TEAM_ID`
- Value: `5AGZ74SY29`

---

## 可选的 Secrets (用于公证 Notarization)

### 5. APPLE_ID (可选)

**说明**: Apple Developer 账号邮箱

**GitHub 设置**:
- Name: `APPLE_ID`
- Value: `o0x1024@gmail.com`

---

### 6. APPLE_PASSWORD (可选)

**说明**: App 专用密码 (不是 Apple ID 密码)

**获取步骤**:
1. 访问: https://appleid.apple.com
2. 登录后进入 "安全" 部分
3. "App 专用密码" → "生成密码"
4. 输入标签: "GitHub Actions"
5. 复制生成的密码 (格式: xxxx-xxxx-xxxx-xxxx)

**GitHub 设置**:
- Name: `APPLE_PASSWORD`
- Value: 生成的 App 专用密码

---

## 可选的 Secrets (用于 Tauri 更新器)

### 7. TAURI_SIGNING_PRIVATE_KEY (可选)

**说明**: Tauri 更新器签名私钥

**生成步骤**:
```bash
# 安装 Tauri CLI (如果还没安装)
cargo install tauri-cli

# 生成密钥对
tauri signer generate -w ~/.tauri/aqiu.key

# 查看私钥内容
cat ~/.tauri/aqiu.key
```

**GitHub 设置**:
- Name: `TAURI_SIGNING_PRIVATE_KEY`
- Value: 私钥文件的完整内容

**注意**: 还需要将公钥 (`aqiu.key.pub`) 添加到 `tauri.conf.json` 的 `updater.pubkey` 字段

---

### 8. TAURI_SIGNING_PRIVATE_KEY_PASSWORD (可选)

**说明**: 生成 Tauri 签名密钥时设置的密码

**GitHub 设置**:
- Name: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- Value: 密钥密码 (如果生成时没设置密码,可以留空或不设置此 secret)

---

## 快速配置检查清单

### 最小配置 (必需,用于构建签名的 macOS 应用):
- [x] APPLE_CERTIFICATE
- [x] APPLE_CERTIFICATE_PASSWORD
- [x] APPLE_SIGNING_IDENTITY
- [x] APPLE_TEAM_ID

### 完整配置 (推荐,包含公证):
- [x] APPLE_CERTIFICATE
- [x] APPLE_CERTIFICATE_PASSWORD
- [x] APPLE_SIGNING_IDENTITY
- [x] APPLE_TEAM_ID
- [x] APPLE_ID
- [x] APPLE_PASSWORD

### 更新器配置 (可选):
- [ ] TAURI_SIGNING_PRIVATE_KEY
- [ ] TAURI_SIGNING_PRIVATE_KEY_PASSWORD

---

## 注意事项

1. **证书类型**: 
   - 当前使用的是 **Apple Development** 证书,适用于开发和测试
   - 如需公开分发,建议申请 **Developer ID Application** 证书

2. **证书有效期**: 
   - Apple Development 证书有效期为 1 年
   - 过期后需要重新生成并更新 GitHub Secrets

3. **安全性**: 
   - 这些 Secrets 包含敏感信息,请勿公开分享
   - GitHub Secrets 是加密存储的,只有仓库管理员可以修改

4. **测试**: 
   - 配置完成后,可以手动触发 GitHub Actions 测试
   - 或者推送一个 tag (例如 `v0.1.0`) 触发发布流程

---

## 验证配置

配置完成后,可以通过以下方式验证:

1. 在 GitHub 仓库中进入 Actions 标签页
2. 手动运行 workflow (如果启用了 `workflow_dispatch`)
3. 或推送代码触发 CI
4. 查看构建日志,确认签名步骤成功

---

## 故障排除

### 问题: "SecKeychainItemImport: One or more parameters passed to a function were not valid"

**原因**: 证书密码错误或证书格式不正确

**解决**:
1. 确认 `APPLE_CERTIFICATE_PASSWORD` 正确
2. 重新导出证书并转换为 Base64
3. 确保证书是 .p12 格式

### 问题: "No identity found"

**原因**: `APPLE_SIGNING_IDENTITY` 与证书不匹配

**解决**:
1. 运行 `security find-identity -v -p codesigning` 查看可用身份
2. 复制完整的身份名称到 `APPLE_SIGNING_IDENTITY`

---

## 相关链接

- [Apple Developer Portal](https://developer.apple.com/account)
- [Tauri 签名文档](https://tauri.app/v1/guides/distribution/sign-macos)
- [GitHub Actions Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
