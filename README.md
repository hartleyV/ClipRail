# ClipRail Rust

ClipRail轻量管理粘贴板内容，简单易用，采用Rust语言，支持Windows 和 Linux 平台。

## 目标发布文件

- Windows x64：`ClipRail.exe`
- Linux x64：`ClipRail`


## 功能

- 监听文本和图片剪贴板
- 内容 SHA-256 自动去重，不限制记录数量
- Item 单击复制、清晰的“✓ 已复制”状态
- Item 置顶、独立背景色、批量选择和删除
- Item 拖动排序、拖出窗口删除、拖动渐变动画
- 响应式图片预览和始终可见的置顶按钮
- 日期筛选及 `data/archives/YYYY-MM-DD.json` 自动归档
- 可配置显示/隐藏快捷键
- 竖栏置顶与边缘自动隐藏
- 拖动顶部移动竖栏，拖动左边缘调整宽度
- 数据保存在可执行文件同级 `data` 目录


## 编译

安装 Rust 后双击：

```text
build-windows.bat
```

输出：`ClipRail.exe`。

## Linux 本机编译

Debian/Ubuntu 先安装系统依赖：

```bash
sudo apt install build-essential libx11-dev libxkbcommon-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev
chmod +x build-linux.sh
./build-linux.sh
```

输出：`ClipRail`。

## 使用

将生成的可执行文件放到一个具有写权限的目录，直接运行。首次记录内容后会自动创建：

```text
data/
├─ clips.json
├─ settings.json
├─ images/
└─ archives/
```

默认快捷键：`Alt + Shift + V`。
