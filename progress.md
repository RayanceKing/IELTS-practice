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
- 紫色品牌特殊情况清除：`--color-brand-*` → teal/amber + shui 渐变；`--primary-soft`/`suite` overlay/`btn-brand` 改 glass warm；Library 本地 brand 重声明与 History/Settings 硬编码紫渐变删除。
- 原生路径推进：`useReadingCoach` 进 strict；`useReadingUiPreferences`/`useReadingEndlessState` 走 Tauri SQLite settings；Shui 背景主题迁出 localStorage；删除答案/提交 sessionStorage 死写路径。

## 2026-08-07

- 用户确认实施 Application 层方案，并要求设置持续目标执行到底。
- 已创建持续目标：新增 `ielts-application`、迁移写作评估/Coach、抽离 AI runtime、保持行为并通过全量门禁。
- 已运行 planning-with-files session catchup；工作树干净，无未同步或未提交改动。
- 已完成现状基线复核：Application 层只处理跨数据库/LLM/事件的垂直用例，普通 CRUD 保持直连 `ielts-db`。
- 重构前执行 `cargo test --workspace`：编译在 `phase7_modes` 4 个既有测试的缺失字段处失败；已与本次改动隔离记录。
- 子代理完成 AI/runtime 依赖侦察：确认 `commands::ai` 可机械拆为 Tauri `ai/config.rs` 与 `ai/runtime.rs`，Application 不得引用宿主状态。
- 已新增 `ielts-application` workspace crate，落地 `ApplicationError`、`LanguageModel`、写作/Coach store、`EventSink` 及两个垂直 service；尚待编译校验和 Tauri 接线。
- Application 层接线完成：`writing_start_evaluation` 与 `coach_run` 已收缩为 Tauri 适配器，AI config/runtime 移入 `src-tauri/src/ai`，旧 provider 模块删除。
- 新增 14 个 application 单元测试，覆盖幂等恢复/事件重放、模型失败/非法 JSON、feedback degraded、Coach 成功与失败、runtime 未配置，以及模型 await 期间 store mutex 已释放。
- 修复四类既有门禁夹具漂移：endless timer 字段、migration v10 断言、single reading timer snapshot、submitted writing 不应直接进入历史。
- 完整验证通过：`cargo test --workspace --target-dir target/gate`、Vue typecheck/build、static suite `18/18`、packaged Tauri E2E 全部通过。
- 用户要求继续建立 Agent 后端并授权自主决策；已创建持续目标，首期范围冻结为工具调用循环、持久化审计、短期 workspace grant 和受限文件读写，不包含 UI、shell 或任意网络工具。
- `planning-with-files` session catchup 首次因 Windows GBK 无法输出 Unicode 失败；改用 UTF-8 后恢复完成，Application 阶段没有未同步代码。
- Agent 三面只读审计完成：冻结独立模型协议、v11 审计表/v7 备份、短期 workspace grant 与 canonical path containment 方案；进入 application 工具循环实现。
- G2 完成：新增独立 `AgentModel`、枚举消息、脱敏工具审计端口及有限顺序循环；19 个 application 测试通过，覆盖成功、未知工具、provider 失败、轮次/调用上限和模型 I/O 无锁。
- G3 完成：SQLite v11 增加 `agent_runs`/`agent_tool_calls`，实现 begin/finish、终态保护和启动中断恢复；备份升 v7 并冻结 v6 清单，数据库生命周期/迁移/备份共 20 个定向测试通过。
- G4 完成：`AiRuntime` 并列实现 OpenAI-compatible `AgentModel`，支持 nullable content/多 tool calls；文件工具实现 UTF-8/1 MiB/相对路径/canonical containment/SHA-256 乐观锁/原子替换，18 个 Tauri lib 测试通过。
- G5 完成：新增短期进程内 `WorkspaceGrants`、原生目录选择、`agent_run`/`agent_get_run` commands、application store 审计适配和启动恢复；Tauri lib 测试扩展至 21 个。
- G6 安全测试完成：覆盖绝对/父目录/敏感路径、目录外 symlink、非 UTF-8、读写尺寸上限、哈希缺失/冲突、原子临时文件清理、重复 tool id、轮次/调用上限和 DB lock 无跨 I/O。
- Agent 第一阶段目标完成：Rust workspace 全绿；Vue typecheck/build 全绿；static suite 18/18；packaged Tauri E2E 的启动、路由、UI、reading IPC、备份边界、资源与 SQLite 重启检查全部通过。
- 新层级收敛目标已建立：坚持单机本地边界，不新增 crate、通用 repository、权限引擎或分布式状态；先修真实职责和一致性缺陷。
- H1 审计完成：确认普通 CRUD 直连 `ielts-db` 是合理的，ApplicationStore 的短锁端口有价值；定位 writing provider TOCTOU、AI vault 持锁 I/O、Agent 部分副作用和重复 DTO 等问题。
- H2 首个窄切片完成：`writing_start_evaluation` 将已选 `AiProviderConfig` 快照传入后台 runtime 构造，不再重新读取默认配置；保留 durable handle 与后台失败落库语义。Application 测试新增 provider/model 快照断言，`cargo test -p ielts-application` 21/21、Tauri crate `cargo check --lib` 通过。
- H2 延伸切片完成：Agent 在执行任何工具前整批校验 call id/name/剩余配额，重复 ID 与同批超限不再产生部分文件副作用；写作公开 `list_events`/`cancel` 纯转发已删除，command 直接使用 store trait；Coach 空问题在 application 入口拒绝，fake store 不再复制业务校验。
- H3 完成：普通 CRUD 继续直连 `ielts-db`；未引入通用 repository。AI 配置和备份恢复改为 SQLite 元数据快照 -> 锁外 vault/keyring 检查 -> 短事务 reconciliation；runtime API Key 解析也不再在 SQLite mutex 内访问凭据。
- H4 当前进展：保留 workspace containment、SHA-256 乐观锁、原子写入和大小/UTF-8 限制；移除 `AiRuntime` 的 Debug/Clone 派生，避免密钥被格式化或复制。Tauri lib 21/21 通过。
- H4/H5 收口：完整 Rust workspace、Vue typecheck/build、static suite 18/18 与 packaged Tauri practice E2E 均通过；静态 AI gate 适配两阶段 vault reconciliation 后恢复绿色。
- H6 下一窄切片已确定：优先消除 `ielts-application/src/agent.rs` 与 `ielts-db/src/agent/mod.rs` 的同构 run/tool audit DTO 和 status 映射；保留 AgentStore 的短锁测试边界，不删除已有历史能力，不新增第三套契约。
- H6 首个切片完成：`AgentStore` 直接接收 `ielts-db` 的 begin/finish audit commands，Application 删除 `AgentRunStart/Finish`、`AgentToolCallStart/Finish` 和重复 status enum；ApplicationStore 变为四个短锁直通调用。既有 21 个 Application 测试与 21 个 Tauri lib 测试通过。
- H6 下一审计点：Agent 工具执行后若 audit DB finish 失败，当前会以 `agent.persistence_failed` 中断并留下 running 记录；下一轮先补 fake store characterization test，再决定 best-effort audit 或显式 uncertain 终态，避免把文件副作用和用户结果混为一谈。

