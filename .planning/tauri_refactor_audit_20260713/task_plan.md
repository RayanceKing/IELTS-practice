# Tauri 重构现状审计计划

## 目标
冻结当前工作区成果，并依据 `developer/docs/IELTS_Practice_Rust_Tauri_重构任务书.md` 对 Tauri 2 / Rust 原生化的真实完成度进行证据化审计。

## 阶段
- [x] 阶段 1：核对 Git 状态并提交确认前已有工作区变更
- [x] 阶段 2：读取任务书，划定并发审计边界
- [x] 阶段 3：并发子代理审计并等待全部返回
- [x] 阶段 4：主代理交叉核验关键结论与测试状态
- [x] 阶段 5：输出 Core Verdict / Key Insights / Linus Plan 审计报告

## 约束
- 审计阶段不继续实现重构。
- 不清理、不覆盖用户现有变更。
- 子代理只使用一次；派发后立即等待全部完成。
- 任务书与产品实际调用链优先于提交说明。

## 错误记录
| 错误 | 尝试 | 处理 |
|---|---:|---|
| bundled `rg.exe` 在当前 WindowsApps 路径启动被拒绝 | 1 | 改用 PowerShell `Select-String`，不重复调用同一失败命令 |
| `cargo test --workspace` 失败：`ielts-db` settings 两个单测 panic | 1 | 不重跑相同命令；转为读取失败测试与校验器，定位契约漂移 |
| `cargo test -p ielts-db --tests` 仍先执行相同 lib unit tests 并失败 | 2 | 停止 broad cargo test；若需验证集成测试，仅点名具体 `--test` binary |
| 点名集成测试时 `ai_config_security` 仍因 `hasSecret` heuristic 失败 | 3 | 根因已跨 unit/integration 复现；停止该链重试，仅验证不依赖 AI config 写入的其他 Phase tests |

## 最终判定
- 架构切换已完成到 shipping runtime 单一 Tauri/Rust；不能回退到 Electron/Fastify。
- Phase 0–10 均有实现骨架，但没有任何一个 Phase 可以按任务书最终出口整体标绿。
- 当前首要阻断依次为：备份静默丢数据；阅读 payload/服务端判分断链；写作异步评测/事件断链；AI config 无法保存；Phase 7/8 双真相；发布与体验证据。
