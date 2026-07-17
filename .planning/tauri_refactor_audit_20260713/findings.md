# 审计发现

## 已知上下文
- 前一阶段声明已落地 suite auto-pick、evaluation policy、endless ownership。
- 用户要求先提交当前工作区，再按任务书并发审计。

## 证据
- 确认时 `HEAD=f306bdd`，Git 产品工作区无已跟踪或未跟踪业务变更。
- `.planning/` 是本轮审计新增且未纳入产品提交的工作记忆。
- 已创建空检查点提交 `0cea4a7 chore: checkpoint before Tauri refactor audit`，冻结审计起点。
- 任务书自报状态存在明显张力：进度板称 Phase 0–10 均未验收完成；后段大量任务勾选为实现完成。审计必须区分“代码存在、产品接线、运行验证、发布验收”四层，不能照抄勾选框。
- 任务书明确的终验缺口包括真实 AI、reading 全文资源打包、god-page/类型边界、packaged Tauri E2E、签名/更新、Windows/macOS 实机与视觉/性能证据。
- 审计边界可按任务书自然拆为三组并发证据：① Phase 0–4 + 10（领域契约、Tauri 壳、SQLite、历史/设置/备份、清理发布）；② Phase 5–7（写作、阅读、模式状态机）；③ Phase 8–9 + UX/测试（legacy 收尸、a11y、性能、前端产品接线）。
- 主代理最终需横向核验 Definition of Done、强制故障注入、任务书自报勾选与实际调用链是否一致。
- 任务书目标数据模型强调一个事实只存一处：`attempts`、`attempt_answers`、`writing_evaluations`、annotations、suite items、coach messages 与 namespaced settings；legacy 只能在冷导入 adapter 出现。
- 产品冻结合同覆盖全局导航、阅读各题型/计时/套题/无尽/背题、写作恢复与降级、统一历史、备份恢复、密钥和更新回滚。任何 Rust 模块“存在”但 Vue 未走该调用链，都不能算完成。
- Phase 2/10 的勾选框与备注直接矛盾：updater/签名/回滚被勾选，但 pubkey 为空且无实操；必须判为实现骨架存在、发布验收未完成。
- Phase 4 文案仍提到 Electron fallback，但 Phase 10 又声称产品树全删；需审计当前 repository 是否仍保留运行时双路径或仅开发 fallback。
- Phase 5/6/8/9 不能仅凭 Rust 单测标绿：真实 provider、packaged asset、全题型键盘等价、动态 legacy 资产、视觉 PNG 和设备 P95 都是独立验收面。

## 子代理交叉结论（待主代理核验）
- 平台核心：shipping runtime 已 Tauri-only；Phase 0–4/10 均未整体达到“接线且验证”。最危险的是 backup 只保存 attempt summary/settings/secret refs，恢复会丢 answers、annotations、evaluations、suite/coach 等用户事实。
- 领域契约：Rust DTO 存在，但 TS `generated/domain.ts` 自称手写；domain 当前缺少实际单测/生成漂移 gate。
- 安全发布：同一 main 窗口叠加全部 capability；updater inactive、空 pubkey，只 check 不 install；无真实签名/回滚证据。
- 写作：prompt 贯通，temperature namespace 错配；Compose 同步等待评测并在跳页前广播事件，evaluation DTO 又缺恢复所需 id，真实产品流存在卡死风险；orchestrator 未接产品。
- 阅读：前端对 `PracticeAssetV2Payload` 再包一层，页面按错误形状读；submit 把客户端 answer key 送回 Rust，判分真相不可信；draft marked/timeline 恢复不闭环。
- Phase 7：suite auto-pick 已进 Rust，但不筛可答题资产、不验证 custom P1/P2/P3；endless 池/random/preference fallback 仍在前端；memorize Rust command 未接 UI；suite/endless submit 非单事务。
- Phase 8：repository 存在，但 coach `query`/`question` 字段错配；Notes 仍写 settings KV；stable anchor revalidate/mismatch 未接 UI。
- Phase 9/QA：dragdrop 键盘替代不存在，旧测试只断言 CSS 字符串；god-page 仍约 5267 行；视觉截图过期，性能预算无设备 P95；现有 6/6 + 5/5 只覆盖静态构建和最小 packaged smoke。

