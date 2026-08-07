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
- 仍未完成的工程债务：`PracticeReadingPage.vue`/`PracticeLibraryPage.vue` 仍偏大；`useReadingCoach` 尚未进入 strict；release 签名/updater secrets、Windows/macOS notarization 和真实账号 AI 成功调用需要外部凭据。
- 2026-07-13 god-page 拆分进展：阅读拖拽/交互已迁入 `useReadingInteractions`；attempt/highlights/timer 已 strict。页面仍含高亮 DOM、提交/endless/suite 编排和超大 CSS。
- 设计系统崩坏根因：同时存在 terracotta writing tokens、liquid-glass `--lg-*`、页面内 Shui/Bauhaus/Bloom 重声明，以及 Atlas 源站 HeroUI 体系。现已建立 `styles/design-system` 作为全局 token 源；页面级 CSS 仍大量硬编码颜色/阴影，需继续按组件迁到 token。
- 紫色品牌已从 shipping Vue 树清除：`#667eea/#764ba2` 与对应 rgba 不再出现。brand primary/secondary 现为 teal/amber，gradient/soft/ring 走 shui/bloom；`.btn-brand` 改为 glass sheen 而非紫渐变按钮。Library 本地 brand 重声明已删，仍残留本地 gray/spacing 重声明特殊情况。
- 2026-07-13 原生路径审计：Web Storage 作为 durable 旁路的主犯是 reading UI 偏好、背景主题、endless 状态；答案/提交 sessionStorage 是只写不读死代码。已统一到 `frontend-preferences`（SQLite）。Settings/Library 仍保留清理遗留 Web Storage 的迁移/清缓存逻辑，属过渡期可接受。
# Library / Settings page split findings (2026-07-12)
- `PracticeLibraryPage.vue` and `SettingsPage.vue` persisted durable UI preferences and backup indexes directly in Web Storage, creating a second source of truth beside Tauri SQLite settings.
- Durable values moved in this pass: GPL acknowledgement, browse preferences, reading suite preferences, reading backup index, and writing settings backup index.
- Direct Web Storage remains only in temporary-cache cleanup and legacy theme/endless compatibility paths. Removing those requires a public Tauri cache/theme/session contract outside this subtask's file ownership.
- The pages remain large; this pass isolates persistence side effects but does not claim full god-page decomposition.

## 2026-08-07 Application 层重构基线

- `src-tauri/src/commands/writing.rs` 的 `writing_start_evaluation` 当前负责 provider 配置、持久化 prepare、异步任务、网络调用、finish 和 Channel 事件发送；普通 writing CRUD 同文件但无需迁移。
- `src-tauri/src/commands/enrichment.rs` 的 `coach_run` 当前负责追加用户消息、加载历史、解析 runtime、调用 LLM、完成消息或记录 degraded failure。
- `src-tauri/src/commands/ai.rs` 同时混合 AI 配置命令、Keyring 对账、HTTP runtime、retry 与 completion 解析；业务模块反向导入 `commands::ai`，依赖方向错误。
- `AppDb` 是单 `Mutex<rusqlite::Connection>`；当前写作和 Coach 都会在网络 await 前释放锁，这一行为必须保留并增加回归测试。
- `ielts-db::writing::evaluation` 已拥有持久化状态机、`PreparedEvaluation`、事件与 deterministic provider；首轮仅包裹现有函数，不迁移 schema 或重写状态机。
- Vue 的写作和 Coach facade 仍有编排逻辑，但本轮保持现有 command/DTO，不同时改前端。
- AI 抽离的隐藏调用点包括 `commands/backup.rs` 对 `list_ai_configs_with_vault` 的调用；配置层搬迁时必须同步更新，不能只改 writing/Coach。
- `AiRuntime`/provider config 必须留在 Tauri 基础设施：它们依赖 `AppDb`、`AppVault`、Keyring 与 reqwest。Application 只能持有 `LanguageModel`、store 和 event sink 端口，否则形成 crate 循环。
- 数据库保存 provider 品牌值（openai/openrouter/deepseek），仅运行时归一为 openai-compatible；抽离时不可改变该持久化语义。
- 重构前 Rust workspace 基线失败：`crates/ielts-db/tests/phase7_modes.rs` 第 133、622、649、679 行的旧测试构造缺少 `timer_snapshot`。产品代码尚未修改，因此这是既有门禁缺陷。

## 2026-08-07 Application 层重构结论

- `ielts-application` 只拥有写作评估与 Coach 两个垂直用例；普通 CRUD 仍由 Tauri command 直接调用 `ielts-db`。
- `LanguageModel` 是唯一 AI 端口；provider 配置、Keyring、HTTP client、超时和重试已移到 `src-tauri/src/ai`，application crate 不依赖宿主或基础设施类型。
- `ApplicationStore` 的每个方法只持有一次短生命周期 SQLite mutex；fake model 在 completion 内可重新获取同一 store mutex，测试证明网络 await 没有跨越数据库临界区。
- 写作保持幂等恢复、事件重放、deterministic 过渡路径、score 成功但 feedback 非法时 degraded；Coach 保持先持久化用户消息、失败记录与 `preserve_scores = true`。
- Tauri command 名称、输入输出 DTO、Vue 调用和数据库 schema 均未修改；旧 writing/Coach provider 模块已删除，重复 prompt/JSON 解析只保留在 application。
- 门禁基线同步修复：迁移版本断言更新到 10；Reading timer 测试显式提供 snapshot；history task type 测试先完成 evaluation 再进入历史；AI 静态安全门禁改为检查 command + config adapter 两层。
- 验证完成：Rust workspace 全绿；Vue typecheck/build 全绿；static suite 18/18；packaged Tauri E2E 全部边界通过。

## 2026-08-07 Agent 后端边界审计

