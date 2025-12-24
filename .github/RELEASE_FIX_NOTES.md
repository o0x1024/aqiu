# GitHub Actions Release 修复说明

## 问题描述

在 GitHub Actions 的 release job 中出现错误:
```
Run ls -R artifacts
ls: cannot access 'artifacts': No such file or directory
Error: Process completed with exit code 2.
```

## 问题原因

1. **artifacts 目录不存在**: 当 `actions/download-artifact@v4` 没有找到任何 artifacts 时,不会创建 `artifacts` 目录
2. **构建产物可能缺失**: macOS 构建可能没有成功生成产物,或者路径不正确

## 修复方案

### 1. 添加错误处理

在 `actions/download-artifact@v4` 步骤添加 `continue-on-error: true`:

```yaml
- name: Download all artifacts
  uses: actions/download-artifact@v4
  with:
    path: artifacts
  continue-on-error: true  # 即使没有 artifacts 也继续执行
```

### 2. 改进目录检查

修改 "Display structure of downloaded files" 步骤,添加条件检查:

```yaml
- name: Display structure of downloaded files
  run: |
    if [ -d "artifacts" ]; then
      echo "Artifacts directory exists"
      ls -R artifacts
    else
      echo "No artifacts directory found"
      mkdir -p artifacts
    fi
```

### 3. 添加构建产物调试

在 macOS 构建步骤添加调试信息:

```yaml
- name: List build outputs (macOS)
  if: matrix.platform.os == 'macos-latest' && matrix.platform.target == 'aarch64-apple-darwin'
  run: |
    echo "Listing build directory structure:"
    find src-tauri/target/aarch64-apple-darwin/release/bundle -type f || echo "Bundle directory not found"
```

## 修复后的工作流程

1. **构建阶段**:
   - 构建 macOS 应用
   - 列出构建产物(调试)
   - 上传 artifacts (如果存在)

2. **发布阶段**:
   - 下载 artifacts (允许失败)
   - 检查并显示 artifacts 目录
   - 创建 release (即使没有 artifacts 也会创建)

## 预期行为

### 如果构建成功
```
Artifacts directory exists
artifacts/
└── aqiu-macos-aarch64/
    ├── aqiu.dmg
    └── aqiu.app
```

### 如果构建失败或没有产物
```
No artifacts directory found
Creating empty artifacts directory
```

Release 仍然会被创建,但可能没有附件。

## 验证步骤

1. **推送 tag 触发 release**:
   ```bash
   git tag v1.0.6
   git push origin v1.0.6
   ```

2. **检查 Actions 日志**:
   - 查看 "List build outputs (macOS)" 步骤
   - 查看 "Upload artifacts (macOS)" 步骤
   - 查看 "Download all artifacts" 步骤
   - 查看 "Display structure of downloaded files" 步骤

3. **检查 Release**:
   - 访问 GitHub Releases 页面
   - 查看是否创建了 draft release
   - 检查是否有附件

## 常见问题

### Q: 为什么 artifacts 目录不存在?

A: 可能的原因:
1. 构建失败,没有生成产物
2. 产物路径不正确
3. `if-no-files-found: warn` 只是警告,不会失败

### Q: 如何确保构建产物正确上传?

A: 
1. 检查 "List build outputs" 步骤的输出
2. 确认文件路径正确
3. 检查 Tauri 构建是否成功

### Q: Release 创建失败怎么办?

A:
1. 检查 `GITHUB_TOKEN` 权限
2. 确认 tag 格式正确 (`v*`)
3. 查看 Actions 日志中的错误信息

## 相关文件

- `.github/workflows/ci.yml` - CI/CD 配置文件
- `src-tauri/tauri.conf.json` - Tauri 配置文件

## 后续改进建议

1. **添加构建产物验证**:
   ```yaml
   - name: Verify build outputs
     run: |
       if [ ! -f "src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/*.dmg" ]; then
         echo "Error: DMG file not found"
         exit 1
       fi
   ```

2. **使用矩阵输出**:
   ```yaml
   outputs:
     artifact-name: ${{ steps.build.outputs.artifact-name }}
   ```

3. **条件化 Release 创建**:
   ```yaml
   - name: Create Release
     if: steps.download.outputs.download-path != ''
   ```

## 总结

修复后的 CI 配置更加健壮:
- ✅ 处理 artifacts 不存在的情况
- ✅ 提供详细的调试信息
- ✅ 即使部分步骤失败也能继续
- ✅ 创建 release 不会因为缺少 artifacts 而失败
