# techo

`techo`（てちょう，日语“手账”）是一个以 [Ratatui](https://ratatui.rs/) 构建的日常手账 TUI。

它参考纸质手账的固定结构：当天日期、右上待办、左侧时间刻度辅助的 Schedule、底部 Free Memo、月视图和每日引文。Schedule 的纸上“一天”从当天 04:00 延续到次日 04:00；时间尺帮助定位节奏，但不会把记录限制为某个小时的一条条目。只有待办与两块 memo 会写入日志；月视图和引文是 TUI 的展示层。

## 启动

```powershell
cd D:\Project\tui-playground\techo
.\dev.ps1 run
```

`dev.ps1` 会自动载入已安装的 Visual C++ Build Tools 环境，再把参数转交给 Cargo，因此无需手动打开 Developer PowerShell。它同样适用于 `.\dev.ps1 test`、`.\dev.ps1 clippy -- -D warnings` 等命令。若你已经在 Visual Studio 的 Developer PowerShell 中，则可以直接使用 `cargo run`。

首次启动会创建 `logs/YYYY-MM-DD.md`。该文件是普通 Markdown，包含 YAML frontmatter、Obsidian 的任务复选框和标准标题，因此可以直接在 Obsidian 中打开、检索和编辑。

## 快捷键

| 按键 | 功能 |
| --- | --- |
| `s` | 直接聚焦 Schedule |
| `t` | 直接聚焦 Todo |
| `f` | 直接聚焦 Free Memo |
| `Tab` / `←` / `→` | 在 Schedule、Todo、Free Memo 间切换 |
| `↑` / `↓` 或 `j` / `k` | 在 Schedule 中以 30 分钟移动时间针；在 Todo 中选择条目 |
| `Enter` | 在 Schedule 时间针的位置创建/编辑记录 |
| `n` | 新建待办 |
| `e` | 编辑当前 Todo、Schedule 或 Free Memo |
| `Space` | 勾选/取消当前待办 |
| `Ctrl+S` | 保存 |
| `Ctrl+Enter` | 在编辑弹窗中保存（`Enter` 换行） |
| `q` / `Esc` | 退出（编辑时 `Esc` 取消编辑） |

## 日志示例

```markdown
---
date: 2026-07-31
tags:
  - techo
---

# 2026-07-31

## TODO
- [ ] Read the Ratatui tutorial

## Schedule

### 09:30
Build the first screen in the morning.

### 00:40 (+1)
Write down the late-night idea.

## Free Memo
The terminal grid feels close to a paper techo.
```