- 现有 `LanguageModel` 是稳定的 JSON 文本 completion 合同；OpenAI tool calling 允许 `content: null` 并返回多个 `tool_calls`，必须新增并列 `AgentModel`，不能给现有 DTO 堆可选字段。
- Agent 消息使用枚举表达 system/user/assistant/tool result，避免 `role: String` 加多组互斥 `Option` 形成非法状态；application 负责有限轮次和顺序工具调用，HTTP runtime 只负责单轮 provider 请求。
- 工具原始输入/输出可能包含文件正文或任意模型生成数据；持久化端口只接收工具执行器生成的脱敏审计 JSON，禁止直接保存 raw content、workspace grant、绝对路径、API key 或请求头。
- 数据库新增 v11 `agent_runs` 与 `agent_tool_calls` 两表即可；begin/finish 分离，模型及文件 I/O 期间不持 SQLite mutex。备份需升 v7 并冻结原 v6 表清单，否则历史 v6 包会被误判缺表。
- Tauri 已有 `tauri-plugin-dialog` 和进程内 UUID grant 先例；workspace grant 应为 `grant_id -> canonical_root + expires_at`，重启失效，不向 WebView 开放 `fs:*` capability。
- 路径边界必须拒绝绝对路径、盘符/UNC、`..`，并对已有目标或最近存在父目录 canonicalize 后做组件级 containment；字符串前缀判断不能防 Windows junction/symlink 逃逸。

## 2026-08-07 Agent 第一阶段实现状态

- `ielts-application/src/agent.rs` 新增独立 `AgentModel`、枚举消息、工具执行器和 `AgentService`；模型单轮请求与工具执行严格顺序，硬限制 8 轮/24 次调用，provider/空响应/重复 call id 均有持久化终态错误。
- `src-tauri/src/ai/runtime.rs` 保留现有 JSON completion body，另行发送 `tools`/`tool_choice`，解析 `content: null` 与多个 tool calls；两条协议共享有限重试，不共享 DTO。
- `src-tauri/src/agent/file_tools.rs` 只暴露 `read_file`、`write_file`、`replace_in_file`。文件正文只作为本轮模型上下文，不写入审计；审计仅含相对路径、字节数、SHA-256、替换次数和受控错误。
- `src-tauri/src/agent/workspace.rs` 的授权是 UUID → canonical root + 15 分钟 TTL，重启即失效；WebView capability 未扩展为 fs/shell。
- `src-tauri/src/app/application_store.rs` 每个 Agent store 方法只通过一次 `AppDb::with_conn` 短锁调用 ielts-db；模型和文件工具均在锁外执行。
- DB 审计主键使用 `(run_id, sequence)`，不把外部 provider `call_id` 当全局主键；application 仍拒绝空/重复 call id，避免协议关联歧义。

## 2026-08-07 本地后端层级收敛首轮

- Writing evaluation 的配置选择已冻结为一次快照：command 在 prepare 前读取 `AiProviderConfig`，后台任务使用同一快照构造 runtime；不再在句柄返回后重新选择默认模型，避免评估记录 provider/model 与实际请求漂移。
- Agent response 在任何工具执行前整批校验非空 id、唯一 id、非空 name 和剩余工具配额；重复 call id/同批超限不会再产生部分文件副作用。路径 containment、SHA-256 乐观锁、原子写入、UTF-8/尺寸限制保持不变。
- AI 配置/备份恢复的凭据边界已改为两阶段：SQLite 锁内只读取 `SecretRef`/配置元数据，锁外访问本机 vault/keyring，再以内存 `ref_id` 集合做短事务 reconciliation。runtime API Key 解析同样不在 SQLite mutex 内做 vault I/O。
- Coach 空问题现在由 application service 在写入消息和加载模型前拒绝；fake store 不再复制该业务判断。写作公开 `list_events`/`cancel` service 纯转发已移除，command 直接调用 store trait，内部 start 的事件重放仍走 application。
- `AiRuntime` 不再派生 `Debug`/`Clone`，避免包含 API Key 的运行时被意外格式化或复制；没有新增权限、审计服务或其他企业级安全层。
- 门禁证据：`cargo test --workspace --target-dir target/gate` 全绿；`npm --prefix apps/writing-vue run typecheck`、`build` 全绿；static suite `18/18`；packaged Tauri practice E2E 全部通过。
- 静态 AI 检查器已从旧 command 内函数名改为识别新的 `src-tauri/src/ai/config.rs` 两阶段 reconciliation 适配器，门禁契约与实际层级保持一致。

## 2026-08-07 Agent audit 契约收敛

- `ielts-application` 已删除与 `ielts-db::agent` 同构的 `AgentRunStart/Finish`、`AgentToolCallStart/Finish` 及 status enum；`AgentStore` 直接接收 DB begin/finish command。
- `ApplicationStore` 不再逐字段复制 run/tool audit payload，也不再做 application status 到 stored status 的 match；仍保留 ApplicationError 的统一 IPC 错误码和每次 DB 调用的短锁边界。
- Application 层继续拥有 Agent 循环、工具批量预检、模型消息和脱敏错误 JSON；数据库层继续拥有 SQL 约束、终态校验、恢复和备份。没有新增第三套契约，`agent_get_run` 返回 DTO 未改。

## 2026-08-07 Agent audit finish failure

- `finish_tool_call` 位于文件工具副作用之后，若写入失败不能继续追加 `ToolResult` 或发起下一轮模型调用；直接返回 `agent.persistence_failed`，让 DB 保持 `running`，由启动恢复标记 `interrupted`。
- 正常文本完成的 `finish_run` 失败同样直接透传持久化错误，不重试终态写入，避免提交后错误造成二次 CAS 语义混乱。
- provider/非法响应/限额路径的 `finish_run` 只是补偿性收尾；收尾失败不得覆盖原始业务错误，使用 tracing 记录。
- 未新增 `uncertain` 状态或新的持久化抽象，复用现有运行恢复机制。

