# ClipRail Rust

ClipRail 的轻量原生版本。Windows 和 Linux 分别生成一个可直接运行的小型文件，不需要 Python、`.venv`、Qt 或安装器。

## 目标发布文件

- Windows x64：`ClipRail.exe`
- Linux x64：`ClipRail`

启用 LTO、体积优化、移除调试符号和 panic unwind。实际大小由编译器、图形后端和目标平台决定，通常远小于 PySide6 版本。

## 保留功能

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

## 最简单：GitHub Actions 自动生成两个文件

1. 创建一个 GitHub 仓库并上传本目录全部文件，包括 `.github`。
2. 打开仓库的 **Actions → Build ClipRail → Run workflow**。
3. 构建完成后下载：
   - `ClipRail-Windows`：包含 `cliprail.exe`
   - `ClipRail-Linux`：包含 `cliprail`

CI 会在真实 Windows 和 Ubuntu 环境分别编译，因此不需要在本机安装两套交叉编译工具链。

## Windows 本机编译

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

## 平台说明

Windows PE/EXE 与 Linux ELF 是两种不同的可执行格式，因此不能用同一个二进制文件同时运行；本项目通过同一份 Rust 源码生成两个独立的小文件。
