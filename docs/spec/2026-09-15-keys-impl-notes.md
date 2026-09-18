# Implementation Notes: 键位策略与 mac 适配

起因：在 mac 上试用时，按 Windows/Linux 习惯设计的键位到不了 techo，最基本的保存就有冲突。调研了 nano、micro、helix、vim/neovim、emacs 终端模式和 Textual 框架的做法之后定下本文的策略。本文记录决定和理由；`?` 页面是应用内的集中展示，[keys.md](../keys.md) 是面向用户的对照表，改键位时两处一起改。

## 决定

- **保存主键是 Ctrl+S。** 与 micro、现代 nano、Textual 生态一致。Ctrl+Enter 在传统终端里和 Enter 无法区分，代码仍然接受，但不再出现在页脚和帮助里，以免 Terminal.app 用户以为 techo 坏了。
- **编辑器的 Ctrl 字母键遵循 readline 习惯。** Ctrl+A 行首、Ctrl+E 行尾，与 shell、nano、emacs 和 macOS 原生文本框相同。techo 已经用 Ctrl+C 退出、靠括号粘贴而不是 Ctrl+V，本来就是终端式而非桌面式，所以将来不考虑把 Ctrl+A 用作全选。
- **通用层只用字母、方向键和 Ctrl 加字母。** 不绑定 Alt/Option：mac 终端默认把 Option 当成输入特殊字符。不依赖 Cmd/Win：传统终端协议里没有它们的编码，Terminal.app 的 ⌘S 是「导出文本」，AppKit 菜单在按键抵达终端视图之前就把它消费掉了。
- **功能键都有字母替身。** mac 键盘没有 Home/End/PgUp/PgDn，Terminal.app 默认还把 Home/End 用来滚动自己的窗口。日视图 `T` 回今天（小写 t 是 todo 面板）、`K`/`J` 对应 PgUp/PgDn；年视图 `,`/`.` 翻月；便签里 Ctrl+A/Ctrl+E 对应 Home/End。原有功能键全部保留。
- **kitty 键盘协议只做增强，不做前提。** 启动进入备用屏之后向终端询问，支持则推送 DISAMBIGUATE_ESCAPE_CODES，退出和 panic 时弹出。开启后 Ctrl+Enter 可辨认，mac 的 ⌘ 以 Super 修饰键到达，techo 把 Super+S 也当作保存。这是「按终端能力协商」而不是「检测到 mac 就换键位」；唯一按平台区分的是显示：只有 macOS 且协议已开启时，页脚和帮助才写 ⌘S。Windows 上 crossterm 固定回答不支持，Windows Terminal 本身就能区分 Ctrl+Enter。
- **帮助页只列键位。** 按使用场景分组：page、dates、year、note、journal，左列按键、右列作用，每个 techo 响应的键都在其中。日记目录、words.txt 和本月的名字与颜色收在最后的 journal 组，同样是两列形式，不再和键位说明混排。终端太矮时可以滚动（方向键、j/k、PgUp/PgDn、滚轮），其他任何键关闭。
- **`techo --keys` 诊断模式。** 原样打印终端送来的每个按键和修饰键，并说明协议是否开启，Ctrl+C 结束。按键不生效时先用它区分是终端没发还是 techo 没认。这是 micro 的 `> raw` 和 Textual 的 `textual keys` 的做法。

## 不做的事

- 不做用户自定义键位。目前没有配置文件，改键的收益不足以引入一套配置格式；`?` 页面已经把键位集中起来。将来若做，帮助页需要跟随实际绑定。
- 不把撤销/重做、选区等编辑器功能混进本轮。它们是编辑器功能规划，与键位可达性无关。
- Esc 仍然直接放弃草稿，不加确认。这是既有的便签式交互决定；正因为如此，保存键必须有第二条可靠路径，也是加 ⌘S 和字母替身的理由之一。

## 实现要点

- `main.rs`：`enhance_keyboard` 在 `EnterAlternateScreen` 之后调用，因为协议栈属于当前屏幕；用一个原子布尔记住是否推送过，`restore` 只在推送过时弹出。`supports_keyboard_enhancement` 在 unix 上会发出查询并最多等两秒，对不回答设备属性查询的伪终端会拖慢启动，所以 PTY 冒烟脚本现在会回答主设备属性查询（只答 DA、不答 `?u`），既不等待也不切换编码。
- `app.rs`：`enhanced` 记录协议状态，`command_saves()` 决定是否提及 ⌘S；帮助页的滚动位置由绘制时按终端高度夹紧。
- `editor.rs`：`line_start`/`line_end` 抽出来给 Home/End 和 Ctrl+A/Ctrl+E 共用。
- `ui.rs`：`help_lines` 生成分组的两列内容，`draw_help` 按高度取一段并画页脚；弹窗宽 84 列，键列 20 列，说明列 58 列以内。

## 验证

- `./dev.ps1 -CargoArgs @('fmt','--check')`、`./dev.ps1 -CargoArgs @('clippy','--locked','--all-targets','-D','warnings')`、`./dev.ps1 test --locked`。
- 新增测试：Ctrl+A/Ctrl+E 到行首行尾且不插入字母；Super+S 保存而普通 s 是字母；`T`、`,`、`.`、`J`、`K` 的替身行为；帮助页滚动键与关闭键；帮助页在 120×40 完整显示、在 80×24 滚到最后一行；⌘S 只在 macOS 且协议开启时出现；Ctrl+Enter 不再出现在帮助里。
- 顺手修了一个日期依赖的旧测试 `down_past_the_end_starts_a_new_item`：它假设「今天」就是测试固定的 2026-09-11，过了那天就失败；现在改为打开真实的今天。
- 冒烟脚本改用「techō · keys」识别首启帮助，CSI 参数解析允许 `<`、`>`、`=` 前缀，避免协议序列卡住解析。
- 未在真实 mac 终端上验证。需要在 Terminal.app、iTerm2、Ghostty 或 kitty 上各试一次：`techo --keys` 看 ⌘S 是否以 Command 修饰键到达，再在便签里按 Ctrl+S、⌘S、Ctrl+A、Ctrl+E。