## 2026-08-08 Vue 视觉搬迁首轮

- `opensource` 参考树是静态 HTML/CSS，不存在可搬迁的 Vue 组件；当前产品只吸收壳层、卡片、控件、空态和响应式经验，继续使用现有 Vue composable、Tauri DTO 与 SQLite 数据源。
- 全局壳层统一到 1400px token；导航图标使用本地内联 SVG，不新增图标依赖或复制 opensource 的 DOM/JS 协议。
- 390x844 packaged WebView 暴露了 Library tabs 的 intrinsic width 会把整页撑到 540px；正确修复是让 header/tabs `min-width: 0` 并由 tabs 自身横向滚动，不能用页面级 `overflow-x: hidden` 掩盖。
- Library 的 author CSS 会覆盖原生 `[hidden]`，导致未开放“听力”筛选可见；局部恢复 `[hidden] { display: none !important; }` 后，可见类型只剩“全部/阅读”。
- 最终证据：desktop/mobile Overview 与 Browse 截图无页面级横向溢出；Vue typecheck/build、static suite 18/18、packaged Tauri E2E 全绿。

## 2026-08-08 Vue 视觉搬迁 Reading History

- 参考树的练习历史是桌面双列、窄屏单列；当前 Vue 已恢复该几何，且 loading/empty 节点跨越整个 grid，避免空态只占第一列。
- 移动端结果动作在 `.record-result` 内保持同一右侧 flex 区域，未移动 `data-record-action`、checkbox 或 `@click.stop` 事件。
- 当前 author CSS 会覆盖原生 `[hidden]`，Library 局部规则仍需保留；历史搜索清除按钮继续使用 `:hidden` 以保持测试与布局状态契约。
- packaged WebView 390x844 实测 History section `scrollWidth=375`，页面无横向溢出；桌面 packaged E2E UI route visual 也保持通过。
- 下一块应先处理 Settings 概览的分区 surface 与共享 modal 几何，再考虑 Settings 旧 CSS 清理；不要在视觉切片中重写备份/恢复或 API Key 数据流。

## 2026-08-08 Vue 视觉搬迁 Settings AI 卡

- Settings 总览的 AI 与评测卡已有稳定的 Vue 数据和事件契约，最小搬迁单位是 CSS 视觉规则而不是模板或脚本重写。
- 顶部 accent、动作区分隔和统计 badge 使用现有 `--atlas-*` / `--lg-*` token，未引入 opensource 的第二套运行时或数据结构。
- `settings-detail-modal` 当前缺少专用定位、最大高度和滚动规则；它被明确留到后续独立切片，不能把总览视觉改动与弹层行为混在一起。
- Settings detail modal 已通过全局 skin 补齐 fixed scrim、bounded panel、内部滚动和横向 tabs；`.atlas-source-ui:has(.settings-detail-modal)` 状态在 modal 打开时锁住 app shell，避免底层页面滚动条与 modal 滚动条叠加。
- 390px packaged WebView 实测 API 配置入口、Escape 关闭和 `scrollWidth=375`；四个入口仍由原有 `data-settings-open` 与 `openSettingsDetail` 驱动。
- U3 下一窄切片为全局 History 页面表面与弹层几何，仍只改视觉 CSS，不改 history repository、筛选语义或图表数据。

## 2026-08-08 Vue 视觉搬迁 Global History

- Global History 已有稳定的筛选/图表/列表结构，最小视觉切片只需在 skin 中补 surface accent、列表容器和 overlay 几何，不需要移动模板节点。
- 详情和删除 overlay 之前只有背景颜色，没有 fixed/inset、最大高度和内部滚动；现在由全局 skin 提供 bounded dialog，移动端不会把长内容撑出页面。
- 390px packaged WebView 实测 `scrollWidth=375`，日期/分数筛选在移动端按列排列；header 的导出/清空按钮保持可见且不横向溢出。
- modal 打开时通过 `.atlas-source-ui.atlas-source-ui:has(.history-page .dialog-overlay)` 锁住底层 app shell，避免底层和 dialog 同时滚动。
- U3 视觉目标已完成，后续 U4 应优先处理 Writing Compose/Evaluating/Result 的布局连续性；不把 Prompt 或评测 API 调整混入视觉切片。

## 2026-08-08 Vue 视觉搬迁 Writing Compose

- Compose 的 DOM、v-model、题目选择、草稿保存、提交门禁和 Tauri 路由已经稳定；视觉搬迁可安全限制在全局 skin，避免与 writing application/AI contract 同批修改。
- 页面 scoped CSS 已拥有基本几何，因此本轮只用高特异性 `.atlas-source-ui .compose-page.compose-page ...` 收口 surface、segmented control、prompt accent、editor input、footer 和 dialog；没有复制 opensource JS 或 DOM 协议。
- `:has(.compose-page .dialog-overlay)` 可复用 Settings/History 的 modal scroll-lock 模式；弹层保持 fixed scrim，避免确认层打开时 app shell 底层滚动。
- Packaged UI route visual 报告确认 Compose desktop `viewportWidth=1440`、`scrollWidth=1425`、`offenders=[]`；静态套件与全量 practice E2E 仍为绿色。
- Result 是下一个更高收益的 Writing 切片：现有 skin 对 `.text-primary/.score-fill/.bullet` 的合并规则会把 Result 标题刷成背景色，迁移评分/反馈 surface 时必须拆开为 text color、SVG stroke 和 bullet background 三种职责，并补 390px 单列几何。

## 2026-08-08 Vue 视觉搬迁 Writing Result

