# IELTS Atlas Rust/Tauri 重构计划

## 目标
完成可验证的 Rust/Tauri 原生应用：淘汰旧 Electron/file:// 双路径，补齐真实 AI、资源打包、前端类型化与测试/DoD 证据。

## 阶段
- [completed] 1. 独立复核任务书、前次审计与当前代码基线
- [completed] 2. P0 基线：Vue build、阅读全文 command/resource seed、最小绿色门禁
- [completed] 3. 全量 AI Rust provider、Coach、配置管理、事件状态与安全闭环
- [completed] 4. 资源清单、Tauri bundle 与启动幂等 seed
- [in_progress] 5. 前端 Vue/TS 单一路径、god-page 拆分与剩余严格类型收敛
  - 已完成：阅读交互/拖拽 composable 抽取；attempt/highlights/timer/interactions 纳入 strict tsconfig
  - 已完成：设计系统地基 `styles/design-system`（Atlas HeroUI/Shui tokens + alias + base）
  - 已完成：紫色渐变品牌全量撤出；brand/primary 重定向 glass warm（shui peach/teal + bloom）
  - 已完成：useReadingCoach 严格类型；阅读 UI 偏好/背景主题/无尽模式 → Tauri settings；删除只写不读的 sessionStorage 答案/提交镜像
  - 未完成：PracticeReadingPage 高亮/提交/endless 继续拆；PracticeLibraryPage god-page；Library 本地 spacing 覆盖可再收敛
- [in_progress] 6. 测试门禁、CI、release 与 DoD 文档证据
- [completed] 7. 删除 legacy shipping tree 并验证功能 parity
- [pending] 8. 全量验证与交付摘要

## 2026-08-07 Application 层重构

- [completed] A1. 冻结写作评估、Coach、AI runtime 的现有行为与依赖边界
- [completed] A2. 新建 `ielts-application` crate 与最小端口/错误模型
- [completed] A3. 抽离 `src-tauri/src/ai`，实现 `LanguageModel` 适配器
- [completed] A4. 迁移写作评估编排，保持 command/DTO/事件序列不变
- [completed] A5. 迁移 Coach 编排，保持失败降级与分数不变量
- [completed] A6. 补齐 characterization/failure/锁释放测试
- [completed] A7. 依次运行 Rust、Vue、static suite、practice E2E 全量门禁

### 固定决策

- `ielts-application` 是垂直用例层，不承接普通 CRUD。
- 第一阶段允许依赖 `ielts-db` 的公开类型，但禁止依赖 Tauri、reqwest、Keyring 或直接 SQL。
- 首批只迁移写作评估与 Coach；不修改数据库 schema、Vue 页面、Tauri command 名称或 DTO。
- AI 通过最小 `LanguageModel` 端口接入；provider 配置、凭据和 HTTP 留在 Tauri 基础设施层。
- 网络 `await` 期间不得持有 SQLite mutex；现有评测幂等、事件恢复、degraded 结果和 Coach 分数保护均为兼容合同。

## 2026-08-07 Agent 后端第一阶段

- [completed] G1. 审计现有 AI/DB/Tauri 边界并冻结最小 Agent 数据流、安全边界和兼容合同
- [completed] G2. 在 `ielts-application` 建立 Agent 模型协议、工具端口、有限状态循环与 fake characterization tests
- [completed] G3. 在 `ielts-db` 增加 Agent run/tool-call 审计表、事务 API、备份覆盖与迁移测试
- [completed] G4. 在 Tauri 基础设施实现 OpenAI-compatible tool calling 和受限工作区文件工具
- [completed] G5. 增加短期工作区授权与 Agent Tauri commands，不修改现有 command/DTO
- [completed] G6. 增加路径逃逸、符号链接、尺寸、哈希冲突、循环上限和锁释放安全测试
- [completed] G7. 执行 Rust、Vue、static suite 与 packaged E2E 全量门禁并独立复核

### Agent 第一阶段固定决策

- Agent 编排属于 `ielts-application`；HTTP/Keyring、工作区授权和文件系统工具实现属于 `src-tauri`；持久化审计属于 `ielts-db`。
- `LanguageModel::complete` 保持不变，新增独立 `AgentModel` 协议，避免破坏写作评估与 Coach。
- 首期只提供 UTF-8 `read_file`、受哈希保护的 `write_file`/`replace_in_file`；不提供 shell、进程、任意网络、目录删除或绝对路径工具。
- 每次运行必须持有由原生目录选择器签发的短期 workspace grant；模型参数不能直接指定宿主绝对路径。
- 文件路径必须是 workspace 内相对路径，并拒绝父目录、符号链接逃逸、敏感控制路径和超限内容；已有文件写入必须携带读取所得 SHA-256。
- 工具调用顺序执行、次数和轮数有硬上限；未知工具或参数错误作为结构化 tool result 返回模型并写入审计。
- SQLite 方法保持短锁；模型和文件 I/O `await` 期间不得持锁。
- 本阶段只交付后端 command 与合同，不同步重做 Vue Agent UI。