## 2026-08-07 Agent audit finish 语义

- 补充 fake store 的一次性 `finish_tool_call` / `finish_run` 注入失败能力。
- 固定三条低复杂度合同：工具完成审计失败立即停止并返回持久化错误；成功 run 收尾失败直接返回持久化错误；失败路径的补偿性收尾失败只记录、不覆盖原始 provider/校验/限额错误。
- 复用既有 `running -> interrupted` 启动恢复，不新增 `uncertain` 状态、重试层或补偿事务。
- 新增 3 个 application characterization tests；`cargo test -p ielts-application` 24/24，`cargo test --workspace --target-dir target/gate` 全绿。

## 2026-08-08 Vue 视觉搬迁目标

- 旧后端层级目标已完成并归档；新持续目标已建立：只把 `opensource` 的视觉/交互连续性搬到当前 Vue，不搬 JS 架构、存储或运行时。
- UI 路线拆为 U1 盘点基线、U2 全局壳层、U3 Library/History/Settings、U4 Writing、U5 Reading/Coach、U6 Agent 视觉入口、U7 回归清理。
- 当前处于 U1，待完成两个工作树的页面/组件映射和第一批低风险视觉切片选择。
- U1 盘点完成：opensource 是静态 index + CSS，无可迁移 Vue；视觉可复用范围冻结为壳层、Library/History/Settings/Reading 的结构与状态，明确不搬 JS/data/DOM 协议。
- U2 首个切片完成：NavBar 改为 IELTS Atlas 品牌头部与带图标的总览/题库/写作/记录/设置导航，App 主内容宽度与 padding 对齐 opensource 的 1400 shell；路由、DTO、数据流未改。
- 验证：`npm --prefix apps/writing-vue run typecheck`、`npm --prefix apps/writing-vue run build` 通过。
- U2 收口：导航 emoji 改为稳定的内联 SVG，导航宽度统一使用 `--shui-shell-max-width`，移动端横向滚动固定为不换行且隐藏滚动条；U2 标记完成。
- U3 启动：Reading Overview 增加 opensource 风格的 eyebrow/subtitle 层级与 P1/P2/P3 视觉标记，保留所有 Vue 事件、`data-*` 测试锚点和 Tauri 数据流。
- 390x844 packaged WebView 实测发现 Library 的 tabs 固有宽度把页面撑到 540px；通过给 header/tabs 明确 `min-width: 0`、tabs 自身承接横向滚动并隐藏滚动条，页面 `scrollWidth` 收敛到 375px，无全局横向溢出。
- U3 Browse 工具区完成第一轮视觉搬迁：图标化搜索、偏好浮层、玻璃工具栏与空态图标落地；desktop/mobile 截图无页面级溢出。视觉核对同时发现 author CSS 覆盖原生 `hidden`，已用 Library 局部 `[hidden]` 规则恢复未开放“听力”筛选等隐藏语义。
- 本切片最终验证：Vue typecheck/build 通过，static suite 18/18，packaged Tauri E2E 全通过；390x844 Browse 快照 `scrollWidth=375`，可见类型筛选仅“全部/阅读”。

## 2026-08-08 Vue 视觉搬迁 Reading History

- Reading History 列表恢复 opensource 的桌面双列布局，900px 以下回到单列；loading/empty 状态横跨全部列。
- 历史 header 的筛选、导出、批量删除和清除动作按参考布局重新排列；搜索框去除不存在的前置图标留白。
- 移动端记录结果区保持分数与删除动作同一 flex 区域；批量选择、标题详情和删除事件节点未改变。
- 统计卡增加品牌 accent 条，批量删除状态使用虚线边框，删除按钮默认弱化并在 hover/focus 时强调。
- 验证：Vue typecheck/build、`python developer/tests/ci/run_static_suite.py`（18/18）、`python developer/tests/e2e/suite_practice_flow.py`（全部 packaged checks 通过）。
- 390x844 packaged WebView History 实测 `scrollWidth=375`，History section 与空态完整显示，无页面级横向溢出；截图为 `developer/tests/e2e/reports/library-history-mobile-section.png`。
- U3 当前完成 Overview、Browse、History 三个低风险切片，下一块冻结为 Settings 概览分区视觉，不触碰 settings command、DTO 或备份流程。

## 2026-08-08 Vue 视觉搬迁 Settings AI 卡

- 仅在 `opensource-skin.css` 增加 AI 与评测总览卡的 accent、动作区分隔和统计 badge 层级；SettingsPage 模板、脚本、Tauri DTO 与事件均未改。
- 保留四个 `data-settings-open` 入口及 `openSettingsDetail` 行为；详情 modal 几何留作后续独立切片。
- 验证：Vue typecheck/build、static suite 18/18、packaged Tauri E2E 全部通过。
- 390x844 packaged WebView Settings 实测 `scrollWidth=375`，AI 卡片四个入口按列换行、统计 badge 不溢出；截图为 `developer/tests/e2e/reports/settings-mobile.png`。
- Settings detail modal 已补齐 fixed scrim、最大高度、内部滚动、tabs 横向滚动和关闭按钮状态；modal 打开时锁住 app shell 底层滚动。
- 390x844 packaged WebView 点击 API 配置实测 `position=fixed`、`appOverflow=hidden`、`scrollWidth=375`，Escape 后 modal 正常卸载；截图为 `developer/tests/e2e/reports/settings-modal-mobile.png`。
- U3 的下一个切片冻结为全局 History 页面表面与弹层几何；不与历史查询/图表数据逻辑混合。

