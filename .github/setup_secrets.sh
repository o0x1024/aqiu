#!/bin/bash

# GitHub Secrets 快速配置脚本
# 此脚本帮助你准备需要上传到 GitHub Secrets 的值

set -e

echo "=================================="
echo "GitHub Secrets 配置助手"
echo "=================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 输出目录
OUTPUT_DIR="/tmp/github_secrets"
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}输出目录: $OUTPUT_DIR${NC}"
echo ""

# 1. 转换证书为 Base64
echo "=================================="
echo "1. Apple 证书配置"
echo "=================================="

CERT_PATH="$HOME/Documents/o0x1024_apple_dev_cert.p12"

if [ -f "$CERT_PATH" ]; then
    echo -e "${GREEN}✓ 找到证书: $CERT_PATH${NC}"
    
    # 转换为 Base64
    base64 -i "$CERT_PATH" > "$OUTPUT_DIR/APPLE_CERTIFICATE.txt"
    echo -e "${GREEN}✓ 证书已转换为 Base64${NC}"
    echo -e "  保存位置: ${YELLOW}$OUTPUT_DIR/APPLE_CERTIFICATE.txt${NC}"
    
    # 提示输入密码
    echo ""
    echo -e "${YELLOW}请输入证书密码 (导出 .p12 时设置的密码):${NC}"
    read -s CERT_PASSWORD
    echo "$CERT_PASSWORD" > "$OUTPUT_DIR/APPLE_CERTIFICATE_PASSWORD.txt"
    echo -e "${GREEN}✓ 证书密码已保存${NC}"
    echo -e "  保存位置: ${YELLOW}$OUTPUT_DIR/APPLE_CERTIFICATE_PASSWORD.txt${NC}"
else
    echo -e "${YELLOW}⚠ 未找到证书文件: $CERT_PATH${NC}"
    echo "  请手动指定证书路径或跳过此步骤"
fi

echo ""

# 2. 获取签名身份
echo "=================================="
echo "2. 签名身份配置"
echo "=================================="

echo "正在查找可用的签名身份..."
IDENTITIES=$(security find-identity -v -p codesigning 2>/dev/null | grep "Apple Development")

if [ -n "$IDENTITIES" ]; then
    echo -e "${GREEN}✓ 找到以下签名身份:${NC}"
    echo "$IDENTITIES"
    echo ""
    
    # 提取第一个身份
    FIRST_IDENTITY=$(echo "$IDENTITIES" | head -1 | sed 's/.*"\(.*\)"/\1/')
    echo "$FIRST_IDENTITY" > "$OUTPUT_DIR/APPLE_SIGNING_IDENTITY.txt"
    echo -e "${GREEN}✓ 推荐使用:${NC} $FIRST_IDENTITY"
    echo -e "  保存位置: ${YELLOW}$OUTPUT_DIR/APPLE_SIGNING_IDENTITY.txt${NC}"
    
    # 提取 Team ID
    TEAM_ID=$(echo "$FIRST_IDENTITY" | grep -o '([A-Z0-9]\{10\})' | tr -d '()')
    if [ -n "$TEAM_ID" ]; then
        echo "$TEAM_ID" > "$OUTPUT_DIR/APPLE_TEAM_ID.txt"
        echo -e "${GREEN}✓ Team ID:${NC} $TEAM_ID"
        echo -e "  保存位置: ${YELLOW}$OUTPUT_DIR/APPLE_TEAM_ID.txt${NC}"
    fi
else
    echo -e "${YELLOW}⚠ 未找到签名身份${NC}"
fi

echo ""

# 3. Apple ID 配置
echo "=================================="
echo "3. Apple ID 配置 (可选,用于公证)"
echo "=================================="

echo -e "${YELLOW}请输入 Apple ID (邮箱) [留空跳过]:${NC}"
read APPLE_ID

if [ -n "$APPLE_ID" ]; then
    echo "$APPLE_ID" > "$OUTPUT_DIR/APPLE_ID.txt"
    echo -e "${GREEN}✓ Apple ID 已保存${NC}"
    echo -e "  保存位置: ${YELLOW}$OUTPUT_DIR/APPLE_ID.txt${NC}"
    
    echo ""
    echo -e "${YELLOW}请输入 App 专用密码 (不是 Apple ID 密码) [留空跳过]:${NC}"
    echo "  获取方式: https://appleid.apple.com → 安全 → App 专用密码"
    read -s APPLE_PASSWORD
    
    if [ -n "$APPLE_PASSWORD" ]; then
        echo "$APPLE_PASSWORD" > "$OUTPUT_DIR/APPLE_PASSWORD.txt"
        echo -e "${GREEN}✓ App 专用密码已保存${NC}"
        echo -e "  保存位置: ${YELLOW}$OUTPUT_DIR/APPLE_PASSWORD.txt${NC}"
    fi
else
    echo "跳过 Apple ID 配置"
fi

echo ""

# 4. 生成摘要
echo "=================================="
echo "配置摘要"
echo "=================================="

echo ""
echo -e "${GREEN}已生成以下 Secrets 配置文件:${NC}"
echo ""

for file in "$OUTPUT_DIR"/*.txt; do
    if [ -f "$file" ]; then
        filename=$(basename "$file" .txt)
        echo -e "  ${BLUE}$filename${NC}"
        echo -e "    文件: $file"
        
        # 显示内容预览 (除了密码和证书)
        if [[ ! "$filename" =~ PASSWORD|CERTIFICATE ]]; then
            content=$(cat "$file")
            echo -e "    值: ${YELLOW}$content${NC}"
        else
            echo -e "    值: ${YELLOW}[已加密保存]${NC}"
        fi
        echo ""
    fi
done

echo ""
echo "=================================="
echo "下一步操作"
echo "=================================="
echo ""
echo "1. 访问 GitHub 仓库设置:"
echo -e "   ${BLUE}https://github.com/你的用户名/aqiu/settings/secrets/actions${NC}"
echo ""
echo "2. 点击 'New repository secret'"
echo ""
echo "3. 逐个添加 Secrets:"
echo "   - Name: 使用文件名 (不含 .txt)"
echo "   - Value: 复制文件内容"
echo ""
echo "4. 可以使用以下命令快速复制内容到剪贴板:"
echo ""

for file in "$OUTPUT_DIR"/*.txt; do
    if [ -f "$file" ]; then
        filename=$(basename "$file" .txt)
        echo -e "   ${YELLOW}cat $file | pbcopy${NC}  # 复制 $filename"
    fi
done

echo ""
echo "=================================="
echo "安全提示"
echo "=================================="
echo ""
echo -e "${YELLOW}⚠ 这些文件包含敏感信息,请在配置完成后删除:${NC}"
echo -e "   ${YELLOW}rm -rf $OUTPUT_DIR${NC}"
echo ""

echo -e "${GREEN}✓ 配置准备完成!${NC}"
