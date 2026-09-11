# Implementation Notes: 首个可用版本交互优化

Spec: `interaction-feedback.md`

## Design decisions

- 以 UX-001 至 UX-006 为本轮验收范围，同时修复已复现的数据往返丢失、取消残留、保存错误退出和小窗口崩溃。
- 保留英文短操作文案，明确显示按键与板块的对应关系；当前板块使用文字标记与加粗边框，颜色作为补充。保留中文内容输入。
- Schedule 改为按时间排序的条目列表；新增/编辑使用时间与正文两字段。默认给 free memo 更多空间，长列表滚动。
- 默认选中 free memo，Enter 即可记录。宽屏下 schedule 高度随条目数适度增加，上限为内容区约三分之一且最多十行，其余高度给 free memo；过矮或过窄时只展示当前板块。
- 年视图采用独立页面，随终端尺寸分页展示月份；鼠标与键盘均可选择日期。编辑期间不切换底层日期，防止草稿丢失。
- 将数据、交互、编辑器、日历和渲染拆为小模块，避免在原有单文件中继续耦合。
- 确认原启动环境包含 `NO_COLOR=1`、`TERM=dumb`。选中态不依赖颜色，不修改用户全局终端环境。
- Schedule 主页面使用单行摘要，时间始终随条目显示；多行标记为 `↵`，Enter 打开完整可滚动编辑器。保留 04:00 日界和同一时间的多条独立记录。
- 新增/编辑统一 Ctrl+S 保存、Esc 取消；不依赖某些终端无法区分的 Ctrl+Enter。编辑器支持中文、粘贴、换行、方向键和 Home/End；鼠标可切换字段、保存和取消。
- 年历根据终端大小显示全部十二个月或分页，`y`/月历标题进入，键盘和鼠标选择日期，`g` 直接输入日期；Home 返回今天。编辑期间隔离底层点击和导航。
- 安装版使用固定用户数据目录，支持 `TECHO_DIR` 和 `--data-dir`；`dev.ps1` 默认显式打开仓库 logs，保留现有开发数据入口。
- Markdown v2 对待办续行和日程正文缩进，Free Memo 作为最后一节原样保存。旧格式首次保存前保留 `.md.alpha.bak`；只浏览日期不创建日记文件。
- 使用同目录临时文件写入、sync 后重命名；数据目录持有系统文件锁。保存前比较原文件内容，失败时保留编辑器及草稿，不更新已保存模型。
- 本次开发版本标为 `0.1.0-alpha.2`；不发布、不打标签，保留人工体验反馈阶段。
- 完成渲染检查后，收紧最小窗口年历按钮文案，保留完整可点击按钮；多行日程摘要始终为 `↵` 留出空间，不因截断而丢失标识。
- 编辑器采用清空背景的大表单，减少底层内容干扰，并避免弹层边界切到中文宽字符。

## Deviations

None.

## Tradeoffs

- 本轮不扩展重复日程、提醒、同步或插件系统。
- 月相在所选公历日期 12:00 UTC 取样，使用平均朔望月近似并显示 `(approx.)`，不请求网络。基准新月与周期取自 [NASA 月相表](https://eclipse.gsfc.nasa.gov/phase/phases1901.html)，用 2000 年 1 月及 [2026 年 9 月](https://eclipse.gsfc.nasa.gov/phase/phases2001.html)的主要月相日期校验。它不提供天文事件精确时刻或当地观测朝向。

## Open questions

None. 本轮明确需求均已实现并完成下述验证，后续继续收集实际使用手感的反馈。

## Requirement checks

| Requirement | Implementation and evidence |
| --- | --- |
| UX-001 明确板块切换 | 底部和标题直接展示按键与板块名；Tab/Shift+Tab、鼠标共同支持。已检查日页面预览。 |
| UX-002 可见选中态 | `>` 标题、粗边框、列表选中行反显；渲染测试断言选中标题，单色预览确认可辨认。 |
| UX-003 鼠标操作 | 根据每次实际渲染生成点击区域，覆盖空白、边框、条目、表单按钮；缩放后鼠标选区与闰日点击测试通过。Linux PTY 鼠标事件解码、板块选择和日期点击验收通过。 |
| UX-004 条目式日程 | 时间与事项表单、时间校验、排序及修改；全文进入编辑器，主列表只显示摘要并滚动，free memo 获得主要空间；排序、长列表与长文本测试通过。 |
| UX-005 全年日期导航 | 全年/分页月历、跨年、直接日期输入、回到今天；日期隔离、跨月/闰日、六周月份最小窗口测试通过。 |
| UX-006 月相 | 所选日期离线近似月相；基准及当代 NASA 日期测试通过，日期栏预览确认显示。 |
| 保存可靠性 | 多行/标题/空行往返、取消无残留、保存失败重试、外部修改保护、旧格式备份和目录锁测试通过。 |

## Verification commands

- Windows: `./dev.ps1 test`；`./dev.ps1 -CargoArgs @('clippy', '--locked', '--all-targets', '-D', 'warnings')`；`./dev.ps1 -CargoArgs @('fmt', '--check')`。
- 可选预览：设置 `TECHO_PREVIEW_DIR` 后执行 `cargo test export_previews -- --ignored`。预览使用测试数据，不读取私人日志。
- Linux: `cargo test --locked`、`cargo build --locked`、`python3 scripts/terminal_smoke.py <binary> [snapshot-directory]`。验收脚本使用可丢弃目录和真实 PTY，覆盖鼠标事件、中文多行粘贴、保存重启、年历与日期跳转、缩放和退出后的终端恢复。
- 已加入 Windows/Linux CI，尚未推送触发远端运行。

## Verification results (2026-09-11)

- Windows / Rust 1.97.1：20 项测试通过；1 项预览导出测试默认忽略，已单独执行通过；格式检查、Clippy（所有 targets，warnings 作为错误）和最终 debug 构建通过。
- Ubuntu 24.04 / x86_64 / Rust 1.97.1：相同 20 项测试通过，构建通过；最终代码使用 `--locked --offline` 验证。
- Linux 真 PTY 验收通过：无色环境下鼠标切换板块；中文和含 Markdown 标题的多行粘贴；任务勾选；日程保存；长笔记滚动到光标；全年展示；2024-02-29 点击跳转；跨年日期隔离；取消删除；80×1 后恢复；退出后终端模式恢复；重启后全部内容仍在。
- 已检查主页面、年历及日程表单的渲染和 PTY 文本快照；验证产物在被 git 忽略的 `target/previews`、`target/terminal-smoke`，可用上述命令重建。
- Linux 工具链位于 `/home/xinzh/.cache/techo-dev`，不改用户 shell 配置或默认 Rust 工具链。准备阶段临时目录丢失问题已通过持久缓存解决。
- 独立 Windows 窗口曾启动本轮较早的 release 构建；最终代码位于源码与 `target/debug/techo.exe`。保存退出当前窗口后，运行 `./dev.ps1` 即加载最终版本及原 logs 目录。
