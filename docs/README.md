# 文档导航

持续维护的说明留在 `docs/` 根目录；评审放在 `review/`，方案与实施记录放在 `spec/`。两个目录都平铺文件，不再增加主题子目录。

## 使用与维护

使用从 [Guide](guide.md) 开始，按键与终端问题查 [Keys and terminals](keys.md)，数据位置与格式查 [Files](files.md)。构建与测试查 [Development](development.md)，发布流程查 [Releasing](releasing.md)。这些说明应随实现更新，不承担逐轮讨论记录的职责。

## 评审与方案

当前初版完善建议见 [2026-09-18 初版体验与发行准备评估](review/2026-09-18-initial-release-readiness.md)。它是 `advisory` review，不是已批准的 spec，也不是已完成的实施报告。

历史设计评审见 [2026-09-11 设计评审](review/2026-09-11-design-review.md)，对应的 [实施记录](review/2026-09-11-design-review-impl-notes.md) 与它平级保存。评审中的部分具体方案已被实施时的反馈替代，不能只读原始设想。

交互优化的 [2026-09-11 spec](spec/2026-09-11-interaction-feedback.md) 和 [实施记录](spec/2026-09-11-interaction-feedback-impl-notes.md) 同样平级保存。另有 [2026-09-11 发布流程实施记录](spec/2026-09-11-release-process-impl-notes.md) 和 [2026-09-15 键位适配实施记录](spec/2026-09-15-keys-impl-notes.md)；它们原本没有独立 spec，不为整理目录补造一份。

## 命名约定

```text
docs/
  README.md
  guide.md
  keys.md
  files.md
  development.md
  releasing.md
  review/
    YYYY-MM-DD-topic.md
    YYYY-MM-DD-topic-impl-notes.md    # 有对应实施记录时
  spec/
    YYYY-MM-DD-topic.md
    YYYY-MM-DD-topic-impl-notes.md
```

review 与 spec 都使用日期前缀。实施记录沿用对应文档的日期和主题前缀，只追加 `-impl-notes`；后续修改不改变文件名前缀，实际实施与验证日期写在正文里。没有对应文档的独立实施记录使用自身记录日期。历史文件按原记录日期或首次入库日期归档，而不是此次搬迁日期。

spec 开头注明状态即可，例如 `proposed`、`accepted`、`implemented` 或 `superseded`；基线、范围和验收按任务需要补充。review 保留评估基线和建议属性。文件进入某个目录不等于方案已获批准，也不把历史文档中的“待处理”自动变成当前任务。小修复不强制配齐两份文档。

## 给开发 agent 的阅读约定

先读根 [README](../README.md) 与本页，再按任务读当前说明、相关 review/spec 及其实施记录，不必遍历全部历史。判断当前行为时核对代码、测试和工作流；判断本次改动目标时依据用户当前任务与明确批准的方案。历史评审和较新的建议都不自动覆盖既有决定，例如 Esc 取消策略应先查看键位实施记录。

完成变更后更新相关使用说明，并在同目录的实施记录中写明实际取舍、验证结果和未验证部分。迁移文档时同步更新相对链接，不保留会独立漂移的重复正文。此次整理只改变文档位置、导航和命名约定，不改变产品行为。