- Result 现有 `.result-layout` 使用固定 viewport 高度，且没有专用移动断点；全局 skin 现在统一使用稳定的双列 grid，并在 1040px/640px 做单列与指标收缩。
- 评分 summary 是右栏第一个直接 `.glass-card`，无需新增模板 class 即可限定 accent 条、评分环尺寸和 surface；其它右栏卡片继续共享同一层级，不复制数据结构。
- 全局 skin 对 `.text-primary`、`.score-fill`、`.bullet` 必须拆开写职责，避免 SVG/文本元素受到 `background-color` 污染；本轮已修正。
- `.essay-body` 内的 v-html error highlight 继续由 ResultPage scoped `:deep` 规则负责语义颜色，全局只补 radius，不改 highlight class 或 HTML 生成。
- Packaged 全量 E2E 在 Result CSS 加入后仍保持通过；下一步应审计 Evaluating 的 `.aurora-sphere` 硬编码旧色与状态 rail surface，保留事件状态机。

## 2026-08-08 Vue 视觉搬迁 Writing Evaluating

- Evaluating 的状态 rail、progressbar、log list 和 action row 已有完整 DOM/事件合同；适合只在 skin 中覆盖，不需要把事件状态机搬到 CSS 或新 composable。
- `.aurora-sphere` 原先硬编码旧紫色；通过 `--atlas-accent`、`--atlas-accent-alt` 和 `--atlas-accent-soft` 重定向，保留 conic-gradient 和 breathing animation，避免复制第二套颜色。
- Evaluating 与 Compose/Result 共用 `.essay-panel/.glass-card/.right-panel` 名称，所有新规则必须保留 route scope，避免跨页面污染。
- 已通过 desktop packaged route smoke；下一次必须用 390px/320px 专用 WebView 检查 rail、长日志、重试/取消按钮和 essay panel 是否出现横向溢出，然后再标记 U4 收口。

## 2026-08-08 Writing 移动核验结论

- Built Vue 不能直接以 `file://` 启动；资源协议导致根节点不挂载。一次性本地 HTTP server 是不改源码的最小截图路径。
- 导航在窄屏中故意由自身承接横向滚动，DOM 项目可位于 viewport 之外但 document `scrollWidth` 不增加；页面级 overflow 检查应限定在 route root，不能把隐藏的 nav scroll content 误判为页面回归。
- Result 必须注入完整 Tauri history detail 才能验证 score/metrics/feedback，而不是只看 `loadError` 空壳；真实形状数据下 390/320px 均无 route 内 overflow。
- Writing 三页在 desktop packaged smoke 与窄屏浏览器等价视口均通过，U4 可安全收口；下一阶段只迁 Reading/Coach 视觉，不改其 composable、回答状态或 coach persistence。

## 2026-08-08 Reading Coach 视觉结论

- Coach 是提交后出现的独立视觉岛，现有 Vue/composable/API 合同稳定；最小搬迁单位是路由级 CSS owner，不是组件重写。
- `--atlas-reading-nav-height` 已是底部导航几何真源。Coach 继续复制 12/20/64/70px 断点魔数会制造重叠特殊情况，因此 FAB/panel 必须从该变量派生。
- Coach 不能使用 9999 层级；settings/notes modal 分别位于 2500/2600，Coach 降至 2200 后模态遮罩重新获得正确交互所有权。
- 对整个 panel 设置 `touch-action:none` 会禁用消息区原生滚动；消息区应为 `pan-y`，离散操作控件应为 `manipulation`。
- packaged E2E 在新 skin 下完整通过，证明 `data-reading-coach-*`、提交/重置可见性、notes dialog 与 Reading 数据流没有被 CSS 搬迁破坏。

## 2026-08-08 Reading Coach 几何与标注工具栏结论

- Coach 几何的唯一真源方案已被四视口计算样式验证：桌面导航 72px、移动导航 112px 时，FAB/panel 的 `bottom` 都随变量正确变化，没有复制断点魔数。
- 最低 360x640 视口下 panel 高 432px、top 32px，消息列表承担滚动且 composer 保持可见；不需要新增低高度特殊分支。
- `.reading-selection-toolbar` 与 header/floating panel 共享 glass selector 是垃圾视觉所有权：它让深色上下文 action strip 变成普通页面 surface。本轮从共享组移除并建立 `#selbar` 局部 owner。
- `#selbar` 使用 opensource 与既有 E2E 共同合同 `#1e293b`，不能通过修改全局 `--atlas-ink` 达成，否则会污染所有页面文字色和暗色主题。
- 题目导航不能照搬 opensource 新版 `part-nav-section/q-column/q-bar-segment`，因为当前 Vue 只有扁平 `questionOrder`；下一步只能迁移共同的 surface、状态和响应式布局。

## 2026-08-08 Reading 题目导航结论

- 现有 112px 导航通过 `overflow-y:auto` 隐藏了结构性高度不足；“document 没横向溢出”不等于可用，390/360px 的题号行实际位于 viewport 底部之外。
- 继续让六个操作项 `flex-wrap:nowrap` 后被压缩是垃圾响应式策略。移动端三列两行 grid 用同一数据结构消除了按钮宽度、行高和套题/单篇特殊情况。
- 平板只需要两行 128px；手机需要 184px。两个断点都继续写回 `--atlas-reading-nav-height`，因此 Reading page padding、Coach FAB/panel 和导航仍共享一个几何真源。
- 套题进度的两行内部排版是唯一额外高度来源；移动端改为单行 flex 后，所有操作项精确 44px，不需要再抬高整个导航。
- Desktop 72px 布局保持不变；没有搬运 opensource 新版 Part 分组 DOM、JS 导航逻辑或状态模型。

## 2026-08-08 Agent 工作区视觉结论

