"""把三状态托盘图标(32x32 PNG)转成 RGBA 字节数组，生成 Rust 源文件。

这样 tray.rs 用 Image::new(rgba, 32, 32) 直接构造图标，
无需 image-png feature / image crate 解码依赖。
"""
from PIL import Image

STATES = ["working", "resting", "paused"]
OUT = r"C:\Users\IKUN\Project\大番茄\src-tauri\src\tray_icons.rs"

def to_rust_array(rgba_bytes, name):
    # 生成形如 &[0xff, 0x1f, ...] 的字面量
    chunks = [rgba_bytes[i:i+16] for i in range(0, len(rgba_bytes), 16)]
    lines = []
    for c in chunks:
        line = "    " + ", ".join(f"0x{b:02x}" for b in c) + ","
        lines.append(line)
    return f"pub static {name}: &[u8] = &[\n" + "\n".join(lines) + "\n];\n"

parts = ["// 自动生成：三状态托盘图标 RGBA 数据（32x32），由 gen_tray_rgba.py 生成\n"]
parts.append("// 请勿手动修改；修改请编辑 scripts/gen_tray_icons.py 后重新运行 gen_tray_rgba.py\n\n")
for state in STATES:
    path = rf"C:\Users\IKUN\Project\大番茄\src-tauri\icons\tray\tray-{state}-32.png"
    img = Image.open(path).convert("RGBA")
    assert img.size == (32, 32), f"{state} 尺寸非 32x32: {img.size}"
    rgba = img.tobytes()
    name = f"TRAY_{state.upper()}_RGBA"
    parts.append(to_rust_array(rgba, name))
    print(f"  {state}: {len(rgba)} bytes")

with open(OUT, "w", encoding="utf-8") as f:
    f.write("".join(parts))
print("done:", OUT)
