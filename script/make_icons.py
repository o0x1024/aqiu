import os
from PIL import Image

def generate_tauri_icons(source_image_path, output_dir="icons"):
    """
    输入一张高清PNG图片，生成Tauri V2所需的Windows, Linux, macOS图标集合
    """
    
    # 1. 检查源文件
    if not os.path.exists(source_image_path):
        print(f"错误: 找不到源文件 {source_image_path}")
        return

    # 2. 创建输出目录
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
        print(f"创建目录: {output_dir}")

    try:
        # 打开图片并确保是 RGBA 模式
        img = Image.open(source_image_path).convert("RGBA")
        print(f"已加载源图片: {source_image_path} (尺寸: {img.size})")
    except Exception as e:
        print(f"无法打开图片: {e}")
        return

    # ==========================================
    # 1. 生成通用 PNG 图标 (Linux & 基础资源)
    # ==========================================
    # 文件名 : 目标尺寸
    png_files = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256, # @2x 通常是两倍分辨率
        "icon.png": 512,       # Tauri 默认的主图标
    }

    print("\n正在生成通用 PNG 图标...")
    for filename, size in png_files.items():
        resized_img = img.resize((size, size), Image.Resampling.LANCZOS)
        save_path = os.path.join(output_dir, filename)
        resized_img.save(save_path, "PNG")
        print(f" -> 已生成: {filename}")

    # ==========================================
    # 2. 生成 Windows 磁贴/Store 图标 (根据截图列表)
    # ==========================================
    # 这些通常是 Visual Studio / MSIX 打包需要的
    windows_tile_files = {
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
        "StoreLogo.png": 50        # StoreLogo 通常是 50x50
    }

    print("\n正在生成 Windows Store/磁贴图标...")
    for filename, size in windows_tile_files.items():
        resized_img = img.resize((size, size), Image.Resampling.LANCZOS)
        save_path = os.path.join(output_dir, filename)
        resized_img.save(save_path, "PNG")
        print(f" -> 已生成: {filename}")

    # ==========================================
    # 3. 生成 Windows .ico (包含多种尺寸)
    # ==========================================
    print("\n正在生成 Windows .ico 图标...")
    # .ico 标准尺寸包含 16, 32, 48, 64, 128, 256
    ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_path = os.path.join(output_dir, "icon.ico")
    img.save(ico_path, format="ICO", sizes=ico_sizes)
    print(f" -> 已生成: icon.ico (包含层级: {ico_sizes})")

    # ==========================================
    # 4. 生成 macOS .icns (包含多种尺寸)
    # ==========================================
    print("\n正在生成 macOS .icns 图标...")
    # 注意: Pillow 生成 icns 时，如果原图足够大(如1024x1024)，它会自动处理内部尺寸
    # 为了兼容性，最好确保源图 > 512px
    if img.size[0] >= 512 and img.size[1] >= 512:
        icns_path = os.path.join(output_dir, "icon.icns")
        # macOS icns 格式支持
        img.save(icns_path, format="ICNS")
        print(f" -> 已生成: icon.icns")
    else:
        print("警告: 源图片小于 512px，生成的 .icns 可能不清晰或不完整。建议使用 1024x1024 的源图。")

    print(f"\n✅ 所有图标已生成完毕！请查看 '{output_dir}' 文件夹。")

# --- 配置部分 ---
if __name__ == "__main__":
    # 在这里修改你的源图片文件名
    SOURCE_IMAGE = "dog.png" 
    
    # 运行生成
    generate_tauri_icons(SOURCE_IMAGE)