## 2026-08-07 本地后端层级收敛

- [completed] H1. 审计 command/application/db/AI-Agent 之间的重复转换、错误映射与无收益抽象
- [completed] H2. 选择一个窄切片，删除重复职责并用 characterization tests 冻结行为
- [completed] H3. 复核普通 CRUD 的直连边界，避免 application 演变为通用 repository 层
- [completed] H4. 收敛本地 Agent 基础设施，只保留 API Key 隔离、路径 containment 和文件并发保护等真实边界
- [completed] H5. 执行 Rust、Vue、static suite 与 packaged E2E 门禁
- [in_progress] H6. 基于实测结果决定下一窄切片，循环推进直至层级职责稳定

### 层级收敛固定决策

- 不新增 crate、DI 容器、事件总线、权限引擎、通用 repository 或 Agent 状态机框架，除非出现可复现的循环依赖或行为缺陷。
- `ielts-application` 只承接跨 SQLite、模型、文件或异步状态的垂直用例；普通 CRUD 继续允许 Tauri command 直连 `ielts-db`。
- 单用户本地应用继续使用单 SQLite connection mutex；优化目标是缩短临界区，不是引入连接池或分布式事务。
- 安全边界限定为不泄露 API Key、不允许 workspace 路径逃逸、修改文件保留 SHA-256 并发保护；不扩展 RBAC、审批流、独立沙箱进程或远程审计系统。
- 每个切片只改一个明确责任边界，保持 command 名称、Vue DTO、数据库 schema 与用户流程不变，并在进入下一切片前通过现有门禁。
- 下一审计切片已冻结 Agent audit 写入失败语义：工具/成功终态的 finish 失败直接透传并停止；失败路径收尾 best-effort 保留原错误；复用 `running -> interrupted` 恢复，不新增 uncertain 状态。

## 2026-08-08 Vue 视觉搬迁路线

### 目标

将 `F:\workspace\IELTS Atlas`（opensource）作为视觉/交互参考源，把稳定的页面层级、色彩、间距、排版、组件状态和导航连续性逐步搬到当前 `apps/writing-vue`。只搬视觉与交互经验，不搬 opensource 的 JS 数据流、存储、运行时或资源加载方式。

### 约束

- 保留 Vue、Tauri command/DTO、SQLite 事实源和现有 API/composable；UI 改造不得反向改变后端合同。
- 不在同一切片重写数据层、路由协议、Prompt、Agent 或数据库 schema。
- 页面 section 使用现有设计 token；不再引入第二套颜色/间距 token。
- 每个切片必须有 desktop/mobile 截图核验，并通过 `npm --prefix apps/writing-vue run typecheck`、`npm --prefix apps/writing-vue run build`，随后运行仓库静态/E2E 门禁。

### 阶段

- [completed] U1. 盘点 opensource 与当前 Vue 的视觉基线，冻结页面/组件映射
- [completed] U2. 搬迁全局壳层：导航、背景、内容宽度、标题/按钮/状态 token
- [completed] U3. 搬迁 Library/History/Settings 的列表、筛选和空/加载/错误状态
  - 已完成：Reading Overview、Reading Browse、Reading History 视觉切片及桌面/390px WebView 核验
  - 已完成：Settings AI 总览卡、Settings detail modal、全局 History surface/overlay 几何
  - 下一切片：Writing 页面布局与评测状态展示
- [completed] U4. 搬迁 Writing 页面布局与评测状态展示，保持现有 Tauri 流程
  - 已完成：Compose 双栏工作区、题目配置分段控件、提示面板、编辑器 header/footer、字数状态和确认层的 opensource skin 视觉收口
  - 已完成：Result 评分/反馈侧栏 surface、评分环/指标层级、错误职责拆分和移动端单列布局
  - 已完成：Evaluating 状态 rail、进度/日志 surface、AI orb token 化和移动端单列布局
  - 已完成：390px/320px 专用移动视口核验，三页均无页面级横向溢出