- opensource 没有可直接搬迁的 Agent 页面；可复用的是 workspace 主区/侧栏、紧凑 toolbar 和响应式堆叠经验，不能复制其旧 JS view/controller。
- 当前 Vue Agent 视觉页应继续留在共享 App/NavBar 壳层，Reading 的 frameless route 规则不适用于 Agent 工作区。
- 首个 Agent UI 切片只做本地状态占位是最小零破坏方案；真实 workspace grant/run/audit 接线应作为后续独立产品切片，不能伪装成视觉搬迁的一部分。
- 六项移动导航本身可横向滚动且不撑宽 document；把 Agent 排在写作之后可让新入口在 390/360px 首屏出现，不需要额外 `scrollIntoView` 脚本。
- 页面三栏在 1440px 为约 327/598/367px，980px 收敛为 321/567px 加底部 run panel，390/360px 单列；四视口均无页面级横向溢出。

## 2026-08-08 U7 CSS 收口结论

- 仅按 CSS 父级上下文判断重复，不能把 media query 下的同选择器当作垃圾；最终 root 同作用域重复为 0。
- 页面本地样式仍保留行为和回退责任；只有与 `.atlas-source-ui + route class` 完全同值、且 shipping App 永远提供该 scope 的声明才可删除。
- Packaged route visual 现在覆盖 Library、Compose、Topics、Settings、History 和 Agent；Reading/Coach、备份、资源和 SQLite 边界仍由既有流程覆盖。
- Agent 页面当前是纯视觉/本地状态占位，不声明 workspace grant、模型调用或审计数据已经接入；后续真实 Agent UI 接线必须作为独立用例切片。

## 2026-08-08 U8 题库管理切片冻结

- `TopicManagePage.vue` 已有稳定的题库查询、筛选、分页、上传、编辑和删除行为；本切片不改模板、脚本、Tauri command 或数据库。
- opensource 的可复用经验是题库/库管理工作区的紧凑 header、搜索与筛选工具栏、列表卡片层级、loading/empty/error 状态和 bounded modal，而不是旧 `LibraryManager` 的 JS/data 协议。
- 当前 Topic 页面 scoped 样式仍把移动端 header、filter、card footer 和 dialog 作为局部旧实现；需要在 `opensource-skin.css` 用 `.atlas-source-ui .topic-manage-page.topic-manage-page` 接管视觉 owner，避免全局 `.card/.dialog` 规则串色。
- U8 验收重点是 `1440x900`、`980x720`、`390x844`、`360x640`：route root 不得横向溢出，弹层必须 bounded 且内部滚动，按钮保持可触控，loading/empty/error 不应只占 grid 第一列。

## 2026-08-08 U8 实现与验证

- `opensource-skin.css` 现在是 Topic 管理页面的高特异度视觉 owner：紧凑 header、搜索/筛选 toolbar、卡片网格、状态 surface、分页和三类 modal 均使用 Atlas glass token；移动端在 760/480px 断点收敛为单列且不依赖页面级裁切。
- Topic modal 使用 fixed scrim、bounded dialog、内部滚动和 `:has()` app-shell scroll lock；桌面/平板/手机均不会产生第二条底层滚动。
- 图片上传区保留原有 DOM 结构和文件校验，但增加 `role="button"`、`tabindex="0"` 与 Enter/Space 触发，补充 `focus-within`；这修复了 opensource 对应上传入口已有而 Vue 缺失的键盘交互。
- 定向脚本 `developer/tests/e2e/topic_manage_visual_check.py` 使用 Tauri mock 只提供样例题目数据，不接触真实 API；四视口 document/body `scrollWidth` 均等于 viewport，6 张卡片、fixed modal、键盘 file chooser 均通过。
- Settings 审计发现的通用确认弹层缺口留作下一独立切片，未与 U8 混改。

## 2026-08-08 U9 Settings 弹窗切片冻结

- SettingsPage 的 onboarding、update、confirm 三个 `.dialog-overlay` 只继承了 scrim，没有 fixed/inset/grid/z-index/max-height；危险区确认层可能按普通文档流出现，无法稳定遮挡底层内容。
- opensource 的 `theme-modal` 经验可迁移的只有 shell 几何：fixed scrim、居中 bounded content、86vh 左右高度和窄屏内部滚动；不迁移其 JS controller、storage 或 DOM 协议。
- U9 只对 `.settings-page` route scope 增加 modal shell，detail modal（已有 z40）与二级确认层使用明确层级，保留所有现有按钮/事件。

## 2026-08-08 U9 Settings 弹窗实现

- 二级 overlay 使用 direct-child route selector，固定为 `z-index: 60`，dialog 使用 `min(620px, 100%)`、受限最大高度和内部滚动；760px 以下使用顶部对齐与 `100dvh` 高度边界。
- `:has(.settings-page > .dialog-overlay)` 同时锁定 app shell 滚动，并把 `app-main` stacking context 临时抬到 130，确保 sticky NavBar（120）不会拦截遮罩顶部的点击。
- 只读审计确认 SettingsPage 模板和数据流无需改动；三个弹窗继续使用原有 v-if、按钮、关闭事件和 Tauri 调用。

## 2026-08-08 U10 Settings detail surface 实现

- 外层 `.settings-detail-panel` 保留 `var(--atlas-shadow)` 高层 surface；直接子 `.settings-panel` 和其列表/模式/about 内容改用 `var(--atlas-surface-soft)` / `var(--atlas-control-surface)` 并清除大阴影。
- `.danger-zone` 只保留 danger rim 与 `var(--atlas-danger-surface)`，不再叠加普通 raised panel 阴影；所有 tab 的表单、滚动和按钮合同未变。
- 定向 Settings 检查覆盖五个 tab：每个 tab 至少保留一个 content section，外层 detail 有 shadow，内部 section/nested content 的 computed `box-shadow` 均为 `none`；四个视口通过。
- U10 全量验证通过：Vue typecheck/build、static suite `18/18`、`cargo test --workspace --target-dir target/gate`、packaged Tauri E2E 全部通过。下一切片冻结为 History detail modal 状态层级。
# 2026-08-08 U11 History Detail

