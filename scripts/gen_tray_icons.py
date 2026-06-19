"""生成 FocusLock 三状态托盘图标（32x32 与 64x64，PNG）

三状态：
- working: 蓝色 #185FA5
- resting: 绿色 #0F6E56
- paused:  灰色 #888780

每个图标：圆角方块背景 + 白色简化锁形（锁环 U + 锁体方块）
"""
from PIL import Image, ImageDraw

STATES = {
    "working": (24, 95, 165, 255),
    "resting": (15, 110, 86, 255),
    "paused":  (136, 135, 128, 255),
}

def draw_lock(draw, size, color):
    """在 size×size 画布中央画白色锁形"""
    s = size
    # 锁环 U 形：用一个圆角矩形描边 + 背景色遮下半
    ring_lw = max(2, s // 14)
    cx = s // 2
    ring_w = int(s * 0.42)
    ring_h = int(s * 0.34)
    ring_top = int(s * 0.24)
    draw.rounded_rectangle(
        [cx - ring_w // 2, ring_top, cx + ring_w // 2, ring_top + ring_h],
        radius=ring_w // 2,
        outline=color,
        width=ring_lw,
    )
    # 遮住环下半（用背景色，但这里背景已画好，我们用透明擦除——改用先画锁体再画环的顺序）
    # 简化：锁体方块覆盖环底部
    body_w = int(s * 0.58)
    body_h = int(s * 0.40)
    body_top = ring_top + ring_h - ring_lw
    draw.rounded_rectangle(
        [cx - body_w // 2, body_top, cx + body_w // 2, body_top + body_h],
        radius=max(2, s // 16),
        fill=color,
    )

def make_icon(state, bg, size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # 圆角方块背景
    radius = max(2, size // 5)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=bg)
    # 白色锁形
    draw_lock(d, size, (255, 255, 255, 255))
    return img

out_dir = r"C:\Users\IKUN\Project\大番茄\src-tauri\icons\tray"
import os
os.makedirs(out_dir, exist_ok=True)

for state, bg in STATES.items():
    for size in (32, 64, 128):
        img = make_icon(state, bg, size)
        img.save(f"{out_dir}\\tray-{state}-{size}.png", "PNG")
        if size == 32:
            img.save(f"{out_dir}\\tray-{state}.png", "PNG")
    print(f"  {state}: 32/64/128")

# 默认（working）也作为 tauri.conf.json 引用的初始图标
make_icon("working", STATES["working"], 32).save(
    f"{out_dir}\\tray-default.png", "PNG"
)
print("done:", out_dir)