- [completed] U5. 搬迁 Reading 页面布局、题目导航、标注和 Coach 面板，保持现有 composable
  - 已完成：Coach FAB/浮层的导航避让、模态层级、触控滚动、暗色对比和移动端触控目标视觉收口
  - 已完成：Coach 切片通过 Vue typecheck/build、static suite 18/18 与 packaged Tauri E2E；组件 DOM、文案、事件和 API 未改
  - 已完成：1440x900、1280x720、390x844、360x640 Coach 几何核验；无导航重叠、层级错误或横向溢出
  - 已完成：标注 `#selbar` 恢复 opensource 深色 action strip，精确颜色/按钮合同与全量门禁通过
  - 已完成：题目导航在 980px 以下改为两行 grid、640px 以下改为三列两行操作区；无内部纵向滚动或按钮裁切
  - 已完成：桌面 packaged 截图与 820/390/360px 定向几何核验，保留扁平 `questionOrder` 和全部 DOM/事件合同
- [completed] U6. 搬迁 Agent 入口的视觉占位和工作区交互，不扩展 Agent 后端合同
  - 已完成：新增 `/agent` Vue route 与顶层导航入口，移动端首屏顺序为总览/阅读/写作/Agent
  - 已完成：三栏桌面、两栏平板、单栏手机工作区，覆盖文件、提示词、上下文和运行状态本地交互
  - 已完成：页面不调用 `agent_pick_workspace`、`agent_run` 或 `agent_get_run`，未改 Rust command/DTO/SQLite
  - 已完成：1440/980/390/360px 截图、几何与本地交互检查；Vue、static 18/18、packaged Tauri E2E 全绿
- [completed] U7. 全量视觉回归与门禁，清理仅由旧视觉迁移产生的重复 CSS
  - 已完成：清理 History/Result 两处同作用域重复规则，以及 Compose/Result/Reading/Evaluating 五条被 canonical skin 完全覆盖的旧声明
  - 已完成：静态 shell 合同增加 `/agent` route/nav 检查；packaged route visual 增加 Agent 页面截图与几何检查
  - 已完成：PostCSS 同作用域重复选择器检查为 0；四视口 Agent 检查与 packaged route visuals 无 offenders
  - 已完成：`cargo test --workspace`、Vue typecheck/build、static suite 18/18、packaged Tauri E2E 全部通过

- [completed] U8. 题库管理工作区视觉连续性
  - 仅改 `TopicManagePage` 的 route-scoped skin 与弹层/响应式几何
  - 保留现有题库 CRUD、筛选、分页、上传和删除事件合同
  - 覆盖 loading、empty、error、card、pagination、editor/import/delete modal 状态
  - 已完成：上传区补齐键盘可达的 `role=button`/`tabindex`/Enter/Space 触发与 focus 状态
  - 已完成：`1440/980/390/360px` 视口几何检查、弹层 fixed/bounded、文件选择器键盘触发
  - 已完成：Vue typecheck/build、static suite 18/18、packaged Tauri E2E、Rust workspace tests

- [completed] U9. Settings 二级弹窗视觉边界
  - 只补 Settings route-scoped fixed overlay、bounded dialog、内部滚动与层级
  - 保留 onboarding/update/confirm 的现有 Vue 状态、按钮和 Tauri 调用
  - 不在本切片重做 Settings detail 内部 panel 层级
  - 已通过 1440/1024/390/360px 弹窗几何与全量门禁

- [completed] U10. Settings detail surface 层级
  - 只降低 detail modal 内部重复 raised panel 的阴影与背景重量
  - 保留 detail modal、tab、form、danger/about 内容的 DOM、状态和交互
  - 不改变 Settings overview 卡片、弹窗层级或数据流
  - 已通过五个 detail tabs、危险区和关于页的 1440/1024/390/360px 几何与全量门禁

- [completed] U11. History detail modal 状态层级
  - 只审计/搬迁 History 详情中的 loading、error、feedback、score 与空态 surface
  - 保留历史查询、评分数据、删除确认和 DTO/API 合同
  - 继续只采用 opensource 视觉经验，不迁移旧 JS 或 DOM 协议
  - 已完成：外层 dialog 成为唯一 raised surface，总分/反馈/正文/分析/元数据恢复清晰层级
  - 已完成：补齐 API 空结果的只读空态，保留 loading/error/重试/关闭原交互
  - 已完成：success/loading/error/empty 在 1440/1024/390/360px 均无溢出并通过全量门禁