## 2026-08-08 Vue 视觉搬迁 Global History

- 全局 History 的筛选与分析 surface 增加 opensource 风格的顶部品牌 accent；历史记录列表恢复玻璃容器、行间分隔、hover 状态和右侧分数 pod 层级。
- 详情与删除 overlay 补齐 fixed scrim、bounded dialog、内部滚动和移动端顶部对齐；modal 打开时锁住 app shell 底层滚动。
- 保留筛选字段、图表组件、分页、选择/删除按钮和所有现有 Vue 事件；未修改 history repository 或分数计算。
- 390x844 packaged WebView 实测 `scrollWidth=375`，筛选区单列堆叠且 header action 不溢出；截图为 `developer/tests/e2e/reports/history-mobile.png`。
- 验证：Vue typecheck/build、static suite 18/18、packaged Tauri E2E 全部通过。
- U3 收口，下一阶段转入 U4 Writing 页面布局与评测状态展示。

## 2026-08-08 Vue 视觉搬迁 Writing Compose

- 仅在 `apps/writing-vue/src/styles/opensource-skin.css` 增加 Compose 路由级视觉 owner；未改 `ComposePage.vue` 模板、脚本、草稿、题目、提交或评估调用。
- 双栏工作区统一到 opensource 的 glass surface 层级；左侧 hero/指标/config toolbar、Task/题目来源 segmented controls、prompt surface、编辑器 header/footer、字数警告和确认 dialog 使用现有 `--atlas-*` / `--lg-*` token。
- Compose dialog 增加 fixed scrim、bounded surface 与 app shell 滚动锁；移动端保持既有单列、按钮堆叠和 textarea 几何，不引入页面级横向裁切。
- 验证：`npm --prefix apps/writing-vue run typecheck` 通过；`npm --prefix apps/writing-vue run build` 通过；`python developer/tests/ci/run_static_suite.py` 为 `18/18 passed`；`python developer/tests/e2e/suite_practice_flow.py` packaged Tauri 全部 checks 通过。
- 当前 packaged Compose desktop 仍保持 `scrollWidth=1425`、viewport `1440`、无 offender；下一窄切片冻结为 Result 评分/反馈 surface 与移动端单列布局。

## 2026-08-08 Vue 视觉搬迁 Writing Result

- 仅在 `opensource-skin.css` 增加 Result 路由级视觉 owner；保留 `ResultPage.vue` 的 sessionId、结果加载、viewMode、句子展开、v-html 标注和 router 事件。
- 评分 summary、评分环、metrics grid、degraded warning、feedback/rationale cards、essay body 和 error details 统一使用 opensource glass hierarchy；桌面维持双列，1040px 以下单列，640px 以下指标改为单列并让操作按钮可换行。
- 修复既有 `.text-primary/.score-fill/.bullet` 合并规则：标题只设置文字色，评分环只设置 SVG stroke，bullet 只设置背景色，避免标题被错误涂成整块 accent。
- 验证：Vue typecheck/build、static suite `18/18`、packaged Tauri E2E 全部通过；Compose/Library/Topics/Settings/History UI snapshots 仍无 offenders。
- U4 下一窄切片冻结为 Evaluating 状态 rail；继续只改视觉，不碰 event listener、取消/重试和 Result/Compose 路由。

## 2026-08-08 Vue 视觉搬迁 Writing Evaluating

- 仅在 `opensource-skin.css` 增加 Evaluating 路由级视觉 owner；保留 Tauri evaluation event listener、sequence 去重、取消/重试、Result/Compose 路由和错误状态机。
- 作文面板、AI hero、aurora orb、进度 rail、错误卡、实时日志和 action row 统一到 Atlas glass surface；局部旧紫/蓝 conic-gradient 改为现有 accent token，动画和进度宽度绑定不变。
- 1040px 以下改为单列，640px 以下沿用紧凑卡片与全宽动作按钮；没有页面级 `overflow-x:hidden`。
- 验证：Vue typecheck/build、static suite `18/18`、packaged Tauri E2E 全部通过，既有 Compose/Library/Topics/Settings/History 路由快照无变化性失败。
- U4 下一步是专用移动视口截图核验，完成后转入 U5 Reading/Coach 视觉切片。

## 2026-08-08 Vue 视觉搬迁 Writing 移动核验

- 使用同进程临时 HTTP server + Playwright 对 390px/320px 视口核验 Compose、Evaluating、Result；临时 server 在脚本退出时关闭，未修改产品/测试源码。
- Compose 390/320px 分别为 `rootWidth=358/288`、`scrollWidth=390/320`；Evaluating 同样无 route 内 offender。
- Result 注入真实形状的 Tauri history detail，实际渲染 `7.5` 评分环、4 个 metric cards、feedback/plan/review/rationale/task analysis；390/320px 均 `scrollWidth === viewportWidth` 且 `rootOffenders=[]`。
- 截图：`compose-mobile-390.png`、`evaluating-mobile-stub-390.png`、`result-mobile-stub-390.png`（另有 320px 版本）。
- U4 收口并标记 completed；U5 Reading/Coach 进入 in_progress，继续保留现有 reading composable 和 Tauri command。

## 2026-08-08 Vue 视觉搬迁 Reading Coach

