# 审计发现

## 仓库定位
- `F:\workspace\IELTS Atlas`：旧 opensource Web 工作树，无 Cargo/Tauri/Vue。
- `F:\workspace\IELTS Atlas APP`：目标 Rust/Tauri 工作树，已有 `src-tauri/`、`crates/`、Vue/Vite 产物和任务书。

## 已知缺口
- writing 使用 `DeterministicProvider`，真实 AI 尚未接入。
- `tauri.conf.json` 未声明题库/词典/媒体 resources。
- Electron afterPack/update/server 遗留脚本仍存在。
- CI 无 Rust/Tauri build/package/资源/AI 门禁，旧 JS 聚合测试失败。

## 2026-07-12 独立复核证据

### Rust/Tauri 与资源
- `src-tauri/src/commands/writing.rs:58-67` 生产命令硬编码 `DeterministicProvider`；`crates/ielts-db/src/writing/evaluation.rs:107-164` 按词数映射分数并返回固定反馈，Cargo 无 HTTP provider 依赖。
- `src-tauri/tauri.conf.json:31` 未声明 `bundle.resources`；`assets/generated/reading-exams` 主要是 JS，启动 `src-tauri/src/lib.rs:110-124` 不扫描/注册内置资源。
- `reading_list_assets` 只查索引；没有等价的阅读全文 command。Vue 提交仍依赖前端 `payload`，干净安装后题库可能为空或无法启动练习。
- `crates/ielts-db/src/secrets/mod.rs:23-29,108-112` 是 Base64 + 明文文件 vault，不是 OS keychain/Stronghold。
- SQLite mutex 当前包住整个评测状态机；直接接网络 provider 会阻塞其他 DB command，必须先拆短事务 checkpoint。

### Vue 与旧路径
- `apps/writing-vue/src/views/PracticeReadingPage.vue` 约 5767 行、164 个函数，god-page 仍在；多个页面仍有大量 DOM/window 直接操作。
- `apps/writing-vue/src/modules/legacy/legacyScriptLoader.ts:16-99`、`PracticeReadingPage.vue:2252-2300` 动态加载根目录 JS/CSS/assets；这些文件不在 `frontendDist` 或 Tauri bundle 中，安装包会断。
- Vue 26 个组件无 `lang="ts"`，无 `tsconfig.json`/`vue-tsc`；生成 `domain.ts` 存在重复声明，核心 API 仍是 JS。
- `localStorage/sessionStorage` 仍是草稿、结果、阅读状态等事实源旁路；canonical DTO 还在 camel/snake/legacy alias 间转换。

### 测试门禁与 DoD
- `npm --prefix apps/writing-vue run build` 失败：`client.js` named import `newIdempotencyKey`，但 `writing-repository.js` 未导出该符号。
- `python developer/tests/ci/run_static_suite.py` 实跑 `120 total / 104 pass / 16 fail`，仍强制 `npm run build:server`、Electron DAO、根 `index.html` 等已淘汰宿主契约。
- `python developer/tests/e2e/suite_practice_flow.py` 因 Playwright 缺失失败；脚本固定 `file:// root/index.html` 和 `window.app/window.storage`，不覆盖 Tauri。
- CI/release 没有把上述两道门禁、Vue typecheck、Tauri packaged smoke、资源完整性、真实 provider、密钥负向测试设为阻断条件。
- 任务书和 DoD 勾选大量 ✅，但同一任务书 `:1431` 仍承认真实 LLM、阅读全文 command、签名密钥和实机 E2E 未完成，属于文档超前。

## 结论
当前应判定为“Rust/Tauri 骨架 + 迁移中 Vue 壳”，而不是完成态。Electron/Fastify 热路径虽基本删除，但 legacy 资源和 file:// 测试仍构成产品/门禁双路径。不得先删除旧资产；先建立唯一资源 DTO、全文 command、可打包资源和绿色 Vue build，再迁移测试并删除旧宿主。

## 实施进展
- Vue shipping build 已恢复，repository export 错误已修复。
- Rust 已增加 `reading_get_asset_payload`；Vue 选中索引后通过 command 获取完整 payload。
- 225 份 reading JSON resource pack 与 SHA-256 manifest 已生成，Tauri bundle 声明资源，启动时校验并幂等 seed。
- OpenAI-compatible provider 已实现；评测拆为 prepare/network/finish，网络期间不持 SQLite mutex；deterministic 仅作为明确降级。
- 静态门禁已改为 Tauri/Vue shipping baseline，实跑 5/5；旧 file:// E2E 已停止冒充产品覆盖，但 packaged Tauri runner 尚未实现。
- 当前 vault 仍为 Base64 文件，安全闭环未完成；god-page、全面 TypeScript、legacy bridge 与浏览器状态旁路仍需处理。
- AI 已完成 Rust 单一所有权：统一 runtime、真实 provider test、写作评测、Reading Coach、配置 CRUD、默认配置激活、OS keyring 与安全门禁。Vue 不再保存 API Key 或返回假成功。
- AI 静态安全门禁和 packaged Tauri 基线通过；真实外部账号成功调用需要用户提供 API Key，仓库内只验证可复现 mock/错误路径。
- shipping legacy 收尸完成：删除根 `index.html`、`js/`、`css/`、`templates/` 和 Vue `modules/legacy/`；根 package 移除 `express`。
- 删除后验证：`npm run typecheck`、Vue build、`cargo check --workspace --locked`、static gate `6/6`、packaged Tauri E2E `5/5` 全部通过。
- 仍未完成的工程债务：`PracticeReadingPage.vue`/`PracticeLibraryPage.vue` 仍偏大；`useReadingAttempt/Coach/Highlights` 尚未进入 strict；release 签名/updater secrets、Windows/macOS notarization 和真实账号 AI 成功调用需要外部凭据。
# Library / Settings page split findings (2026-07-12)
- `PracticeLibraryPage.vue` and `SettingsPage.vue` persisted durable UI preferences and backup indexes directly in Web Storage, creating a second source of truth beside Tauri SQLite settings.
- Durable values moved in this pass: GPL acknowledgement, browse preferences, reading suite preferences, reading backup index, and writing settings backup index.
- Direct Web Storage remains only in temporary-cache cleanup and legacy theme/endless compatibility paths. Removing those requires a public Tauri cache/theme/session contract outside this subtask's file ownership.
- The pages remain large; this pass isolates persistence side effects but does not claim full god-page decomposition.