- `HistoryPage.vue` 的详情数据流已经完整，U11 不需要修改 API、DTO 或异步请求；视觉问题来自外层 dialog 与所有内容块使用同等 raised/glass surface。
- 主分数原本的品牌渐变被后置的 History route 组合规则中性化，导致总分、分项、正文和分析缺少主次。
- `detailData` 为 null 且 loading/error 都结束时原模板没有 body；这是一个真实空态缺口，可用单一 `v-else` 补齐，不扩展业务逻辑。
- U11 采用单一 route-scoped owner：外层 dialog 保持唯一 elevated surface，正文/分析用 soft surface，反馈用 info surface，分项用 control surface，基本信息只保留一个外框。

# 2026-08-08 U12 Small-Screen Navigation

- 主导航固定为六个入口；`<=900px` 原实现改为横向滚动并隐藏滚动条，390/488px 现有截图只能看到前四项，历史和设置入口不可发现。
- 无需增加菜单状态、ResizeObserver 或 JS 测量。`<=640px` 使用固定 3×2 grid 即可完整展示六项，保留图标与短标签。
- `PracticeReading`、`PracticeReadingSuite`、`PracticeReadingReview` 为 frameless route，不受主导航断点影响。
- 主 nav 缺 `aria-label`，活动链接只有视觉 class；补充 `aria-label="主导航"` 与 `aria-current="page"` 不改变路由合同。

# 2026-08-08 U13 Reading Suite

- 当前套题 skin 在同一个 `:is()` 选择器中同时给 header、summary、passages 和 passage row 设置 raised background/shadow，形成 card-inside-card。
- 套题 row 已有 `active/submitted/locked` class，可只通过 route-scoped CSS 表达状态，不需要改 API 或新增状态字段。
- `suite` 为 null 且没有 error 时原模板没有可见 body；补充一个只读 empty state，保持 `practiceReadingSuite.get` 和错误分支不变。
- U13 的层级目标：header unframed；summary/passages 是工作区 raised surface；row 是 control surface；active 使用 selected surface；submitted 使用 info surface；pending 使用 muted surface；loading/empty 使用 bounded soft state。
- Rust `PassageStatus` 的稳定值是 `pending|active|submitted`，页面按 `${entry.status}` 生成 row class；任何 `locked` selector 都是无效视觉合同，必须以 `pending` 为准。
- `PracticeReadingSuitePage` 是 frameless route；视觉回归必须断言无 `.nav-shell`、`.app-main--frameless` 存在、summary/passages 仍有 shadow、被动 rows 无 raised shadow以及四视口无页面横向溢出。

## 2026-08-08 U14 Reading Library

- `PracticeLibraryPage` 的 More Tools、Reading Settings、config list 共享 `.view.hero-panel` / `.hero-settings-group > .hero-panel` 的 raised 规则；这会把工作区和每个子项都抬成同等级卡片。
- U14 采用单一 route-scoped owner：More Tools/Settings 外层保留 raised，tool/settings/config 子项使用 control/soft surface；featured tool 和题库状态 badge 使用 selected/info accent。
- clock/PDF 是 `.app-main` 内的 fixed overlay；由于 app-main 自带低 stacking context，导航栏可拦截 overlay 控件。仅在 overlay 存在时提升 app-main，不改导航或路由。
- U14 的专用 Tauri mock 证明 1440/1024/390/360px 下 More Tools 三卡、Settings 四 panel、config、clock、PDF 均 bounded 且无页面级横向溢出。
- 下一缺口是 `ReadingSuiteSelector` modal chrome：`.theme-modal-header/close/body` 无 shipping CSS，`.suite-flow-option` 继承 `inline-flex + nowrap`，窄屏描述会横排或溢出；可作为 U15 CSS-only 切片。

# 2026-08-08 U15 Reading SuiteSelector

- `ReadingSuiteSelector.vue` 的行为合同已经稳定：模式按钮点击直接触发父层创建流程，频率 select 只更新范围，取消和 backdrop close 继续使用现有 emits；U15 不应增加二次确认或改 command/DTO。
- `PracticeLibraryPage.vue` 仅提供 modal 几何，shipping skin 缺少 header/close/body/subtitle/actions owner；通用 `.suite-flow-option` 的 `inline-flex + nowrap` 会使 `<small>` 描述在窄屏横排。
- modal 位于 `.app-main` stacking context 内，单独 `z-index: 200` 无法稳定压过 `NavBar` 的 `z-index: 120`；打开 SuiteSelector 时必须沿用 U14 的 `.app-main` 提升与背景滚动锁定。
- U15 最小实现只改 `opensource-skin.css` 和定向 Playwright mock；下一切片冻结为自选套题选择条 `custom-suite-selection-bar` 的状态层级和窄屏几何。

# 2026-08-08 U16 Custom Suite selection bar

- `customSuiteDraft` 已有 `selecting/ready`，chip 已有 `filled`，确认按钮已有 `disabled`；视觉层可以完全复用这些数据，无需增加状态机或模板分支。
- 通用 Library `:is()` selector 的最高参数含两个 class，使其特异度高于看似等价的 route-scoped 三类 selector；U16 用重复 route class 建立真正 owner，避免 `!important`。
- 移动端不能只断言无横向滚动。三列被压成 22px 时 document 仍无横向溢出，却会产生数千像素异常高度；视觉回归必须同时冻结 chip/action 最小宽度。
- 长标题在桌面安全截断、手机正常换行；ready surface 由全部三个 `filled` chip 推导，不新增业务状态。