- 仅在 `opensource-skin.css` 增加 Reading 路由级 Coach skin；`ReadingCoachPanel.vue`、`PracticeReadingPage.vue` 模板、`useReadingCoach.ts`、Coach API 和提交后可见性均未改。
- FAB 与 panel 的底部位置统一从 `--atlas-reading-nav-height` 派生，分别位于底部导航上方 12px/64px；层级降至 2200，使 settings/notes modal 的 2500/2600 scrim 保持最高所有权。
- 消息区恢复 `touch-action: pan-y`，按钮/chip 使用 `manipulation`；关闭、chip、输入与发送控件保持至少 44px 触控目标，640px 以下 composer 单列。
- FAB/send 文字固定高对比白色，panel 使用 `100dvh`、safe-area 和 Atlas glass token；未改变“询问教练”文案、ID、`data-*`、ARIA 或 emits。
- 验证：Vue typecheck/build 通过，static suite `18/18`，packaged Tauri E2E 的启动、路由、Reading IPC、notes dialog、提交、备份、资源和 SQLite 重启 checks 全部通过。
- U5 保持 in_progress；下一步先做 1440x900、1280x720、390x844、360x640 的 Coach 几何核验，再处理题目导航/标注工具栏。

## 2026-08-08 Reading Coach 几何与标注工具栏

- 用构建产物的入口 CSS + `PracticeReadingPage` CSS chunk 做四视口计算样式核验；测试为临时 HTTP server + Playwright，未写入产品或测试源码。
- 1440x900、1280x720、390x844、360x640 下，Coach FAB/panel 分别保持导航上方 12px/64px；panel `z-index=2200`、overlay `z-index=2500`，消息区 `overflow-y:auto`、`touch-action:pan-y`。
- 四个视口均无页面横向溢出；360x640 最低高度下 panel 顶部仍为 32px，composer 保持可见，消息区独立滚动。
- 从 Reading 通用 glass surface 列表移除 `.reading-selection-toolbar`，并以 `#selbar` 建立深色 action strip owner；背景固定为 opensource 合同 `#1e293b`，hover/focus 为 `#334155`，按钮保持透明白字。
- 最终验证：Vue typecheck/build、static suite `18/18`、packaged Tauri E2E 全部通过；计算样式确认 `#selbar=rgb(30,41,59)`、按钮透明白字。
- U5 下一窄切片转入题目导航几何与状态层级，保留 `ReadingAnswerNav.vue` DOM、事件和现有扁平 `questionOrder`。

## 2026-08-08 Reading 题目导航

- 定向基线确认 112px 移动底栏存在真实纵向溢出：820px 内容高 152px，390px 为 210px，360px 为 228px；题号行落到 viewport 下方，操作按钮被压缩到 40-60px。
- 仅在 `opensource-skin.css` 增加 route-scoped 响应式 owner：980px 以下使用 128px 两行 grid，640px 以下使用 184px 导航和三列两行 44px 操作网格；题号列表继续横向滚动。
- 套题进度块在移动端改为单行 flex，消除首个 grid 项 51.9px 高导致的轻微行叠压；六个操作项最终全部稳定为 44px。
- 最终定向结果：820px 导航 `scrollHeight/clientHeight=127/127`；390/360px 为 `183/183`，操作区 `94/94`，document `scrollWidth` 分别为 390/360。
- Vue typecheck/build、static suite `18/18`、packaged Tauri E2E 全部通过；packaged desktop Reading 截图确认分栏和底部状态层级正常。
- U5 标记 completed，U6 进入 in_progress；下一阶段只处理 Agent 视觉入口与工作区交互，不扩后端 command/DTO。

## 2026-08-08 Agent 工作区视觉切片

- 新增 `AgentWorkspacePage.vue`、`/agent` route 和 NavBar 入口；入口位于写作与历史之间，390px 导航首屏可见。
- 页面使用已有 Vue 本地状态实现文件选择、提示词模式、上下文切换和模拟运行，不调用既有 Agent Tauri commands，也不新增 repository、DTO 或持久化。
- `opensource-skin.css` 新增 route-scoped 工作区样式：桌面三栏、980px 两栏、760px 单栏；全部使用现有 `--atlas-*` / `--lg-*` token。
- 新增 `developer/tests/e2e/agent_workspace_visual_check.py`，验证 1440x900、980x720、390x844、360x640 的页面宽度、grid 几何、移动导航滚动和本地状态交互，并输出四张截图。
- 视觉检查发现并修复屏幕阅读器标签意外可见；移动导航重新排序后 Agent 活跃项在首屏可见。
- 最终门禁：Vue typecheck/build 通过；static suite 18/18；packaged Tauri practice E2E 全部通过；四视口 `documentScrollWidth === viewportWidth`。

## 2026-08-08 U7 全量视觉回归与 CSS 收口

- PostCSS 按 root/media 父级上下文检查 `opensource-skin.css`，确认只有 History `.essay-score-pod` 与 Result `.result-layout` 是同作用域重复；已合并/删除，其他断点重复保留。
- 只删除 5 条完全由 canonical skin 接管的旧声明：Compose `.hero-chip` background、Compose mobile `.practice-left` padding、Result `.text-primary` color、Reading Coach error color、Evaluating layout min-height；没有删除状态、结构或响应式规则。
- 静态 shell contract 新增 Agent route/nav 断言；packaged Tauri route visual 新增 `#/agent` 与 `[data-agent-workspace]` 截图，desktop `scrollWidth=1440`、无 offenders。
- `cargo test --workspace` 全绿；Vue typecheck/build、static suite `18/18`、packaged E2E 全绿；U7 已收口。持续目标保持 active，下一切片等待新的 UI 窄边界冻结。

## 2026-08-08 U8 题库管理工作区

- 已完成只读审计并冻结 U8：只把 opensource 的工作区密度、surface 层级、状态节点和弹层几何迁入 `TopicManagePage`，不搬迁 opensource JS 架构。
- 已更新 `task_plan.md`、`findings.md`；新增 Topic 路由级 skin、bounded modal、移动断点和上传区键盘触发器。
- `npm --prefix apps/writing-vue run typecheck` 通过；`npm --prefix apps/writing-vue run build` 通过。
- `python developer/tests/e2e/topic_manage_visual_check.py` 通过：1440/980/390/360px 无页面横向溢出，卡片/弹层几何稳定，Enter 触发文件选择器通过。
- `python developer/tests/ci/run_static_suite.py` 通过（18/18）；`python developer/tests/e2e/suite_practice_flow.py` packaged Tauri 全部 checks 通过；`cargo test --workspace --target-dir target/gate` 全绿。
- U8 已收口；Settings confirmation overlay 与嵌套 surface 审计记录为下一窄切片，持续目标保持 active。

