# 进度日志

## 2026-07-12
- Library/settings subtask: added `useTauriPreferences` over the Tauri settings repository and migrated durable page preference/backup state away from direct localStorage. Added explicit async hydration before page initialization. Vue typecheck and production build pass.
- 并发完成三面审计：Rust/AI/资源、前端 god-page/遗留、测试/DoD。
- 确认实际目标仓库为 `F:\workspace\IELTS Atlas APP`，原 cwd 为旧 opensource 工作树。
- 创建目标仓库的持久计划文件。
- 本轮开始独立复核，不直接采信上一轮摘要；计划并发检查 Rust/Tauri、Vue/遗留路径、测试与 DoD。
- 三组子代理完成独立复核：Rust/Tauri 资源与 AI、Vue/旧路径、测试/DoD。
- 复核结论：真实 AI、资源打包、阅读全文链路、OS 密钥存储、Vue build/typecheck、Tauri E2E 门禁均未成立；旧动态资源路径和 file:// 门禁仍在。
- 本轮未删除资产、未改产品代码；删除前置条件是新单一路径可启动、可读题、可提交且有可复现门禁证据。
- 用户确认进入完整实施。P0 并行范围：Vue shipping build、Rust 阅读全文/资源链路、Tauri 最小绿色门禁。
- P0 完成：Vue build 通过；Rust 阅读全文 command 与校验测试完成；静态门禁 5/5。
- 资源阶段完成：225 份 JSON pack、manifest、bundle.resources、首次启动幂等 seed。
- 真实 AI 主链完成：OpenAI-compatible provider、无锁网络调用、故障/解析/恢复测试。
- 规定第二道 E2E 当前明确 unavailable，尚需真实 packaged Tauri runner，不能计入 DoD。
- OS keyring 已替换 Base64 文件 vault，并支持 v1 一次性迁移。
- packaged Tauri E2E 驱动安装完成；修复双 logger 启动 panic 后，启动、Vue 路由、reading IPC、bundle resource、SQLite 重启 5/5 通过。
- 前端 typecheck/build 通过；Library/Settings 部分偏好已迁入 Tauri settings，Reading 页面完成第一轮 composable 抽取。
- 写作会话已移除 local/sessionStorage，路由、草稿、评测恢复和结果查询统一 SQLite。
- Library/ShuiBackground 已移除 Three、词汇、成就、旧时钟动态加载；CI/release 首 gate 已包含 packaged E2E 与 bundle manifest。
- legacy 模块剩余引用已缩小为 Settings 引导/更新/主题与 useReadingTimer，根旧资产暂不可删除。
- 用户确认将所有 AI 功能纳入 Rust；当前阶段重新打开，目标包含 provider test、Reading Coach、真实事件/进度流和 Vue 占位清理。
- 全量 AI Rust 化完成：provider配置/密钥、连通测试、写作、Reading Coach、失败状态均由 Rust/Tauri 管理；静态门禁 6/6、packaged E2E 5/5。
- Legacy shipping tree 已删除，根 express 依赖移除；删除后完整 baseline 仍通过。
- 任务书尚不能整体标记完成：god-page/剩余 strict 类型和外部签名、notarization、updater/真实账号证据仍是明确未完成项。

## 2026-07-13
- 工作区快照提交：`5037468 native-ai: move providers into Rust and bury shipping legacy`（443 files）。
- 阶段 5 推进：从 `PracticeReadingPage.vue` 抽出 `readingQuestionIds.ts` + `useReadingInteractions.ts`（交互模型、拖拽、dropzone DOM 同步）。
- 页面从约 5527 行降到 5063 行；逻辑迁出，CSS 仍占大头。
- `useReadingAttempt` / `useReadingHighlights` / `useReadingTimer` / interactions 进入 strict typecheck；`useReadingCoach` 仍未纳入。
- 验证：`npm run typecheck` 通过；Vue production build 通过；static suite `6/6` 通过。
- 设计系统收敛启动：以 `IELTS Atlas` HeroUI/Shui 为唯一视觉事实源，落地 `styles/design-system/{tokens,aliases,base}.css`；写作 terracotta / liquid-glass 名称降级为 alias；Library/Settings 去掉页面内 token 重声明。