# 2026-08-08 U17 Reading Library secondary navigation

- Library 二级导航固定为五个入口；原 `<=900px` 方案使用 `nowrap + overflow-x:auto + hidden scrollbar`，390/360px 只能发现前几项。
- 无需新增菜单状态或测量脚本。`<=640px` 使用 3×2 grid 即可让五项完整可见，同时保留现有按钮、active class 和 `aria-current`。
- 320px 下每列仍有 88px，五个入口均保持 44px 高、无横向 overflow；桌面 1440px 和平板 768px 保持原单行布局。
- 下一审计目标是 Reading History 的 widget flip 和移动动作区；该区域存在 3D rotor、三种 widget、趋势筛选和批量动作，必须先用真实状态回归确认缺口，不能盲目重排。

# 2026-08-08 U18 Reading History audit

- `ReadingHistoryPanel.vue` 已经把趋势范围、heatmap/priority/radar、自定义翻面、筛选、导出和批量删除表达为稳定 props/emits；U18 不需要修改数据结构或 Vue 事件。
- `PracticeLibraryPage.vue` 的通用按钮规则给 `.practice-custom-option` 和 History 动作继承了 `white-space: nowrap`；同时 `<=900px` 只把 header 竖排并把 actions 设为 `width: 100%`，没有冻结手机端三动作的列宽和触控几何。
- 自定义 widget back face 使用 `position: absolute; inset: 0`，外层只有 `min-height: 270px`。在窄屏中必须同时验证三个选项、关闭按钮和 back face scrollHeight；单纯检查 document 无横向溢出会产生假绿。
- U18 的最小验证面冻结为 1440/1024/390/360px：front/back face 均 bounded、三种 widget 可切换、趋势范围状态不变、History 筛选和三动作均完整可见且至少 44px、无页面级横向溢出。
- 真实 mock 回归还暴露一个非空历史路径 bug：`PracticeLibraryPage.vue` 使用 `safeDateMs` 但缺失 import；补齐后历史列表、统计、热力图和优先级数据才能正常渲染。
- 视觉修复后四视口实测：趋势/雷达 canvas `position:absolute` 且不再增加 face 高度；desktop rotor/front/back 同高；360px 自定义 header 允许换行且不越过卡面；移动 History actions 自然换行、按钮高度稳定 44px。
- 截图抽查确认底部导航固定层与历史内容没有产生页面级横向溢出；批量选择 toast 仍由原 Vue 状态驱动，未改事件或 DTO。

## 2026-08-08 U18 收口与 U19 冻结

- U18 的产品改动保持在 Reading History 视图：补齐非空历史路径的 `safeDateMs` 依赖，修复趋势/雷达 canvas 的普通流占高、desktop rotor 与 face 高度不一致、360px custom header 越界和移动记录行零宽；没有改变历史 DTO、repository、统计计算或 emits。
- U18 定向回归覆盖 1440/1024/390/360px 的 trend、heatmap、priority、radar、front/back flip、趋势范围、筛选、批量选择和三项 action；四视口通过且 route/document 无横向溢出。
- 全量门禁证据：static suite `18/18`；packaged Tauri practice flow 所有 checks 通过；`cargo test --workspace --target-dir target/gate` 全部通过。
- U19 冻结为 Reading History record detail and batch-action continuity。它只审计记录卡的内容层级、分数结果、删除/批量动作、loading/empty/error 与窄屏可达性；不把 Reading History 与全局 `HistoryPage` 合并，也不改变后端合同。

## 2026-08-08 U19 Reading History record continuity

- Reading History 记录标题已经是现有 review route 的唯一详情入口；不复制 opensource 的旧 `practiceRecordModal.js` runtime，避免引入第二份历史读取和回放逻辑。
- 发现并修复批量 checkbox 的真实事件冒泡缺陷：row `click` 与 checkbox `change.stop` 会在一次点击中切换两次；`ReadingHistoryPanel.vue` 现在在 checkbox click 上停止冒泡，Set 选择模型和 API 不变。
- `opensource-skin.css` 现在拥有记录卡分数 surface、checkbox 尺寸、删除动作 44px target 和 muted meta 的 route-scoped owner；桌面四列和移动结果行布局继续由 `PracticeLibraryPage.vue` 负责。
- 定向回归 `reading_history_record_visual_check.py` 通过 1440/1024/390/360px：非空记录、长标题、score/delete/action geometry、bulk checkbox on/off、review navigation continuity、delete no-navigation 均通过。
- U19 全量门禁通过：Vue typecheck/build、static suite `18/18`、packaged Tauri practice flow、Rust workspace tests。
- U20 冻结为 Global History batch-action mobile continuity；继续保持全局 `HistoryPage` 与 Reading History 的数据结构、评分尺度和 DOM 独立。

## 2026-08-08 U20 Global History batch continuity

- 全局 `HistoryPage.vue` 的 `.batch-actions` 在本地 scoped CSS 中是 flex，但 960px 断点只写 `grid-template-columns`，因此移动端布局规则无效；这是真实的可复现视觉缺口。
- 只在 `opensource-skin.css` 增加 `.atlas-source-ui .history-page.history-page .batch-actions` owner：桌面/平板允许自然 wrap，手机计数 `flex-basis:100%`，两个动作保持 44px。
- `history_batch_visual_check.py` 复用现有 History detail fixture，不改变列表 DTO 或 invoke mock；四视口均无 document/body 横向溢出，按钮和选择计数均在可见边界内。
- U20 全量门禁通过：Vue typecheck/build、static suite `18/18`、packaged Tauri practice flow、Rust workspace tests。
- 下一切片 U21 只审计全局 History 的 filter/analytics 移动密度；不得借机合并评分尺度、改统计查询或搬入 opensource JS。