## 2026-08-08 U9 Settings 弹窗

- U8 完成后立即冻结 U9：先修 Settings 二级弹窗的真实几何缺口，仍只改视觉层，不接触设置数据流。
- 已新增 Settings route-scoped overlay shell、bounded dialog、内部滚动、二级层级和弹层期间的 app-main stacking 修正。
- 已新增 `developer/tests/e2e/settings_modal_visual_check.py`，覆盖 1440/1024/390/360px 的 onboarding、update、confirm 弹窗几何及底层滚动锁。
- 定向脚本首轮误点了第一步禁用的「上一步」按钮，改为点击遮罩自身关闭后通过；该错误属于测试选择器，未涉及产品逻辑。
- 全量门禁通过：Vue typecheck/build、static suite `18/18`、`cargo test --workspace --target-dir target/gate`、packaged Tauri E2E 全绿；U9 已收口。

## 2026-08-08 U10 Settings detail surface 层级

- Settings detail 只读审计指出 `.settings-panel`、`danger-zone`、`about-section` 等内部 raised surface 与外层 detail panel 重复使用大阴影；本切片冻结为 CSS-only 层级减重。
- 已在 `opensource-skin.css` 中保留 detail 外层高层 surface，将内部 panel/list/mode/about 改为 soft/control surface，危险区保留语义色并去除重复阴影。
- 扩展 `settings_modal_visual_check.py` 覆盖模型参数、数据管理、关于、提示词、API 配置五个 tab；1440/1024/390/360px 全部通过。
- 全量门禁再次通过：Vue typecheck/build、static suite `18/18`、Rust workspace tests、packaged Tauri E2E；U10 已收口，goal 继续 active，U11 冻结为 History detail 状态层级。
# 2026-08-08 U11 History Detail

- 将 U11 标记为 in progress。
- 完成 History 详情 DOM、局部样式和 opensource modal/surface 经验的定向审计。
- 在 `opensource-skin.css` 增加 History detail route-scoped owner，恢复总分锚点并降低内部重复卡片重量。
- 为详情请求返回空数据的情况补充只读空态；未改 API、DTO、删除流程或 Vue 数据流。
- 新增 `history_detail_visual_check.py`，覆盖 success/loading/error/empty、关闭/重试和四个目标视口。
- U11 验证通过：Vue typecheck/build、History 四状态四视口、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- 桌面与手机截图已人工抽查；移动端保留原 DOM 阅读顺序，未用 CSS `order` 制造视觉/辅助技术顺序分裂。
- 冻结 U12 为窄屏主导航完整可见；`NavBar.vue` 在 640px 以下使用 3×2 网格并补齐主导航/当前页语义，未改 route 配置。
- 新增 `nav_responsive_visual_check.py`，覆盖六个视口、阅读/Agent/历史/设置路由、六入口可见性、触控高度和键盘 focus。
- U12 首轮脚本断言在 Agent 文案上暴露编码不稳定，已改为按稳定 href 检查 active 状态，未改变产品代码。
- U12 验证通过：导航六视口四路由回归、Vue typecheck/build、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- 下一切片冻结为 U13 Reading Suite session surface 层级；总体视觉搬迁目标保持 active。
- U13 审计完成：确认套题 API、frameless route 和 row 状态合同，无需修改数据流；开始实现 route-scoped surface owner。

## 2026-08-08 U13 Reading Suite

- 在 `PracticeReadingSuitePage.vue` 增加只读空态，并为错误/加载状态补齐 `role`、`aria-live`、`aria-busy`；`interrupted` 套题状态显示为“已中断”。
- 在 `opensource-skin.css` 增加 Reading Suite route-scoped hierarchy：header 无框，summary/passages 为 raised workspace，passage rows 为 control surface；active/submitted/pending 使用 selected/info/muted 状态面。
- 修复真实 DTO 状态命名：Rust passage status 是 `pending`，皮肤选择器从不存在的 `passage-row--locked` 改为 `passage-row--pending`。
- 新增 `developer/tests/e2e/reading_suite_visual_check.py`，使用 Tauri `suite_get` mock 覆盖 success/loading/error/empty、active/submitted/pending 三行、继续当前路由、frameless shell、无横向溢出和四视口截图。
- 定向视觉回归通过：1440x900、1024x768、390x844、360x800。
- U13 全量门禁通过：Vue typecheck/build、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- 下一窄切片冻结为 U14 Reading Library tools/settings surface 层级；持续 UI 搬迁目标保持 active。

## 2026-08-08 U14 Reading Library surfaces

- 仅在 `opensource-skin.css` 增加 Reading Library route-scoped visual owner：More Tools 外层保持 raised，tool cards 降为 control，featured card 保留 selected surface。
- Reading Settings 的四个子 panel、system info、config list 降为 soft/control surface；保留所有 props、emits、archive input 和 repository 行为。
- clock/PDF overlay 增加 `dvh` bounded 尺寸、44px close target、移动端单列/全宽几何；overlay 打开时提升 `.app-main` stacking context，修复导航拦截 modal 操作的真实 bug。
- 新增 `developer/tests/e2e/reading_library_surface_visual_check.py`，Tauri mock 覆盖 More Tools、Settings、题库 config、clock、PDF 五类表面，四视口无页面级横向溢出。
- U14 定向回归通过：1440x900、1024x768、390x844、360x800。
- U14 全量门禁通过：Vue typecheck/build、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- 下一窄切片冻结为 U15 Reading SuiteSelector modal chrome；持续 UI 搬迁目标保持 active。

## 2026-08-08 U15 Reading SuiteSelector modal chrome

