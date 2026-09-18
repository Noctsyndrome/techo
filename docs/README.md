# 文档导航与阅读约定

techō 的文档服务于三种不同的问题：现在怎样使用和维护、某次变更为什么这样做、接下来有哪些值得评估的改进。请不要把这三种信息当作同一份待实施需求。

本页是阅读入口。本次只增加导航与评估文档，现有说明、历史方案和实施记录没有迁移，也没有因这次整理而改变产品行为。

## 使用与维护：持续更新的说明

普通使用从 [Guide](guide.md) 开始，按键与终端问题查 [Keys and terminals](keys.md)，数据位置与文件格式查 [Files](files.md)。这些文档应描述已经实现的使用方式，而不是尚未采纳的提议。

构建、测试与开发环境查 [Development](development.md)，发布流程查 [Releasing](releasing.md)。`keys.md` 是用户参考，`releasing.md` 是维护者流程；不要因为它们附近存在 `*-impl-notes.md`，就把它们归为历史 spec。

说明文档也可能滞后。例如审阅基线的 `releasing.md` 仍描述 macOS PTY 测试，而实际 CI 只在 Linux 执行该脚本。遇到此类差异，应核对代码、测试和工作流，记录并修正文档；不要为了匹配旧文字而擅自修改产品。

## 评估：有日期、有基线的建议

[2026-09-18 初版体验与发行准备评估](reviews/2026-09-18-initial-release-readiness.md) 汇总了产品定位、低负担交互、具体代码风险、跨平台验证、预编译发行以及文档组织建议。它包含代码依据、建议优先级、待决策事项和验收条件。

该评估的状态是 `advisory`，不是已批准的 spec，也不是实施完成报告。修复候选与改变既有交互的提议在正文中分开标识；阅读它不等于获得整批实现、发布版本或改动历史设计的授权。

## 历史方案与实施记录：按变更主题一起读

| 变更主题 | 原始方案或需求 | 实施记录 | 阅读边界 |
| --- | --- | --- | --- |
| alpha.2 交互优化 | [interaction-feedback.md](interaction-feedback.md) | [interaction-feedback-impl-notes.md](interaction-feedback-impl-notes.md) | 记录一轮交互优化；之后的页面与键位调整可能已改变其中的实现。 |
| 回到手账的设计调整 | [design-review.md](design-review.md) | [design-review-impl-notes.md](design-review-impl-notes.md) | 设计哲学仍有参考价值，但部分具体方案被实测推翻或替代，必须读偏离与取舍。 |
| 键位与 macOS 适配 | 需求与决策写在实施记录中；[keys.md](keys.md) 是当前用户参考 | [keys-impl-notes.md](keys-impl-notes.md) | 包含保存主键、功能键替身、Esc 取消等明确决定，以及未完成的真实终端验证。 |
| crates.io 发布流程 | 当时的用户发布请求，记录于实施说明 | [release-process-impl-notes.md](release-process-impl-notes.md) | 解释当时的发布范围；当前操作流程查 [releasing.md](releasing.md)。 |

两个容易误读的例子：`design-review.md` 提议过时间标尺、单击直接编辑等方案，后续实施记录说明了为什么未沿用；`keys-impl-notes.md` 明确保留 Esc 直接放弃草稿。这些不能仅凭较早的方案或较新的评估建议就被自动反转。

这里没有把所有历史文件整体标为“废弃”。同一个文件可能同时包含仍有效的产品原则、已经实现的功能和被替代的交互细节；需要按具体主题确认。

## 给本地开发 agent 的阅读顺序

先读根目录 [README](../README.md) 和本页，理解产品边界，再按任务选择当前说明。开展初版完善时读上述评估，并只选取用户已经指定或确认的范围；随后读取相关源码、测试，以及该主题的实施记录。涉及设计改变时再回查原始方案，不必每次加载全部历史文件。

判断“当前实现是什么”时，以实际代码、测试及工作流为证据；判断“这次应该改成什么”时，以当前任务和明确批准的 spec 为目标。两者发生冲突时应显式说明。历史记录用于保留决策上下文，评估用于提出候选改进，任何一方都不自动成为更高优先级的新指令。

完成变更后，同步相关使用或维护说明，在该变更的实施记录中写明实际取舍、验证命令、结果和剩余限制。不要仅通过更新旧 review 中的建议来声称实现已经完成。

## 建议的后续目录形态

不建议为了当前体量搭建文档站或多层分类体系。持续维护的五份说明先保留现有路径；以后将历史方案与实施记录按变更主题迁入 `changes/`，评估留在 `reviews/`。

下面是迁移建议，不代表这些 `changes/` 路径已经存在：

```text
docs/
  README.md
  guide.md
  keys.md
  files.md
  development.md
  releasing.md
  reviews/
    2026-09-18-initial-release-readiness.md
  changes/
    alpha2-interaction/
      spec.md
      implementation.md
    paper-planner-design/
      spec.md
      implementation.md
    keyboard-compatibility/
      implementation.md
    crates-release-process/
      implementation.md
```

spec 与实施记录放在同一主题下，而不是拆进两个互不相邻的大目录。原本没有独立 spec 的变更不必事后补造一份。未来新变更只有在需要明确范围、决策与验收时才建立 spec；小修复可以保留简短实施记录或提交说明。

迁移时应一次性修正根 README、Development、文档间的相对链接和引用路径，并检查失效链接。已对外传播的路径可保留短跳转说明，不保留两份会独立漂移的正文。本次没有执行这一步。

## 最小状态约定

新 spec 建议在开头注明类型、状态、基线及相关实施记录。状态可以是 `proposed`、`accepted`、`implemented` 或 `superseded`，但必须有真实的批准、实现或替代依据；不要仅凭日期推断。review 使用 `advisory`，保留审阅时的基线与验证边界。

实施记录应回答实际做了什么、相对 spec 改了什么、为何改变、在哪些环境验证、哪些没有验证。用户说明持续反映当前版本，不承载逐轮讨论。先落实这些区别，比增加更多目录更重要。
