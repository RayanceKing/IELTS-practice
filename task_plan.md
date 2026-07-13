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
  - 未完成：useReadingCoach 严格类型；PracticeReadingPage 高亮/提交/endless 继续拆；PracticeLibraryPage god-page
- [in_progress] 6. 测试门禁、CI、release 与 DoD 文档证据
- [completed] 7. 删除 legacy shipping tree 并验证功能 parity
- [pending] 8. 全量验证与交付摘要

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