- U15 审计确认 `ReadingSuiteSelector.vue` 的 props/emits、模式值、频率 select 和取消/背景关闭事件不需要改动；缺口集中在 shipping skin 的 modal header/close/body/option chrome。
- 增加 route-scoped modal chrome：header 两端对齐、圆形关闭按钮、body 内部滚动、muted subtitle、选项纵向可换行排版、右对齐 actions 和移动端 bounded padding。
- 复用 U14 的 stacking 修复：SuiteSelector 打开时提升 `.app-main` 到导航层级之上，并锁定背景滚动，确保 scrim/close/frequency 操作可达。
- 新增 `developer/tests/e2e/reading_suite_selector_visual_check.py`，覆盖 1440/1024/390/360px 的 closed/open、active/unselected option、frequency select、cancel、backdrop close、focus treatment、stacking 和无横向溢出。
- U15 定向回归通过，desktop/mobile 截图人工抽查确认 modal header、关闭按钮、选项描述、频率选择和取消动作均保持可见且无重叠。
- U15 全量门禁通过：Vue typecheck/build、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- U15 标记完成；下一窄切片冻结为 U16 Custom Suite selection bar，总体视觉搬迁目标保持 active。

## 2026-08-08 U16 Custom Suite selection bar

- 只在 Reading Library skin 中增加自选套题 workflow surface：复用 `selecting/ready`、`filled` 和 `disabled` 既有状态，没有新增 Vue 状态或修改 emits。
- 桌面将 P1/P2/P3 进度、当前提示和确认动作组织为紧凑控制条；手机切成单列 chip 与双列 44px 动作区，长标题允许正常换行。
- 首轮回归抓到旧通用 `:is()` surface 的特异度高于新 owner；用 route-scoped 双类选择器收口，没有引入 `!important`。
- 加严断言后发现移动 media rule 被桌面 owner 压制，曾把 chip 压到 22px；提升断点规则到同等特异度并增加 chip/action 最小宽度检查，消除假绿。
- 新增 `developer/tests/e2e/reading_custom_suite_visual_check.py`，真实走 `custom` 流程，覆盖 selecting、P1/P2/P3 逐步选择、长标题、ready、confirm disabled/enabled、cancel 和四视口截图。
- U16 全量门禁通过：Vue typecheck/build、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。

## 2026-08-08 U17 Reading Library secondary navigation

- 将 `<=640px` 的五个 Library 视图入口从隐藏滚动条的横向条改为 3×2 网格；保留桌面和平板现有布局。
- 五个 `data-view`、`aria-current`、active class 和 `showView` 切换逻辑均未修改；手机按钮统一为 44px 触控目标。
- 新增 `developer/tests/e2e/reading_library_tabs_visual_check.py`，覆盖 1440/768/390/360/320px，并逐一切换 overview/browse/practice/more/settings 验证 active view 和语义。
- 截图人工抽查确认 390/320px 的“更多/数据工具”完整可见，无隐藏横向滚动或文本裁切。
- U17 全量门禁通过：Vue typecheck/build、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- U17 标记完成；下一窄切片冻结为 U18 Reading History widget/mobile controls 审计，总体视觉搬迁目标保持 active。

## 2026-08-08 U18 Reading History audit

- 恢复持久计划并确认 U15-U17 已完成、U18 尚未修改产品代码；工作树中的 Application/Agent/UI 累积改动全部保留。
- 读取 `ReadingHistoryPanel.vue`、`PracticeLibraryPage.vue` 对应 scoped CSS 和 `opensource-skin.css`，冻结 widget flip、三种选项、趋势范围、筛选、导出/批量/清除动作的现有合同。
- 已定位两个待实测缺口：绝对定位 back face 在 270px 卡片内的窄屏裁切风险，以及 History 三动作在通用 `nowrap` 规则下缺少手机端网格所有权。
- 下一步新增专用 Playwright mock，先让几何断言在当前实现上给出证据，再做最小 route-scoped skin 修复。
- 首轮脚本因 mock 忽略 `listHistoryAll` 的 offset 而反复返回首页，历史加载未完成；已按真实分页合同修正夹具，不修改产品代码。
- 第二轮确认动态导入的 Tauri core 直接走 `__TAURI_INTERNALS__.invoke`；夹具改为双入口共享 invoke，不再依赖异常 fallback。
- 第三轮 pageerror 定位到非空历史路径的真实缺失 import：`getRecordDate()` 使用 `safeDateMs` 但页面未导入；已补齐 import，继续运行定向回归。
- 非空历史加载后首次几何断言暴露测试夹具把四个趋势范围误写成三个；已按现有组件合同修正测试，不改产品行为。
- 390px 长标题修复后定位到记录行第二行 `.record-result` 零宽；移动规则已补 `width:100%` 和 action container flex 约束，继续验证趋势/翻面越界。
- 进一步确认零宽来自非批量时隐藏 selection 节点折叠第一 auto grid track；移动 CSS 现在按 bulk selection 是否可见明确跨列/双列布局。
- History header 已满足自然换行和 44px 高度；测试初版错误要求每个按钮 88px 宽，已收窄为防止文字/控件塌缩的 64px，不改产品 action 布局。
- U18 定向视觉回归四视口全绿；清理自定义卡片无处理器的 `role=button`/tabindex/aria-pressed，保留实际控件和翻面状态。
- 视觉截图抽查通过：`reading-history-widget-mobile-current.png` 与 `reading-history-widget-small-current.png` 中趋势、热力图、历史列表、固定导航和批量提示无互相覆盖；开始全量门禁。
- U18 全量门禁收口：`run_static_suite.py` 为 18/18，`suite_practice_flow.py` 的 packaged Tauri 流程全部通过，`cargo test --workspace --target-dir target/gate` 全部测试通过。
- U18 标记完成；总体视觉搬迁目标继续 active，下一窄切片冻结为 U19 Reading History record detail and batch-action continuity。

## 2026-08-08 U19 Reading History record continuity

- 审计确认 U19 不需要复制 opensource 的旧详情 modal runtime；当前 Vue 记录标题已经通过现有 review route 提供详情入口，切片只收口记录卡和批量交互。
- 修复批量模式 checkbox 的 click 冒泡：原先 row click 与 change handler 可能连续切换两次，新增 `@click.stop`，未改变 props/emits、Set 选择模型或 Tauri API。
- 在 `opensource-skin.css` 增加记录卡的 checkbox、score surface、delete/action 44px 几何和 muted metadata 视觉 owner，保留既有状态颜色、标题截断、移动单列和 action 自然换行。
- 新增 `reading_history_record_visual_check.py`，覆盖四视口非空记录、长标题、score/delete/action 尺寸、批量选择开关、review 路由连续性和 delete 不导航合同。
- U19 定向回归通过，开始执行 Vue、static suite、packaged practice flow 与 Rust workspace 全量门禁。
- U19 全量门禁收口：Vue typecheck/build 通过，static suite `18/18`，packaged Tauri practice flow 全部 checks 通过，`cargo test --workspace --target-dir target/gate` 全部通过。
- U19 标记完成；总体视觉搬迁目标继续 active，下一窄切片冻结为 U20 Global History batch-action mobile continuity。