- [completed] U12. 窄屏主导航完整可见
  - 只调整 `NavBar.vue` 的窄屏排布和当前页语义，不改路由数组、active 判定或页面数据流
  - `<=640px` 将六个固定入口改为 3×2 网格，取消不可发现的隐藏横向滚动
  - 覆盖 1440/768/488/390/360/320px、四个代表路由和键盘 focus 几何
  - 已完成：六项均可见、44px 触控目标、无横向页面溢出，`aria-current` 按 href 合同正确
  - 已通过：Vue typecheck/build、导航六视口回归、static suite 18/18、packaged Tauri practice flow、Rust workspace tests

- [completed] U13. Reading Suite session surface 层级
  - 只审计套题摘要、passage row、active/submitted/locked 状态和 loading/error 空态
  - 保留 `practiceReadingSuite` API、session 路由、继续/复盘事件和 frameless shell
  - 继续只采用 opensource 视觉层级，不迁移旧 JS 或存储协议
  - 已完成：summary/passages 保持 raised workspace；active/submitted/pending 行降为状态控制面；修复 `pending` DTO 状态的皮肤选择器；补齐 loading/error/empty 语义
  - 已完成：新增 `developer/tests/e2e/reading_suite_visual_check.py`，覆盖 1440/1024/390/360px、成功/加载/错误/空态、frameless、路由和无溢出
  - 已通过：Vue typecheck/build、专用套题视觉回归、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`

- [completed] U14. Reading Library tools/settings surface 层级
  - 只审计阅读练习库的 More Tools、Settings/config panel、clock/PDF overlay 的 surface 归属和窄屏几何
  - 保留 `PracticeLibraryPage` 的现有 composable、repository、筛选/导出/时钟/PDF 交互合同
  - 继续只采用 opensource 视觉经验，不迁移旧 JS、存储或运行时协议
  - 已完成：More Tools 卡片降为 control surface，featured tool 保留 selected surface；Settings 子 panel、system info、config list 降低重复 raised 层级
  - 已完成：clock/PDF overlay 增加 bounded mobile geometry，并修复 overlay 打开时 app-main stacking context 被导航拦截的问题
  - 已完成：新增 `developer/tests/e2e/reading_library_surface_visual_check.py`，覆盖 More Tools、Settings、config、clock、PDF 的 1440/1024/390/360px 状态和交互
  - 已通过：Vue typecheck/build、专用 Library surface 回归、static suite 18/18、packaged Tauri practice flow、`cargo test --workspace --target-dir target/gate`

- [completed] U15. Reading SuiteSelector modal chrome
  - 只补套题模式选择弹层的 header/close/body/option 排版和窄屏边界
  - 保留 `ReadingSuiteSelector` props、emits、父层创建套题流程和现有频率选择合同
  - 继续只采用 opensource 视觉经验，不迁移旧 JS、存储或运行时协议

- [completed] U16. Custom Suite selection bar
  - 只收口自选套题进度条、P1/P2/P3 chip、ready/disabled 状态和窄屏动作几何
  - 保留 `customSuiteDraft`、逐篇选择、确认/取消 emits 和套题创建数据流
  - 用长题目标题和四视口回归冻结无横向溢出合同

- [completed] U17. Reading Library secondary navigation visibility
  - 只调整五个 Library 视图入口在窄屏的完整可见性和触控几何
  - 保留 `libraryViews`、`data-view`、`aria-current`、query route 和 `showView` 数据流
  - 覆盖 1440/768/390/360/320px，禁止隐藏横向滚动和页面溢出

- [completed] U18. Reading History widget and mobile controls audit
  - 先冻结趋势卡、自定义 widget 翻面、筛选/批量动作和窄屏几何的真实合同
  - 只在发现可复现视觉缺口后增加 route-scoped owner，不改统计/历史数据流
  - 保留 `practiceWidgetSelectorOpen`、widget emits、history filters 和记录操作合同

- [completed] U19. Reading History record detail and batch-action continuity
  - 以 `opensource` 的历史记录列表层级为参考，审计记录卡、分数结果、删除/批量操作和空/错误状态
  - 只迁移视觉层级、密度和窄屏交互几何，不改 history DTO、repository 或 Tauri commands
  - 先冻结真实状态合同，再用四视口定向回归验证，不把全局 HistoryPage 与 Reading History 混成同一 DOM

- [completed] U20. Global History batch-action mobile continuity
  - 只审计全局 `HistoryPage` 的筛选、批量操作、详情 overlay 和窄屏可达性
  - 保留 writing band 与 reading ratio 的现有数据边界，不复用 Reading History 的记录卡 DOM
  - 若发现真实 CSS contract 缺口，只增加 route-scoped owner 和定向四视口回归

- [completed] U21. Global History filter and analytics mobile density
  - 审计任务类型、日期、分数范围和趋势/雷达区在 1024/390/360px 的信息密度与可达性
  - 保持混合历史不能比较 reading ratio 与 writing band 的现有语义，不改统计查询或图表数据
  - 已完成：筛选控件补齐稳定 id/label/aria 关联；comparison table 增加独立横向滚动壳
  - 已完成：History analytics/filter 统一 `min-width: 0`，390/360px 控件达到 44px，范围选择器允许换行
  - 已完成：新增 `developer/tests/e2e/history_filter_analytics_visual_check.py`，覆盖 mixed 四视口及 writing/reading 语义切换
  - 已通过：Vue typecheck/build、定向 History 四视口、static suite 18/18、packaged Tauri practice flow、Rust workspace tests

- [completed] U22. Global History trend scale and analytics range presentation
  - 保留最近 15 条排序窗口，在 Vue presentation 层按 `scoreScale` 分成 writing Band 与 reading accuracy 两个独立 series
  - 写作使用完整 0-9 轴和 `Band` tooltip，阅读使用 0-100% 轴；mixed 不再伪造统一百分比
  - 修复 `LineChart` 单点 area path 的三角闭合，并用 path/pageerror/axis/tooltip 回归冻结
  - 已通过：Vue typecheck/build、定向 History 四视口、static suite 18/18、packaged Tauri practice flow、Rust workspace tests

- [completed] U23. Global History recent-record and pagination mobile continuity
  - 审计全局 History 最近记录的 task badge、时间/时长、score pod、查看/删除和分页在 1024/390/360px 的层级与可达性
  - 保持 Reading/Writing 记录 DTO、批量选择、详情 overlay 和删除 commands 不变，不复用 Reading History 的记录 DOM
  - 只修复可复现的裁切、动作塌缩、分页不可达或状态层级问题，并补四视口回归
  - 已完成：末页删除统一回落到有效页；长 token 可换行；checkbox/action 为 44px 且具有可访问名称
  - 已完成：reading/unlabeled badge 获得视觉状态；score pod 使用 DTO label；duration 优先使用实际值
  - 已通过：Vue typecheck/build、四项 History 回归、static suite 18/18、packaged Tauri flow、Rust workspace tests

- [completed] U24. Global History page-state continuity
  - 审计页面级 loading、error、无历史、筛选无结果在 1440/1024/390/360px 的层级、动作和布局稳定性
  - 保持 `list_history`/statistics 请求门控、筛选重置、错误重试和空态文案合同，不新增后端或状态管理
  - 只修复可复现的 nested card、状态塌缩、动作不可达或 loading/error/empty 几何漂移，并补四视口回归
  - 已完成：四状态统一 `.history-list-state` owner；补齐 live/busy/alert 语义、稳定高度、error surface、44px 恢复动作与 reduced-motion spinner
  - 已通过：16 场景四视口回归、人工截图核验、Vue typecheck/build、五组 History 回归、static suite 18/18、packaged Tauri flow、Rust workspace tests

- [cancelled] U25. 下一窄视觉切片审计与冻结（按用户要求停止 UI 搬迁，未实施产品改动）
  - 盘点尚未形成四视口视觉合同的 shipping Vue 页面与交互状态，以可复现缺口排序
  - 对照 `F:\workspace\IELTS Atlas` 的视觉证据，只迁移适用于当前 Vue/Tauri 产品的信息层级和交互经验
  - 冻结一个最小切片后先写 characterization regression，再决定是否修改产品代码

## 错误记录
| 错误 | 尝试 | 处理 |
|---|---:|---|
| 任务书在原 cwd 不存在 | 1 | 定位到同级 `IELTS Atlas APP` 目标仓库 |
| 计划文件误建在旧工作树 | 1 | 删除误建文件并改建到目标仓库 |
| 对不存在的 exec cell 调用 wait | 1 | 停止该调用，仅对实际运行中的 cell 使用 wait |
| session-catchup 在 Windows GBK 输出 Unicode 失败 | 1 | 设置 `PYTHONIOENCODING=utf-8` 后成功恢复 |
| `cargo test --workspace` 链接测试 exe 时 LNK1104 文件占用 | 1 | 单独 Tauri 测试随后通过；待释放占用后串行复验 workspace |
| release 应用启动退出 101 | 1 | 删除重复日志插件及残留 capability，packaged app 启动恢复 |
| packaged E2E runner 立即检查 Vue/参数形状错误 | 1 | 增加挂载轮询并按真实 command DTO 解包，E2E 5/5 通过 |
| 三个 AI 子代理服务返回 502 | 1 | 无代码产出；重新拆分任务后重派 |
| AI 配置安全测试初版误用通用 upsert_setting | 1 | 改为专用 ai_* command 测试，静态门禁 6/6 通过 |
| Application 计划补丁上下文少一个空格 | 1 | 改用精确小锚点分别追加，未产生文件修改 |
| Application 重构前 `cargo test --workspace` 基线失败 | 1 | `phase7_modes.rs` 4 处 `SubmitEndlessCommand` 缺 `timer_snapshot`；记录为既有门禁缺陷，最终以最小测试补丁修复 |
| Agent 阶段 session-catchup 在 Windows GBK 输出 Unicode 失败 | 1 | 设置 `PYTHONIOENCODING=utf-8` 后恢复成功 |
| 本轮两阶段 AI vault 改造导致静态检查仍寻找旧 command 函数名 | 1 | 更新 `check_ai_config_security.py` 识别 `src-tauri/src/ai/config.rs` 的 refs reconciliation；static suite 恢复 18/18 |
| `rg.exe` 在当前 Windows App 沙箱启动被拒绝 | 1 | 改用 PowerShell `Select-String`/`Get-ChildItem` 完成只读检索 |
| 递归检索 opensource 工作树超过命令时限 | 1 | 改为按已知 CSS/HTML 文件与有限上下文读取，不扩大扫描范围 |
| 将目录直接传给 `Get-Content` 导致只读检索失败 | 1 | 改用 `Get-ChildItem -File` 枚举后再执行定向检索 |
| U12 nav visual check treated rendered Agent label as a stable route identifier | 1 | Assert active link by href/route contract; keep text only for clipping checks |
| U12 nav visual check passed a positional argument to Playwright `wait_for_function` | 1 | Use the Python API's named `arg=` parameter |
| PowerShell exact cache cleanup was blocked by command policy | 1 | Removed only generated `developer/tests/ci/__pycache__` files via an explicit Python `Path` target |
| U13 误读不存在的 `practice-repository.js` | 1 | 根据 `practice-client.js` 的实际 import 定位到 `modes-repository.js` |
| Writing 移动截图首次使用 `file://` 导致 Vue 未挂载 | 1 | 改为同进程临时 HTTP server，390/320px 三页均成功挂载并核验 |
| Playwright `add_init_script` 误传第三个参数 | 1 | 改为 JSON 内嵌初始化脚本，Result 完整评分数据视图核验通过 |
| `rg.exe` access denied | 1 | 改用 PowerShell `Select-String` 完成同一只读检索，未修改产品文件 |
| 专用 Coach 几何脚本只加载入口 CSS | 1 | 改为显式加载 `PracticeReadingPage` CSS chunk 后重跑；未修改产品或测试源码 |
| 专用 Coach 几何脚本等待样式表总数超时 | 1 | 改为等待指定 CSS link 的 `sheet` 可用，不依赖样式表数量；未修改产品或测试源码 |
| Playwright 一次性 Node 命令被 Windows 命令策略拦截 | 1 | 改为 `developer/tests/e2e/agent_workspace_visual_check.py`，复用已安装 Python Playwright |
| PowerShell `Remove-Item` 清理测试缓存被命令策略拦截 | 1 | 确认目标仅为本轮生成的 `developer/tests/ci/__pycache__/*.pyc` 后，用精确 Python unlink/rmdir 清理 |
| U13 视觉脚本初版把 active 行的轻量选中阴影误判为 raised row | 1 | 断言改为只要求 submitted/pending 被动行无 shadow，保留 active selected surface 合同 |
| U13 视觉脚本把 `/library` alias 当作稳定 URL 等待 | 1 | 按现有 router redirect 合同等待 `#/`，不改产品路由 |
| U14 视觉脚本首次关闭 PDF 时被导航 stacking context 拦截 | 1 | overlay 打开时提升 `.app-main` stacking context；重跑四视口通过 |
| U17 后清理精确 E2E `__pycache__` 被 PowerShell policy 拒绝 | 1 | 校验绝对目标后用 Python `Path.unlink/rmdir` 精确删除，仅移除本轮生成缓存 |
| 恢复 U18 时误对不存在的 `noop` exec cell 调用 wait | 1 | 停止该调用；后续只等待真实长任务或子代理状态 |
| U18 history mock 首次忽略分页 offset | 1 | 按 command args 的 offset/limit 对样例数组切片，第二页正确返回空集合 |
| U18 history mock 仅设置 core fallback，真实模块直接走 internals invoke | 2 | 改为共享 `invokeMock`，同时注入 `__TAURI_INTERNALS__.invoke` 与 `__TAURI__.core.invoke` |
| U18 fixture 第三次仍无记录，等待 selector 只给出同一表象 | 3 | 停止猜测；改为等待 loading 结束并输出 invoke 日志、error/empty DOM 诊断 |
| U18 diagnostic wait inherited 30s timeout | 4 | 将 fixture diagnostic wait 限定为 5s，失败时直接输出调用链 |
| U18 invoke 日志仅到首次 list_history，loading 无错误态 | 5 | 下一诊断加入 pageerror/console 捕获，定位响应链的未处理异常 |
| U18 非空 history fixture 暴露 `safeDateMs is not defined` | 6 | 从现有 `historyStats` 模块补齐 PracticeLibraryPage 的缺失 import，保持 getRecordDate 逻辑不变 |
| U18 回归脚本把四个趋势范围误写成三个 | 7 | 按 PracticeLibraryPage 真实 `practiceTrendRanges` 合同修正为四个 |
| U18 mobile geometry 首次失败只报告 overflow 布尔值 | 8 | 错误信息加入 viewport/document/body 三个宽度，便于定位越界 owner |
| U18 overflow owner 尚未定位 | 9 | 几何采样增加 practice-library descendants 的 right-edge offender 清单 |
| U18 mobile delete action remained at x=410 after title constraint | 10 | 采样 history list/item/result/action ancestor rects，定位剩余 grid owner |
| U18 mobile record-result grid item had zero width | 11 | Give second-row result `width:100%` and keep action container shrinkable/flex-aligned |
| U18 zero-width owner persisted after mobile width rule | 12 | Capture computed display/width/min-width/grid/flex values for result and action nodes |
| U18 zero-width owner was caused by hidden selection collapsing the first auto grid track | 14 | Use `:has()` to make non-bulk content/result span both columns and preserve bulk two-column semantics |
| U18 mobile header action geometry assertion omitted actual button sizes | 15 | Include action rects in the failure so only the deficient owner is changed |
| U18 mobile control test demanded 88px width although the contract only requires 44px height | 16 | Keep 44px height and use a 64px minimum width to catch collapse without inventing a grid |
| U18 custom card declared `role=button` without a click/keyboard handler | 17 | Remove the fake interactive semantics; retain actual icon/widget controls and `practiceWidgetSelectorOpen` state |
| U18 initial regression only checked card bounds, not chart-flow ownership or front scrollHeight | 18 | Add assertions for absolute canvases, trend face scrollHeight, equal card/rotor/face heights, and <=360px header bounds |
| U18 geometry error omitted the newly captured computed styles | 13 | Include result/action computed styles in diagnostic output |
| U21 首次 mixed fixture 未展示 writing analytics | 1 | 按 `loadStatistics` 真实 legacy projection 补齐 `latest.score`、`latest.taskType` 与 average 结构；产品代码未改 |
| U22 首轮趋势语义回归捕获 Vue pageerror | 1 | 模板残留 `trendData.length`；改为新的 `trendSeries.length` 并把 pageerror 固化为硬失败 |
| U22 split-series 截图出现单点三角填充 | 1 | 修正 `LineChart.areaD` 单点闭合边界，并加入 path regression |
| U23 Rust workspace gate 被 1 秒 shell timeout 中止 | 1 | 改用 10 分钟明确上限重新执行，未将超时误判为测试失败 |
| U23 packaged flow 在 route screenshot 阶段丢失 WebDriver session | 1 | 定位为本地 EdgeDriver 150 与 WebView2 151 版本漂移；使用独立的官方 151 driver 复验，不修改产品代码 |
| U24 误在 `functions.exec` 内调用 collaboration spawn | 1 | collaboration 工具不能嵌套；改为直接 `spawn_agent` 调用 |
| 发布前 `git ls-remote` 遭遇 Schannel TLS handshake failure | 1 | 不重复同一探测；先完成本地原子提交，再通过 GitHub CLI/推送路径复验远端连接 |
| CI 修复阶段 session-catchup 在 Windows GBK 输出 Unicode 失败 | 1 | 设置 `PYTHONIOENCODING=utf-8` 后成功恢复 |
| `rg.exe` 从 Codex WindowsApps 路径启动被拒绝 | 1 | 改用 PowerShell `Get-ChildItem`/`Select-String` 继续只读检索 |
| 驱动安装器测试命令包含递归临时目录清理，被命令策略拒绝 | 1 | 不重试清理；改用 GUID 唯一临时目录执行验证 |
| packaged runner 动态端口补丁把 helper 插入 `try/except` 中间 | 1 | Python compile 立即捕获；恢复完整 `except` 后再定义 helper |
| EdgeUpdate client 枚举在 StrictMode 下遇到无 `name` 的无关 key | 1 | 先检查 `name`/`pv` registry values 存在，再筛选 WebView2 registration |
| 动态端口视觉复跑中 History tablet/empty 在 CSS 生效前读取几何 | 1 | 保持 180px 断言不变，先等待共享 state owner 的 computed `display:grid` |
| History mobile/filtered rect 为 `159.999938px`，严格 `<160` 假红 | 1 | CSS min-height 保持硬阈值；仅对浏览器 rect 使用 0.5px 子像素容差 |
| History 单项 PowerShell server 包装被 `Stop-Process -Force` 策略拦截 | 1 | 改用标准 `importlib` 限定统一 runner 的脚本集合，由 runner 自行清理进程 |
| 首次 `runpy` 单项注入未改变函数 globals，意外复跑全矩阵 | 1 | 不重复；改用 `importlib.util.module_from_spec` 后单项 16 状态全部通过 |
| 视觉 runner timeout 输出在部分 Python 版本可能为 bytes | 1 | 增加统一 UTF-8 `output_text` 转换，保证失败日志写入不二次报错 |
| GitHub Windows static gate 的 Python 子进程继承 cp1252 | 1 | 在 `run_static_suite.py` 的统一 subprocess 边界固定 `PYTHONUTF8=1` 与 `PYTHONIOENCODING=utf-8` |
| 首次 `gh run watch` 遭遇 GitHub API `unexpected EOF` | 1 | 不重复实时 watch，改用 `gh run view --json jobs` 快照和 job logs API |
| Linux bundle verifier 把 RPM staging 的零字节 `empty` 文件当发布产物 | 1 | 区分完整证据清单与可发布产物；只对 installer/updater 执行非空门禁，并增加正反 characterization tests |
| packaged gate 在本地依赖旧 bundle 留下的 `target/release` 资源目录 | 1 | 每次按 Tauri resource map 把 exe 与资源放进全新临时 runtime，再启动 WebDriver；消除残留假绿 |

