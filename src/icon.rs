//! 程序图标：代码生成的剪贴板图形（带抗锯齿）。
//! 不依赖外部资源文件，不增加发布体积；
//! Windows 任务栏与窗口图标、Linux 窗口图标均使用它。

use eframe::egui::IconData;

const SIZE: usize = 128;

/// 圆角矩形的有符号距离函数
fn rounded_box(px: f32, py: f32, cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = (px - cx).abs() - (hw - r);
    let qy = (py - cy).abs() - (hh - r);
    let ax = qx.max(0.0);
    let ay = qy.max(0.0);
    (ax * ax + ay * ay).sqrt() + qx.max(qy).min(0.0) - r
}

struct Canvas {
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            px: vec![[0.0, 0.0, 0.0, 0.0]; SIZE * SIZE],
        }
    }

    /// 将一个圆角矩形混合到画布上
    fn box_layer(&mut self, cx: f32, cy: f32, hw: f32, hh: f32, r: f32, color: [f32; 3]) {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let d = rounded_box(x as f32 + 0.5, y as f32 + 0.5, cx, cy, hw, hh, r);
                // 1px 宽度的边缘渐变 -> 平滑不锐利
                let a = (0.5 - d).clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let dst = &mut self.px[y * SIZE + x];
                for c in 0..3 {
                    dst[c] = color[c] * a + dst[c] * (1.0 - a);
                }
                dst[3] = a + dst[3] * (1.0 - a);
            }
        }
    }

    fn into_rgba(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(SIZE * SIZE * 4);
        for p in self.px {
            for c in 0..4 {
                out.push((p[c].clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        out
    }
}

fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

/// 剪贴板图标：蓝色底板 + 白色纸张 + 顶部夹子 + 三条文本线
pub fn clipboard_icon() -> IconData {
    let mut c = Canvas::new();
    let accent = rgb(0x27, 0x83, 0xDE);
    let accent_dark = rgb(0x1B, 0x69, 0xB8);
    let white = rgb(0xFF, 0xFF, 0xFF);
    let line = rgb(0xA9, 0xCD, 0xF0);

    // 底板
    c.box_layer(64.0, 70.0, 44.0, 50.0, 14.0, accent);
    // 内部白色纸张
    c.box_layer(64.0, 74.0, 33.0, 38.0, 9.0, white);
    // 顶部夹子
    c.box_layer(64.0, 26.0, 22.0, 13.0, 6.5, accent_dark);
    c.box_layer(64.0, 24.0, 13.0, 6.0, 3.0, white);
    // 文本线
    c.box_layer(64.0, 62.0, 22.0, 3.5, 3.5, line);
    c.box_layer(64.0, 78.0, 22.0, 3.5, 3.5, line);
    c.box_layer(53.0, 94.0, 11.0, 3.5, 3.5, accent);

    IconData {
        rgba: c.into_rgba(),
        width: SIZE as u32,
        height: SIZE as u32,
    }
}