## 2026-08-08 U20 Global History batch continuity

- 审计确认全局 `HistoryPage` 的 writing band/reading ratio 混合尺度、详情 overlay 和 essay list 数据流均应保持独立；本切片只处理批量操作栏的响应式几何。
- 修复 `batch-actions` 在窄屏使用 `grid-template-columns` 但实际仍为 flex 的无效规则；route-scoped skin 现在统一使用 `flex-wrap`，移动端计数独占一行、删除/取消动作平分可用宽度。
- 新增 `history_batch_visual_check.py`，覆盖 1440/1024/390/360px 非空历史、选择计数、两项 action 的 44px 目标、flex-wrap 和 document/body 宽度。
- U20 定向回归通过，开始执行 Vue、static suite、packaged practice flow 与 Rust workspace 全量门禁。
- U20 全量门禁收口：Vue typecheck/build 通过，static suite `18/18`，packaged Tauri practice flow 全部 checks 通过，`cargo test --workspace --target-dir target/gate` 全部通过。
- U20 标记完成；总体视觉搬迁目标继续 active，下一切片冻结为 U21 Global History filter and analytics mobile density。

## 2026-08-08 U21 Global History filter/analytics density

- 在 `HistoryPage.vue` 为 task/date/score/search/statistics range 增加稳定 label 关联，并为 comparison table 增加显式 scroll wrapper；未改 computed、watch、API 或 DTO。
- 在 `opensource-skin.css` 增加 History route-scoped shrink/overflow owner，移动控件统一 44px，analytics header/range selector 可在窄屏换行。
- 新增 `history_filter_analytics_visual_check.py`：mixed 1440/1024/390/360px 无 document/body 横向溢出，1024/390/360 的 comparison table 可横向滚动；writing/reading 分支保持现有 analytics 可见性。
- 人工抽查 `history-filter-analytics-mixed-desktop-current.png` 与 `history-filter-analytics-mixed-small-current.png`，确认无控件/图表/导航重叠，移动阅读顺序与 DOM 一致。
- 定向脚本首轮未展示 writing analytics，原因是 fixture 使用了页面 projection 之后的结构；改为后端真实 `latest.score + average` 输入形状后通过，产品代码未改。
- U21 门禁通过：Vue typecheck/build、History 定向回归、static suite `18/18`、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- U21 标记完成；总体视觉搬迁目标继续 active，U22 冻结为 Global History trend scale and analytics range presentation。

## 2026-08-08 U22 Global History trend scale