## 2026-08-08 U21 Global History filter/analytics density

- `filter-panel` 和 `statistics-section` 的 `overflow: hidden` 原本会把超宽 comparison table 直接裁掉；父 surface 保留装饰裁切，但数据表现在由 `.comparison-table-scroll` 提供独立 `overflow-x: auto`。
- 真实 shrink 边界是 filter range inputs、analytics grid children 和 cards；统一 `min-width: 0` 后，1024/390/360px 均无 page-level overflow。
- 390/360px 的 task/date/score/search/statistics range/reset 控件已统一达到至少 44px；日期与分数双输入继续保留原语义和同排结构。
- 筛选控件补齐稳定 `id`、`for` 与上下界 `aria-label`，不改变 `v-model`、watch、query conversion 或 Tauri DTO。
- `history_filter_analytics_visual_check.py` 覆盖 mixed 1440/1024/390/360px，以及 390px writing-only / reading-only；reading-only 保持 trend-only，writing-only 保持 radar/comparison。
- 人工抽查 desktop 与 360px 全页截图：filter、radar、comparison、trend、records 无重叠；移动 DOM 顺序不变，底部导航未遮挡内容，table 只在自己的 viewport 内滚动。
- 审计另确认 mixed trend 仍把 writing Band 乘 10 后放进 `%` 轴，writing 轴下界仍固定为 4；U21 明确不改图表数据，后续 U22 必须先冻结用户可见语义再做最小调整。

## 2026-08-08 U22 Global History trend scale

- `LineChart` 只有一个 axis/suffix 合同，不能正确承载 reading ratio 与 writing Band；mixed 数据必须在 presentation 层拆成独立同量纲 series，双轴方案属于无收益复杂度。
- 保留原“按提交时间排序后取最近 15 条”的窗口，再按 `scoreScale` 分组；旧记录只用 `activity` 作为 fallback，不改历史查询或 DTO。
- 写作图使用完整 `0-9` Band 域和 `Band ` tooltip prefix；阅读图使用 `0-100%`。mixed 同时展示两图，禁止再出现 Band 7.5 -> 75%。
- 首轮 split-series 截图暴露 `LineChart` 单点面积路径以中心点闭合成三角形；单点折线横跨整图时，面积必须用左右 padding 边界闭合成矩形。
- 最终 desktop/360px 截图确认两张趋势图在同一 analytics surface 内以分隔线区分，没有 card-in-card；单点填充为完整基线矩形，最近记录与底部导航无重叠。
- U22 未新增 Tauri command；回归实测调用仍只有 `list_history` 与 `history_writing_statistics`。

## 2026-08-08 U23 Global History recent-record and pagination audit

- `opensource` 只提供最近记录的信息层级参考：标题、日期/时长、右侧分数、普通态隐藏 checkbox 与批量态选中反馈；它没有 History pagination 或 task badge 合同。
- 参考树移动端仍保留四列记录 grid、18px checkbox 和 hover-only 32px 删除按钮，这些是不可迁移的触控缺陷；U23 必须以当前 Vue 合同建立自己的移动几何。
- 当前 Vue 在 21 条记录的第 2/2 页删除唯一记录后仍以 `offset: 20` 重载，收到 `total: 20/items: []` 后隐藏分页并错误显示空态，第一页 20 条记录不可达。应在 `loadEssays()` 统一钳制超界页码，覆盖单删、批删和外部数据收缩。
- `.essay-list { overflow:hidden }` 会把无断点长 token 静默裁切；页面级 `scrollWidth` 断言无法发现子元素内容丢失，定向回归必须检查 title `scrollWidth/clientWidth` 或 wrapping。
- checkbox 实际只有 18x18，查看/删除按钮为 34x44；390/360px pagination 仍是 nowrap flex，旧 `grid-template-columns` 写在 flex 元素上完全无效。
- U23 保持 `HistoryViewModel`、页内选择语义、详情 overlay、阅读 review route 和删除 commands 不变；不把 `opensource` 的旧 JS/DOM/hover 交互搬入 Vue。
- 初版移动 pagination 虽满足 `display:grid` 和两列宽度，但 DOM 顺序导致上一页、页码、下一页占三行且第二列为空；仅检查 display/columns 会假绿，必须断言 page-info 在按钮上方且两个按钮同一行。
- 最终移动 pagination 为明确两行：page-info 跨首行，上一页/下一页在第二行两列；390/360px 高度 93px，按钮均为 50px。
- 人工核验 1440/1024/390/360 全页截图：记录标题、实际时长、score pod、checkbox 和 action 保持清晰层级；移动长 token 在卡内换行，score/action 未被推出，pagination 与导航无重叠。

## 2026-08-08 U24 Global History page-state audit

- 当前列表 loading 是裸 `.loading` 文本，error/empty 才有 card；状态切换没有共同 surface/语义合同。
- `HistoryPage.vue` 原本给 empty-state `min-height:132px`，但 shipping skin 以更高特异度覆写为 `min-height:0;padding:18px 22px`，无历史状态会收缩成一条浅色横条。
- error 没有 `role=alert`，loading 没有 `role=status`/`aria-busy`，empty/filter-empty 没有 status 语义；重试和 reset 动作逻辑本身清晰，可原样保留。
- `opensource` 仅在 `index.html:760-769` 提供 clipboard + 主文案 + 次级说明空态；没有 loading/error/filter-empty 或 1440/1024/390/360 证据，不能搬迁旧 JS 或臆造状态合同。
- 最小结构是四种状态共用 `.history-list-state`，通过 modifier 表达 loading/error/empty；统一稳定高度、内容层级和 44px action，保持请求 gate/重试/reset 不变。
