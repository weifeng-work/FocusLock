"""生成 FocusLock 应用图标源图 1024x1024 PNG"""
from PIL import Image, ImageDraw

SIZE = 1024
img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# 背景：品牌蓝 #185FA5，圆角填充
bg = (24, 95, 165, 255)
d.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=220, fill=bg)

white = (255, 255, 255, 255)
hole = (24, 95, 165, 255)

# 锁环（U 形）
lw = 26
d.rounded_rectangle([372, 320, 372 + 280, 320 + 280], radius=140, outline=white, width=lw)
# 用背景色矩形遮住锁环下半，形成 U 形开口
d.rectangle([372, 470, 372 + 280, 470 + 160], fill=bg)

# 锁体
bx0, by0, bw, bh = 320, 470, 384, 320
d.rounded_rectangle([bx0, by0, bx0 + bw, by0 + bh], radius=36, fill=white)

# 钥匙孔
d.ellipse([bx0 + bw // 2 - 34, by0 + 110, bx0 + bw // 2 + 34, by0 + 110 + 68], fill=hole)
d.rectangle([bx0 + bw // 2 - 14, by0 + 170, bx0 + bw // 2 + 14, by0 + 230], fill=hole)

out = r"C:\Users\IKUN\Project\大番茄\app-icon.png"
img.save(out, "PNG")
print("saved", out)