- 将 mixed History 的单一伪百分比趋势拆成 writing/reading 两个同量纲 series，仍先排序并截取最近 15 条；未改 list/statistics 查询或 DTO。
- `LineChart` 新增可选 tooltip prefix，写作显示 `Band 7.5`，阅读显示 `82%`；写作轴从截断的 4-9 修正为完整 0-9。
- 回归加入 mixed/write/read series、axis labels、tooltip units、Band 3.0 低分点、单点 area path、pageerror 和后端 command 集合断言。
- 首轮回归捕获模板残留 `trendData.length` pageerror，修正为 `trendSeries.length`；截图随后暴露并修复单点三角填充。
- 人工抽查最终 desktop 与 360px 全页截图：两种趋势独立、填充闭合正确、无卡片/表格/导航重叠。
- U22 门禁通过：Vue typecheck/build、定向 History 回归、static suite `18/18`、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`。
- U22 标记完成；总体视觉搬迁目标继续 active，U23 冻结为 Global History recent-record and pagination mobile continuity。

## 2026-08-08 U23 Global History recent-record and pagination

- U23 标记 in progress；长期目标继续 active，边界仍是只迁移 `opensource` 视觉/交互经验，不迁移旧 JS 架构。
- 完成当前 Vue 与 `opensource` 参考树的并行只读审计；确认参考树没有 pagination/task badge 合同，不能照搬其 hover-only 删除和四列移动布局。
- 实测复现最后一页删除后停留空页且分页消失的功能错误；同时冻结长 token 裁切、18px checkbox、34px 动作按钮和移动分页压缩为定向回归目标。
- session catch-up 首次因 Windows GBK 无法输出 Unicode 标记而失败；设置 `PYTHONUTF8=1` 后恢复成功。
- 新增 `developer/tests/e2e/history_record_pagination_visual_check.py`，以 21 条 mixed 记录覆盖四视口、长 token、reading/unlabeled badge、44px 控件、分页与末页删除回落。
- 首轮红灯复现 title `scrollWidth=2808/clientWidth=1023`、25x27 checkbox、34x44 icon actions、透明 badge；产品修复后几何转绿。
- 加严移动分页断言后捕获 grid 自动布局假绿；显式分配两行两列后定向回归通过，末页删除查询序列为 `0 -> 20 -> 20 -> 0`。
- `npm --prefix apps/writing-vue run typecheck` 通过；U23 四视口定向回归通过，等待截图人工核验和全量门禁。
- 人工核验 `history-record-pagination-{desktop,tablet,mobile,small}-current.png` 通过；四视口无记录卡、分页或导航重叠，进入全量门禁。
- Rust workspace 门禁第一次因 shell timeout 误设为 1 秒而被工具中止；未产生测试结论，改用 10 分钟上限重跑。
- `cargo test --workspace --target-dir target/gate` 全绿；Vue production build 通过。
- History 回归矩阵全绿：`history_detail_visual_check.py`、`history_batch_visual_check.py`、`history_filter_analytics_visual_check.py`、`history_record_pagination_visual_check.py`。
- static suite `18/18` 通过；packaged flow 首轮 launch/routes 通过，但截图时因 EdgeDriver 150 驱动 WebView2 151 而 session disconnected，进入工具链版本修复。
- 使用独立的官方 EdgeDriver `151.0.4129.59` 复验 packaged Tauri flow，全项通过；未修改 runner 或覆盖旧驱动。
- U23 全量 DoD 收口并标记 completed；U24 冻结为 Global History 页面级 loading/error/empty 状态连续性并进入 in progress，长期 goal 保持 active。
- U24 只读审计完成：确认 loading 裸文本、empty 被 skin 压到 `min-height:0`、四状态缺统一语义；参考树只提供单一 empty 文案层级。
- U24 已修改但尚未验收：`HistoryPage.vue` 将 loading/error/empty/filter-empty 统一到 `.history-list-state`，补齐 `status`/`alert`、live region 与 loading busy 语义；`opensource-skin.css` 建立统一状态 surface、稳定高度、error danger surface、44px 恢复动作和 reduced-motion spinner。请求 gate、重试、reset、DTO 与 commands 未改。
- U24 16 场景定向回归通过：1440/1024/390/360px 的 loading/error/empty/filter-empty 均无横向溢出；desktop 状态高度稳定为 180px、mobile 为 160px，重试/重置动作高 50px，恢复行为分别保持 retry/reset。
- 人工抽查 desktop loading/error 与 mobile empty/filter-empty 截图通过；状态 surface、筛选区、顶层导航和文案没有遮挡或错误层级。
- U24 编译与 History 回归矩阵通过：Vue typecheck/build、`history_detail_visual_check.py`、`history_batch_visual_check.py`、`history_filter_analytics_visual_check.py`、`history_record_pagination_visual_check.py`、`history_page_states_visual_check.py` 全绿。
- U24 全量门禁通过：Rust workspace tests、static suite `18/18`、packaged Tauri flow 和 `git diff --check` 全绿；U24 标记 completed，长期视觉搬迁目标继续，U25 进入只读审计与冻结。
- 用户要求停止 UI 搬迁并发布当前状态；U25 只完成了只读候选审计，没有产品代码或测试实现，现标记 cancelled。当前交付边界冻结在已完整验收的 U24。

## 2026-08-10 CI 门禁修复

- 用户要求修复 force push 后全部失败的三个 CI 门禁，并更新旧测试基线以覆盖当前 Application/Agent/Vue/Tauri 实现。
- 按 `github:gh-fix-ci` 读取 run `31369965578` 日志：唯一实际失败为 `browser-actions/setup-edgedriver` 仓库不存在；Rust 与 Tauri jobs 因 `needs` 链全部 skipped，没有产品测试结论。
- 按 `planning-with-files` 恢复持久上下文；首次 catch-up 因 GBK 无法输出 Unicode 标记失败，设置 `PYTHONIOENCODING=utf-8` 后成功。
- 三路只读审计完成：workflow 拓扑、后端/前端覆盖、packaged/16 个定向视觉脚本；结论已写入 `findings.md`。
- C1/C2 完成，C3 进入 in progress；下一步完整读取并修改 workflow、EdgeDriver 安装、视觉 runner 与 packaged driver 日志。
- 已完整读取将修改的 workflow、static gate、packaged runner，并核验 16 个视觉脚本的 endpoint/入口；冻结四文件族最小实现，不修改产品 Vue/Rust 业务代码。
- C3/C4 实施完成：`.github/workflows/tauri-ci.yml` 拆成独立 jobs，`release.yml` 去除失效 action；新增 `install_edge_webdriver.ps1` 和 `run_visual_regressions.py`；packaged gate 增加 Agent IPC 负向契约与 `tauri-driver.log`。
- 本地 static suite `18/18` 通过；`cargo test --workspace --locked` 全绿，实际覆盖 Application 24 项、Tauri host 21 项以及 DB migration/Agent audit/backup 测试。
- EdgeDriver 安装器本地通过：WebView2/driver 均为 `151.0.4129.72`，官方二进制签名有效。
- 统一视觉 runner 本地 16/16 通过，四组共生成 86 张本轮截图 evidence。
- `cargo tauri build --no-bundle` 通过；packaged Tauri E2E 全绿，新增 `agentIpcBoundary` 实际通过，driver 日志正常落盘。
- 独立实现复核发现干净 Rust job 缺 `dist/writing`、固定端口归属和活动 WebView2 识别缺口；已补前端 build、动态端口和 EdgeUpdate 注册解析，并删除 release 重复 build。动态端口首个补丁的 Python 函数边界错误被 compile 检查捕获，正在重验。
- 第二轮动态端口矩阵 15/16 通过，History state 测试暴露 CSS readiness 与 `159.999938px` 子像素假红；已保持 CSS 硬阈值并修正测试时序/rect 容差，History 16 状态单项复验通过。
- 动态端口 packaged Tauri gate 全绿，报告记录独立 WebDriver/native ports，新增 Agent IPC 边界继续通过。
- 最终动态端口视觉矩阵同一次运行 `16/16` 全绿；History page states 在 CSS readiness 与 0.5px rect 容差修复后通过。release bundle Windows runner 同步固定为 `windows-2022`，timeout evidence 做 bytes 兼容收口。
- 最终 `cargo test --workspace --locked` 复验全绿：Application 24 项、Tauri host 21 项及全部 DB/迁移/Agent audit/backup 集成测试通过；C5 完成，C6 进入提交与远端验证。
- 本轮 `rg` 因 Codex WindowsApps 打包路径拒绝访问而无法启动；已改用 PowerShell `Select-String` 完成同一只读定位，没有影响实现或验证结论。
- 远端 run `31376576871` 的视觉矩阵首先通过；static gate 的 17 项产品/Rust检查通过，唯一失败是 `check_reading_data_integrity.py` 在 Windows runner 的 cp1252 stdout 打印中文 JSON 时触发 `UnicodeEncodeError`。
- 将 UTF-8 固定在 `run_static_suite.py` 的统一子进程边界，避免只给单个数据脚本增加特殊情况；其余远端 jobs 保持运行以继续收集独立证据。
- 在显式 `PYTHONIOENCODING=cp1252`、`PYTHONUTF8=0` 的父环境下重跑 static suite，18/18 全绿；Reading source data integrity 成功输出含中文的完整 JSON，确认修复命中远端根因。