## 当前 HEAD 门禁复跑
- `python developer/tests/ci/run_static_suite.py`：6/6 通过（2026-07-13T07:18:37Z）。覆盖 shipping layout、Tauri contract、Vue production build、cargo check、AI 配置静态检查、225 份 reading source data integrity。
- `python developer/tests/e2e/suite_practice_flow.py`：5/5 通过（2026-07-13T07:18:47Z）。覆盖 launch、Vue hash routes、reading IPC、bundled resources、SQLite settings restart。
- 两道门禁均未覆盖写作真实评测、阅读页面渲染/提交、套题中断恢复、无尽/背题、备份全量恢复、coach、键盘拖拽、签名/updater/rollback；通过不能推翻上述产品断链。

## 主代理抽查确认
- Backup DTO 确实仅含 `attempts/settings/secret_refs`；attempt summary loader 明确写入空 `answers/annotations`。这是确定性数据丢失，不是“缺测试”。
- `generated/domain.ts` 首行明确 `Generated-by-hand`；`crates/ielts-domain/src/*.rs` 当前 `#[test]` 数量为 0。
- updater 配置确认 `active:false`、空 endpoints/pubkey；五个 capabilities 均绑定 `main`，权限在同窗合并。
- Compose 确实在 `await evaluate.start()` 返回后才路由；Tauri command 同步 await provider；client 在返回后立刻 emit events；`WritingEvaluationV4` 无 id；事件/恢复断链成立。
- Settings client 统一读写 `app` namespace，而 eval resolver 固定读 `model`；temperature 配置错配成立。
- `PracticeAssetV2Payload` 是 `{asset,payload}`，practice client 将整个返回再放到页面 `payload`；读取形状错一层成立。
- `ReadingSubmitCommand` 接受完整客户端 payload，Rust 从其 `answerKey` 判分；服务端真相被客户端覆盖成立。
- suite auto-pick 只按 category/frequency，候选为空会静默退回 category-only；未见可答题过滤，显式 sequence 只校验长度。
- dragdrop 交互文件没有 keydown/keyup/tabindex 放置逻辑；旧测试只断言 `dragdrop-select` 字符串。a11y 假绿成立。

## Rust workspace 测试
- `cargo test --workspace` 失败于 `ielts-db` lib：15 项中 13 passed / 2 failed。
- 失败项：`settings::ai_config_tests::default_config_drives_the_single_runtime_settings`、`metadata_never_persists_secret_material`。
- 两项均在测试 `unwrap()` 时收到 `Validation("refusing to store API key / secret material in settings table")`；说明当前测试夹具/AI 配置键与密钥防泄漏校验发生契约漂移。任务书中的 cargo 测试绿灯已失效。
- 根因确认：`looks_like_secret_payload` 对 object 的任意包含 `secret` 的字段名直接拒绝，而 `AiConfigDto` 合法元数据自带 `hasSecret`；`upsert_ai_config` 因而连脱敏配置都无法保存。安全校验把“密钥值”和“是否存在密钥的布尔元数据”混为一谈。
- `npm --prefix apps/writing-vue run typecheck` 当前通过；但 static suite 没有执行该门禁。
- `ai_config_security` 集成测试也为 2 passed / 1 failed；失败项 `active_runtime_config_is_complete_and_contains_only_a_secret_name` 同样被 `hasSecret` heuristic 拒绝。该缺陷已由 unit 与 integration 双层确认。
- 排除已确认失败的 AI config 链后，其余 8 个 integration targets 全绿：import golden 5、property 2、phase3 5、phase4 4、phase5 8、phase6 5、phase7 4、phase8 3，共 36 passed。
- 这些 integration tests 证明 Rust repository/state machine 层骨架可运行，但不能证明 Vue 热路径已接线；多项产品断链正是测试绕过页面、客户端 DTO 和 Tauri 生命周期造成。