## 2026-08-10 CI 门禁重构

### 目标

修复 force push 后 `tauri-ci` 的三个失败/跳过门禁，并让 CI 按当前 Vue/Tauri/Application/Agent 架构提供互相独立、可诊断的证据。UI 搬迁保持停止，不修改产品行为。

### 阶段

- [completed] C1. 读取远端 Actions 日志，确认失败发生在 checkout 前的失效 EdgeDriver action 解析
- [completed] C2. 并行审计 workflow DAG、Rust/Vue/Agent/DB 覆盖和 packaged/视觉脚本运行条件
- [completed] C3. 拆分 static、Rust workspace、packaged Tauri、视觉回归和跨平台 bundle jobs
- [completed] C4. 增加仓库自有 WebView2/EdgeDriver 精确版本安装器与统一视觉回归 runner
- [completed] C5. 本地执行 workflow 相关门禁并修复真实失败
- [in_progress] C6. 提交、推送并等待远端全部门禁给出最终结果

### 固定决策

- `rust-test` 必须执行 `cargo test --workspace --locked`，纳入 `ielts-application`、Agent 文件工具、AI runtime 和 Tauri adapter 测试。
- static、Rust、packaged Tauri 与跨平台 bundle 不再串成单点失败链；环境故障不能把无关证据全部标成 skipped。
- EdgeDriver 必须匹配 WebView2 Runtime 的精确版本，只从微软官方地址下载；不再依赖不存在或漂移的第三方 setup action。
- 不恢复随机 CDP 端口、90 秒等待、三次重试或吞掉原生日志的旧补丁。
- U1-U24 定向视觉脚本由一个 runner 启动一次 preview server 后串行执行；不为每组重复安装浏览器、构建前端或启动服务。
- CI 不调用真实 AI provider，不新增 crate、通用测试框架或产品层抽象。
