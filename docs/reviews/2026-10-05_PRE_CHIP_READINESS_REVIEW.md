# Chips 实现前最终准备度审查 — 2026-10-05

## 1. 结论

**当前不放行批量 chips 实现（NO-GO）。** 允许继续基础协议修复、契约整理、独立测试设计和 Host 工具准备；现有 fold 仅能作为有限演示，尚不宜复制为全项目模板。

不是因为“完整 C 编译器尚未实现”，而是因为现有基础机制仍违反已声明的任务推进、await-all 和快照完整性约束。标准检查全部通过，但本轮仓库外 **12 个异常观察测试全部成立**，另有一个静态扫描漏检复现。测试通过在这里表示“问题可复现”，**不是正确性通过**。

| 放行对象 | 本轮判断 | 原因 |
|---|---|---|
| 根框架现有 CPU／restricted 路径 | 可继续使用；不要求本轮重构 | 既有测试通过，领域边界保留；开放 trait／adapter 的可信边界仍存在 |
| Wave 1 单 fold slice | 演示已运行；正式模板放行待修复 | 快照、计算体扫描、输入引用和 route 校验未闭环 |
| Wave 2 M1 frontend | NO-GO | 通用任务协议缺陷 + 尚未冻结的真实上游记录／交接接口 |
| Wave 3 全量语言／优化 | NO-GO | AST/type/IR 全量接口、改写失效规则等未冻结 |
| Wave 4 target／Part B | NO-GO | Linux AArch64 实测报告与 attestation 缺失；不能用 macOS 值代替 |
| Wave 5 corpus／最终 torture | NO-GO | corpus、实例分母、运行环境和 runner 未完成 |

本报告是审查记录，**不是 ADR、契约批准、owner 签字、版本冻结或实施授权**。不修改旧报告的历史结论，也不根据“新代码存在”推定所有设计已获批准。

## 2. 基线、范围与证据

- Git HEAD：`53e34d3dc796fe1b53c26ebdeb488a47ba92e91e`。
- 审查的是**当前工作树**，包括用户已有的 14 个未提交文件修改，不只是 HEAD。
- 当前版本文件／代码：`t01-c01-c06/8`，hash：
  `9216c594cdb1a3265f8497c810a1e050aaf9674d77c9ea4b7d181aed4feed855`。
- 环境：macOS 开发机，`rustc 1.99.0 (b940084d7 2026-09-28)`；不是 Linux AArch64 验收环境。
- 仓库不是 Cargo workspace；根包及三个嵌套包分别检查。
- 本轮只新增本报告及[复现源文件](2026-10-05_PRE_CHIP_READINESS_REPRO.rs)，未改实现、公共契约、任务包、旧报告；没有 commit／push。

用户已有修改：`compiler/contracts/CONTRACT_VERSION`；
`compiler/src/{chips/fold,chips/mod,commit,contract,limits,routing,task}.rs`；
`compiler/tests/{c08_gate1,freeze}.rs`；
`docs/tasks/{CHIP_PLAN,GATE_1_M1_FIRST_SLICE,T01_COMPILER_CONTRACT,WAVE_DISPATCH}.md`。

### 阅读与边界

交叉检查根 README、架构／SFL／设计蓝图、ADR-0001/0002、T00–T13、并行交接／任务模板、guardrails、Gate 1、CHIP_PLAN/WAVE_DISPATCH、M1 proposal/CDR/验收计划，以及原有 12 份审查。实现重点覆盖 framework、arena/IDs/intern、config/target/limits、bus/task/commit、manifest/routing、snapshot/contract、Worker/FoldChip、相应 tests 和 CI／torture／probe 工具。

这不是对未实现 C 语义、所有未来优化或所有 hostile 输入的穷举证明。没有真实候选编译、目标程序执行、GCC torture、联网取 corpus、实际 Linux probe 或 mutation/fuzz 全量运行。

**必须分开三层事实：** `/8` artifact 自洽；规则是否被实现满足；真实语言流水线是否完成。hash 重算一致不能替代后两层。

## 3. 当前确实已经改善的部分

以下 `/8` 修复有源码和 `c08_gate1` 测试依据，不沿用 `/7` 或旧 `/5` 缺陷结论：

1. `ConstantRequest` 已按 kind 严格区分 literal／binary shape，fold 接受转发的两种形状。
2. typed append 与 task/result/diagnostic/request/patch 使用统一总记录预算 preflight。
3. `AppendRecords` 已检查 producer 注册、accepted kind、declared write 和 schema field。
4. `Complete` 的 `Record`、`Records` 两种 carrier 都检查未来 Literal/Const 引用预测。
5. fold 实际使用 `max_const_bits`，不足预算产生 `ConstOverflow`，不伪造合法值。
6. fold 计算体现在只接收 `FoldInput`；全 bus 只在 projector/Worker adapter 边界。
7. `clock_tick_with` 已把 worker 收集接入 reset → dispatch → commit → latch/advance；现有 G1 fixture 不再需要手工改成 `Running`。
8. `/6` 已有新 family/tag/store/seed、wire inventory、Progress 和 bounded recovery 等基础；不能继续笼统说它们完全不存在。

仍然不是完整 `RestrictedChip` 安装链，也不是 source→T07→T08→T09 的真实语言闭环。`c08_gate1.rs:15–18,110–125` 明确用 seeded literal 和 reserved node；这能证明有限 fold／commit 集成，不能证明 lexer、parser、语义或 IR producer 已完成。

## 4. 实现与模板发现

优先级：**P1** = 放行前必须解决的高风险协议／模板／派工问题；**P2** = 中风险一致性或防御缺口，应在相关 slice 派工前解决。修复建议不构成修改公共契约的授权。

| ID | 优先级 | 问题 | 主要阻塞范围 |
|---|---|---|---|
| PCR-01 | P1 | config hash／snapshot 漏编码四个新增 limits | 所有 replay、证据归因和 fold 模板 |
| PCR-02 | P1 | 非空但无 transition 的 proposal 被当作任务执行成功 | 所有 Worker 派工 |
| PCR-03 | P1 | recovery／嵌套失败无法在 idle 时排空 join | 有子任务的 chips |
| PCR-04 | P1 | await-all 实际提前因一个失败 child 终止 parent | 有多子任务的 chips |
| PCR-05 | P2 | OwnBatch 子任务未验证 parent 关联 | Enqueue + AwaitChildren |
| PCR-06 | P2 | draft handle index 未验证，位置身份与 apply 脱节 | append／后续 relocation |
| PCR-07 | P2 | 同字段 AppendRecords + StorePatch append 未拒绝 | append protocol |
| PCR-08 | P2 | fold 接受不存在的 Binary NodeId | G1 输入契约与后续真实上游 |
| PCR-09 | P2 | stage/layer 校验只有独立 helper，执行入口未强制 | 所有 routed chips |
| PCR-10 | P1 | Worker 模板未满足既定 restricted DoD，lint 不扫描其计算体 | 模板复制／批量生成 |
| PCR-11 | P1 | 当前版本、权限和 wave 指南互相矛盾 | 所有任务派发 |
| PCR-12 | P2 | CI 没有扫描真实 chips 树；torture crate 未纳入 | 合并门禁与后续验收 |
| PCR-13 | P2 | quota-1 调度／join 反复全表扫描，扩展后有二次复杂度风险 | 大规模任务／后续 wave |

### PCR-01 — 新增配置不进入 runtime 快照

**证据：** `compiler/src/snapshot.rs:88–100` 只写旧 11 个 limits，遗漏
`max_inflight_per_tick`、`stage_queue_bound`、`max_const_bits`、`max_task_progress`。
相反，`compiler/src/contract.rs:368–386` 的 FrozenSchema 已写这些字段。

复现分别改四个字段，`config_hash` 与 `Snapshot::capture` 都相同。进一步构造两个相同 fold 快照，只把 bit budget 从 128 改成 1；下一 tick 一个提交 Const(5)，另一个失败且没有 Const。**不是执行随机，而是快照丢失了决定执行的输入。** 固定 contract hash 也无法标识每个 job 的配置。

**负责人／修复：** T01/C05。补齐 canonical config 编码，明确兼容／版本处理；对每个 limits 字段和每个 stage 数组元素做敏感性测试，保留“相同 snapshot 但下一 tick 分叉”的回归反例。不要仅测 FrozenSchema hash 变化。

### PCR-02 — 覆盖 proposal 不等于推进任务

**证据：** `routing.rs:449–470` 的 `covered` 只检查有任意 tagged proposal；`commit.rs:620–864` 检查重复 transition，却没有检查每个 dispatched task 至少一个 transition；成功路径 `routing.rs:473–480` 清空 `in_flight`。

一个 handler 只返回合法非空 `AppendRecords`：tick=`Executed`，Const 已提交，但 task 在 latch 后仍 `Running`，ready/in_flight 均空，下一 tick=`Idle`。这直接违反 `task.rs:1028–1034` 和 hashed rule `commit.no-running-at-latch`。StorePatch-only／Enqueue-only 同类路径也应纳入负例，不能只防空 vector。

**负责人／修复：** T01/T02。用明确的 transition 集合统计每个 dispatched task，成功前验证恰好一个；没有 transition 的非空批次在任何 semantic apply 前按冻结规则拒绝／转为 typed failure，并校验 latch 的双条件：零 dispatched Running **且**空 in_flight。不能先 append 再发现任务无归宿。

### PCR-03 — 失败后的 join 不能自主推进

**证据：** `routing.rs:333–338` 在无 ready 时直接 Idle；`routing.rs:540–553` recovery 只 `try_fail_task`，不推进 join；`commit.rs:895–948,1127–1166` 的 join 单次扫描没有对新产生的 parent failure 做闭包传播。

两个独立复现：

- Waiting parent 的 child 返回重复 Complete，commit 拒绝后 child 已 Failed，但 parent 下个 idle tick 仍 Waiting。
- 三层 parent 链，叶子走普通 CONTROL_UNSUPPORTED；中层 Failed，最外层永远 Waiting。

如果之后碰巧有别的 commit，join 可能再次被扫描；正确性不能依赖无关任务出现。显式等待已 terminal 的 child 不是合法的永久等待。

**负责人／修复：** T01/T02。冻结并实现 bounded deterministic join-only 推进／失败闭包，覆盖 normal Fail、commit recovery、Progress-limit failure、诊断 NONE sentinel、多层链和 idle。与成功 reinsert 的容量 preflight、报告归属一起设计，不要在 infallible apply 后加入未预检的 fallible queue 写入。

### PCR-04 — await-all 被实现成部分 fail-fast

**证据：** `commit.rs:917–920,942–945` 遇本 batch 失败 child 就 deferred，已有失败 child 即 join_failed；apply 看到诊断就失败 parent，没有要求其它 children 已 terminal。

复现 parent 等待两个 child：第一个本 tick Failed，第二个仍 Ready（ready_tick=100），parent 已 Failed。与已记录的选择 `M1_PART_A_CONTRACT_PROPOSAL.md §24.16 OB-4`（非 terminal siblings 等待，不取消）及 `contract.rs:95` 的 `join.await-all-terminal-state-only` 不符。

**负责人／修复：** T01/T02。所有 children terminal 才判定 parent；保持确定的失败诊断选择和不在 join 消费结果。覆盖 Failed+Ready、Failed+WaitingHost、Failed+Progress、全部 terminal 混合、不同 child 顺序。这里要求的是正确 await-all，不是加入 sibling cancellation。

### PCR-05 — OwnBatch 与 Committed 的 child 身份规则不一致

**证据：** `commit.rs:673–677` 把 task 发出的所有 Enqueue 都放入 own list，不检查 draft.parent；`resolve_child_ref` (`1471–1489`) 对 OwnBatch 只看 index，对 Committed 则要求 `record.parent == Some(parent)`。

复现 `Enqueue { parent: None }` + `AwaitChildren [OwnBatch(0)]` 被接受，持久 WaitSet 指向一个不是该 parent 子任务的 task。API “本任务产生的请求”和“parent tree 的 child”两个概念脱节。

**负责人／修复：** T01/T02。在预测表保存 parent 并在 preflight 校验；若确实允许 await 独立任务，应正式定义这一例外并统一两种 ChildRef 的契约，而不是让 wire 表达形式决定身份规则。

### PCR-06 — draft index 没有绑定实际位置

**证据：** `records.rs:21–31` 定义 index 是 own append batch 中的位置；`commit.rs:779–805` zip 仅核对 family，`1045–1066` 只迭代 body。

`[42]`、`[0,0]`、`[u32::MAX]` 三种 handle index 都能 append 成功。当前 fold 总用 `[0]`，所以正常 fixture 看不到问题；后续 OwnBatch relocation／link 将无法安全依赖这些身份。

**负责人／修复：** T01/C01/C03。在冻结的 1:1 positional 限制下验证 `handle.index == position`，拒绝重复／越界／非规范值；或删去不使用的 index 并版本化明确位置身份。Gate 1 不需要实现所有 family 的 relocation，但它已有的 handle 不能宣称被验证而实际忽略。

### PCR-07 — 同一字段两种 append 机制可同时提交

**证据：** `task.rs:1035–1036` 明确禁止 AppendRecords 与 StorePatch append 指向相同 `(store, field)`；`commit_proposals` 各自授权，没有交叉校验。

同一 fold task 对 `constants.records` 同时 typed append 和 StorePatch append，并 Complete，被接受：`appended.len=1`、`patches=1`。现阶段 patch 仅记录 intent，**不是已经重复 materialize 两个 Const**；问题是协议已经接受未来会歧义的双机制，并产生不一致的 patch/version 记录。

**负责人／修复：** T01/C03。preflight 建立 task/field 机制集合，冲突拒绝且无 mutation；测试同字段冲突和不同字段合法共存，不能靠删除原测试中的一条 proposal 避开。

### PCR-08 — Binary node 只做 family/arity 解码，没有 liveness 检查

**证据：** `fold.rs:63–88` projection 只读取 literal，不查询 Node；`fold.rs:192–215` Binary 分支忽略 node。`GATE_1_M1_FIRST_SLICE.md:61` 要求 committed NodeId。

`NodeId::from_index(123)` 在空 nodes arena 中不存在，payload 的两个 Literal 均存在，fold 仍 Complete 并生成 5。

**负责人／修复：** T01/T08。由 adapter 投影 node 的 live/present 事实，或冻结并实际调用输入引用校验。G1 的 Node body 仍是 reserved，**不要求此时检查完整 AST Add 语义**；仅检查已承诺的存在性就能堵住当前反例。

### PCR-09 — stage/layer 校验没有接入安装／执行链

**证据：** `manifest.rs:792–809` helper 能报 StageLayerMismatch，但 `ManifestRegistry::register` 不接收 routing；`RoutingTable::register` (`routing.rs:95–105`) 不验证 stage；`chips/mod.rs:193–209,239–264` driver 忽略 layer。

把 const_fold route 从 frozen stage 2 改成 layer 9，注册与 tick 正常成功，FoldChip Complete。helper 单测通过不证明真实入口执行了校验。

**负责人／修复：** T01/T02。提供 integrator-owned 最终 topology 校验入口并使 driver 开始执行前必经；保留错误 layer／错误 owner／缺路由的端到端负例。无需将所有未来 kind 强行塞入 G1，也无需先实现全局 DAG elaborator。

### PCR-10 — 不能把当前 Worker 模板当作 restricted 路径达标

**证据：** `chips/mod.rs:46–60,75–82` Worker 接收全 read-only bus、registry 不执行 ZST 检查；FoldChip 有窄 `compute`，但只是 inherent method，并没有实现 `RestrictedChip`。它的 manifest 只声明 `lex.literals`，projection 还读取 task header、const arena count 和 config bit budget（`fold.rs:59–88,98–112`）。这些额外机械读被注释标出，却未在 field manifest／正式 adapter 例外表完整绑定。

`tools/chip-lint/src/lib.rs:155–163,377–382` 只识别 `impl RestrictedChip`，不会访问 Worker／inherent compute 的 body。一个只有 Worker impl + inherent compute 的 AST fixture，compute 含 `std::env::var`，**strict lint 返回成功**。这只是扫描测试，没有执行环境读取；fixture 不需 type-check，工具的边界本就是 AST。

扫描真实 chips 树却失败两次：`fold.rs:29` 的纯 BTreeMap import 和 `mod.rs:37` 的内部 re-export。根因是 `tools/chip-lint/src/lib.rs:171–182` 的 import-root 规则；**不是发现了真实 Host I/O**。仅改为 `alloc`／`self` 让扫描绿也不能修复 compute 漏检。

与 `docs/tasks/README.md:105–109`、`T13:89–115`、accepted guardrails 的新 compiler chip DoD 尚未对齐。

**负责人／修复：** T01 + framework/lint owner。优先沿既定 `RestrictedChip` 路径绑定 narrow input/output、ZST、adapter；若保留专用 Worker execution，则先获批一份明确的等价约束契约，补 ZST、读取边界和语义体 lint。增加真实模板 positive、Host call／hidden state／跨 chip／full-bus compute negative，审核 helper 调用闭包。root framework 保持 domain-free，不能把 C 规则塞进通用 primitive。

### PCR-11 — 派工入口不能唯一确定“当前”

**当前冲突：**

- 根 README 多处是 `/7`，`396–403` 仍说 Worker compute 收全 bus、shell 不驱动 workers，与 `/8` 代码不符。
- `docs/tasks/README.md:25,54–56` 和 `PARALLEL_EXECUTION.md:5–17` 仍称 `/5` current、`/6` unfrozen、Gate 1 proposed，并沿用不同 wave 列表。
- `T01 §7.1` 的 `/8` addendum 正确，但同节 `/5` 历史表仍无足够醒目的历史隔离，C01/C05 还写机制 absent／hash scope 未协调。
- `COMPILER_SFL_MANIFEST.md:117–130` 仍写只有 StorePatch 查 producer manifest，遗漏 `/8` AppendRecords。
- Gate 1 §5/§6/§7 是历史 `/7` 状态，§8 才是 `/8`；CHIP_PLAN 仍写 Wave 0 `/7`、Wave 1 等 manifest/routing，实际 FoldChip 已在工作树。
- 文档存在 2026-10-06 的执行／授权记录；本轮日期是 2026-10-05。保留记录，不反推它们无效，但提交前需核对时间／时区／授权来源；未来日期不是本轮已经执行的证据。

另外 `WAVE_DISPATCH.md:51–55` 的“stragglers 不阻塞 green wave”只能适用于无必需依赖的交付。缺一个 parser／verifier 的必需 producer 时，suite 绿不构成闭环放行。R1 自动版本 bump 也不等价于批准任意语义／架构变化。

**负责人／修复：** T01/docs integrator。一份当前状态／supersession 表链接实际 artifact、accepted decision、接口范围、可派工 chip、阻塞项；旧内容标历史而不是抹掉。统一 wave 含义，建立逐 chip 的 work order 和依赖闭包门禁。先解决已选方向在正文与附录冲突，再冻结下一 slice；不必重新询问已明确选择的方向。

### PCR-12 — 本地验证强于真实 CI 门禁

`.github/workflows/ci.yml:24–49` 只 fmt/clippy/test root、chip-lint、compiler；**不跑真实 chips 树扫描、不检查 work-order 覆盖、不跑 tools/torture crate**。
`t00-target-probes.yml:46–74` 的 self-tests 仅在匹配路径变更时自动跑，实际 probe 为 workflow_dispatch-only。仓库 workflow 配置也不能证明 GitHub branch protection 已把 job 设为 required。

**负责人／修复：** CI/lint/T00 owner。在 PCR-10 对齐后纳入真实 chips scan；加入 torture crate 三项检查和 contract/work-order consistency。真实 probe 继续可独立 gated，但 Part B 放行必须引用已核实的报告；不要要求每个 target-independent PR 都执行 ABI probe。本轮未查询远端 required checks。

### PCR-13 — 当前扫描算法只适合显式有界的 bring-up

**静态依据：** `routing.rs:214–233` 每 tick 扫描并排序整个 ready 集合，即使 quota=1；`commit.rs:895–905` 每个 commit 扫描所有已分配 tasks 寻找 Waiting parents，已完成记录也留在 arena。预建 N 个互不依赖的 noop task 后顺序完成，每 tick 的 join 扫描仍为 N 条，累计至少 Θ(N²) 次 task 检查；ready 集合重复排序还增加成本。这不是测得的性能数字，但算法结构足以说明不能从小 fixture 推导可扩展性。

**负责人／处理：** T01/T02。在小 G1 profile 中可明确记录有界接受；进入大规模波次前建立任务规模／ticks／allocations 的独立 benchmark，选择 bus 中显式 waiting 索引／dependency worklist 和稳定 ready 顺序，或给现有扫描设可审计的小上界与接受理由。不要引入隐藏跨 tick cache，也不要为速度牺牲 reference order。accepted guardrails §4 要求算法风险先被识别和界定；本轮没有声称完成性能基准或吞吐验收。

## 5. 明确的有限范围与未实现项（不误报为本轮新 bug）

### 5.1 Sole-writer 仍是 wave-gated，不是全局隔离

复现 `ChipId(99)` 申领 CONTROL_START_JOB，declare `constants.records`，可以注册并 append Const。`manifest.rs:692–705` **明确允许非 slice kind 跳过 allowlist**，hashed rule 也写了 wave-gated。因此这是已声明的范围／扩展风险，**不是 `/8` 违反“全局已启用”承诺的新漏洞**。

下一 wave 必须冻结新增 kind 与 field 的 allowlist；若要独占已有 const 字段，应加入跨 kind／group 的负例，明确现有 slice owner 与新行的关系。禁止对 outside-wave manifest 自动放行后宣称全局 sole-writer 已建立。

### 5.2 quota>1、stage queues 与统计尚不具备验收证据

`Limits::try_new` 可以接受 quota>1；`routing.rs:331–372` 实际执行 batch，但 `outcome_for_batch`／`record_tick` (`505–525,642–654`) 只投影 first selected。`commit.rs:972–985` 用旧 `max_queue_len` 代替 canonical per-stage queues，`stage_queue_bound` 没有实际逐 stage 执行；`tasks.ready` 仍是运行队列，不是已实现的派生视图。

Gate 1 明确非目标 quota>1。本轮不以“缺少完整 pipeline”否定 G1，但建议 integrator 的接受 profile **显式固定 quota=1**，不得靠 default 防止误开启，更不得用当前 first-task report 发布吞吐／批次完整性结论。非默认 stage bounds 的 inert 状态需在当前能力表清楚记录，不能泛称所有 bound 均已执行。

### 5.3 结果、Host 和语言记录的下一 wave 阻塞

- join 不消费结果的方向已选；`consume_result` 机制已存在。但 parent 自己 atomic commit 的 consume+publish/retry envelope、consumer fencing、`join_then_replay_retry` 仍需真正绑定与验证。
- scope startup 已有 early TU-root／File-Enter 的方向，不是完全未设计；TU carrier、edge enqueue、root-range invariant、启动 kind/stage 尚需冻结／集成。
- `SemRecord`／TypeKind reuse scan／conversion minimal domain 已有 proposal §24.17–24.18 的 user-accepted recommendation；不能继续称“没有选择”。其记录 body、canonical lookup、VF06 注册和真实 producer 仍未交付。
- PP01 artifact／raw map、PP04 token-range、token↔literal↔PP provenance／reservation、NamePlan materialization 仍需下一 slice 的可执行 typed append／field manifest／错误表。
- IR `FunctionEnd` 的 whole-batch rejection 方向已有选择；当前 TerminatorMissing 仅有 error shape，不是已安装 hook。符号 Part A interpreter、真实 IR lowering／verifier 和 CLI 也尚无完整链。
- H04 candidate CLI、H05 subprocess isolation、Host request satisfaction／wake、CT14 finalization 都不能由 library fold fixture 冒充完成。

## 6. 原有 12 份审查闭环状态

“方向已记录”不等于“已实现”， “旧措辞已更正”不等于新增 chip 通过验收。下表是本轮范围内的复核，不宣称关闭旧报告每个低优先级子项。

| 旧报告 | 当前状态与仍需处理的内容 |
|---|---|
| [DOCUMENTATION_REVIEW](2026-10-05_DOCUMENTATION_REVIEW.md) | DOC-01–18 的处理记录总体保留：Part A 不再要求 -S、NEG identifier 判断、macro paste 顺序、literal protection、heap/purity/provenance 限定等已更正。DOC-01 现在需补 `/8` append；OPEN-03 的 G1 request shape 已冻结，但真实 T07→T09 链仍缺；OPEN-01/02 已有方向，机制／验收未闭环 |
| [FRAMEWORK_CORE_SOURCE_AUDIT](2026-10-05_FRAMEWORK_CORE_SOURCE_AUDIT.md) | unsafe 禁令／reference tick／restricted projection 仍有测试支持。legacy/open backend/adapter 边界仍有效；macro name mismatch 的防御缺口仍在，但 install_projected 有 ZST 检查，不把它误判为当前支持安装链已经失守；Worker 是另一个待对齐入口（PCR-10） |
| [NUMERIC_INVENTORY_AUDIT](2026-10-05_NUMERIC_INVENTORY_AUDIT.md) | 不能再用旧 24/20/20 counts 推导当前缺失；新 refs/family/store/wires/seed 和 consistency tests 已落地。全量未来错误 numeric 表／语言 enum 不因此自动冻结；FrozenSchema 已编码新 limits 但 runtime config 漏编码（PCR-01） |
| [ERROR_INVENTORY_AUDIT](2026-10-05_ERROR_INVENTORY_AUDIT.md) | 新 Progress/backpressure/join/predicted errors 有 numeric mapping；ConstOverflow 已使用。EffectMask carrier 在 §24.16 OB-27 已选择 chip Fail，但旧 proposal/T13 仍需同步；dispatcher carrier 临时借 CommitError 已记录，不称已交付独立 scheduling family；部分旧错误仍 coarse Protocol(1) |
| [NAME_INTERNING_AUDIT](2026-10-05_NAME_INTERNING_AUDIT.md) | 基础 InternTable 稳定 dedup 仍有测试；Names store 与 reservation helpers 已存在。TokenRecord committed/draft typing、keyword/name fixture count、真实 NamePlan relocation/materialization 仍需下一 lex slice 对齐；不要求当前 fold 实现 lexer |
| [TYPE_CONVERSION_AUDIT](2026-10-05_TYPE_CONVERSION_AUDIT.md) | TC-01 的 reuse predicate（structural equality、all committed scan、lowest ID）、TC-03 identity-only、TC-05 non-M1 diagnostic 在 §24.17/18 已有方向；不是重新待选。正文／T06/T07/T13 的不一致、identity recorded-vs-absent、VF06 domain/registration 仍需清理并冻结；没有 Type/Sem 实现正确性证据 |
| [PROPOSAL_S9_KIND_STAGE_AUDIT](2026-10-05_PROPOSAL_S9_KIND_STAGE_AUDIT.md) | G1 三种 kind 和 control stage 已赋值；全 M1 的 kind→stage 仍未实现。Host TaskKind vs HostRequestKind 分层方向已选，映射/chain 尚缺。OB-47 verification chip-list 文档对齐不等于 VF06 已注册；PCR-09 新增真实执行入口缺口 |
| [M1_RECORD_LINK_AUDIT](2026-10-05_M1_RECORD_LINK_AUDIT.md) | Literal/Const future Complete refs 已校验，DraftRecords 新 carrier 方向被撤回，不能继续以它缺验证判当前 bug。typed Node/Scope/Function/Block 的 coherence/liveness 仍是未来 slice obligations；当前已有 Binary node liveness 缺口见 PCR-08 |
| [M1_SOLE_WRITER_FIELD_TABLE](2026-10-05_M1_SOLE_WRITER_FIELD_TABLE.md) | allowlist/error 机制与 fold 一行已落地，不再说完全不存在。其它 field rows、Host/integration writer 边界、消费 fencing 仍未完成；非 slice bypass 是明确 wave-gated residual，不伪称全局防越权 |
| [M1_NEG_UNS_FIXTURE_AUDIT](2026-10-05_M1_NEG_UNS_FIXTURE_AUDIT.md) | 旧 owner／diagnostic-only inventory 有文档处理与 OB-47 等记录；CONTROL_UNSUPPORTED 基础路径已运行。大多数语言 NEG/UNS 仍没有真实 chips、typed code/有限恢复 evidence；不能以 foundation unsupported PASS 充当语言 fixture PASS |
| [M1_REC_WS_FIXTURE_AUDIT](2026-10-05_M1_REC_WS_FIXTURE_AUDIT.md) | replay、T+1、patch negatives 已可测；现在又有 tick fold／append permissions。RW-01/RW-05 的独立 semantic projection 和跨版本比较域仍未完整交付；RW-02/03 的诊断语义／独立 oracle 未靠 full-bus→narrow 手工 projection 自动解决。PCR-02/03/04 是目前真实协议反例 |
| [CI_GATING_AUDIT](2026-10-05_CI_GATING_AUDIT.md) | 原 CI coverage 缺口仍成立：torture crate、真实 chips scan、真实 probe 未成为自动合并门禁。本轮额外在本地跑 torture 检查与 probe self-tests，不代表 workflow 已变更 |

旧报告／proposal 保留大量 `/5` 或“not frozen”历史文字；复核必须跟随其附录的 accepted recommendation 和当前 T01/G1 supersession，不能只 grep 一个 open row 就重新提出同一决策。

## 7. 放行顺序与开工清单

### A. 先修基础，仍由 integrator 串行持有

1. **PCR-01 + PCR-02：** 完整 snapshot config + 每任务恰好一个 outcome/latch invariant；把反例改为永久回归测试（修复后应断言反例不成立）。
2. **PCR-03 + PCR-04 + PCR-05：** join idle／nested／recovery／await-all／same-batch parent，一起冻结顺序和 capacity 规则。
3. **PCR-06 + PCR-07 + PCR-08 + PCR-09：** G1 handle／coexistence／live node／实际 topology gates。
4. **PCR-10：** 正式确定可复制的 chip/adapter execution path、read manifest 例外、ZST 和 lint；先让唯一模板的 positive/negative gates 都成立。
5. **PCR-11 + PCR-12：** 当前状态入口、supersession、work orders 和 CI 门禁同步；按 R1 规则处理确有 protocol/encoding 变化的版本与兼容，不把 hash bump 当语义审批。
6. **PCR-13：** 大规模派工前明确 reference scheduler 的规模上界／benchmark 与后续索引计划；小 slice 的接受不能自动推广到十万 task。

### B. Wave 1 模板放行条件

- [ ] 标准四包检查通过；真实模板静态扫描通过，恶意变体扫描失败。
- [ ] PCR-01/02/08/09 和适用于 G1 append 的 PCR-06/07 已修复并有负例。
- [ ] 计算体没有 bus；adapter 的全部实际读已绑定或有正式机械例外；ZST 被入口执行校验。
- [ ] quota=1 profile 明确；G1 literal/binary、无效 kind、missing refs、unsupported、预算边界、replay、写域均覆盖。
- [ ] 当前版本/hash／派工文件／manifest/route/test 路径一致，演示状态与 M1 acceptance 状态分开。

### C. Wave 2 按 slice，而不是一次冻结 120 个猜测接口

PCR-03/04/05 和 consume/retry 协议放行后，建议冻结顺序：

| Slice | 冻结／负责方 | 首个必须可执行的验收 |
|---|---|---|
| Source/PP | T01/T02/T03：Source response、artifact map、PP request/range、typed append、所有 read/write rows | 从 frozen raw bytes 产生真实 PP records；LF/CRLF/EOF/map boundaries |
| Lex/name/literal | T01/T03/T04/T08：token/provenance、NamePlan、reciprocal token/literal、kind/stage/allowlist | 不预造 C token 的 spelling→literal→publish；与真实 PP 输出集成 |
| Parse/scope/type | T01/T05/T06：TU early commit、File-Enter、ParseContext/continuation、scope/type identity | M1-START-01：无预建 scope；声明可见性、child fail/retry |
| Semantic/constant | T01/T06/T07/T08/T13：SemRecord、identity/no-conversion rule、VF06 | real checked input→constant request→fold；非 M1 effects/conversions typed failure |
| IR/verification/Host | T01/T02/T09/T13/H04：IR bodies/links、FunctionEnd、interpreter、terminal/artifact evidence | 真实 upstream 的 M1-CL-05 + Part A snapshot/trace/interpret；不生成 target asm |

每个 work order 必有：唯一 chip ID/目录、当前 version/hash、允许修改文件、typed input/output、exact fields、kind/guard/stage/layer、error/progress/wait rules、全部测试类别、真实 integration prerequisite、owner、未支持列表。未满足依赖闭包的 straggler 不能被 green suite 掩盖；Unsupported 仅 shell delivery，不是 semantic completion/PASS。

### D. 独立推进但不得冒充完成的 Host 工作

- H00/H02/H03：锁 GCC revision、assets/licenses、官方 driver 生成实例与 options/分母，区分三个 suite。
- H01/H07/T01：provision Linux AArch64、固定工具链、真实 probe、自检事实／unresolved ABI 分类、hash/attestation；HVA、x18、TLS/reloc 等无覆盖项单列。
- H04/H05：candidate 自己编译；文件／subprocess 都在 Host；路径、输出、超时、资源、取消、崩溃隔离。
- H06/H09：每 suite instance/file rate 及 pooled rate；每 configuration 单独报告；缺失／unsupported／wrong code 留分母，基础 fixture 数量不能代替 pass rate。

Linux probe 与 corpus 不阻塞符号 Part A 契约准备；它们分别阻塞 target-dependent 实现验收和最终 torture 声明。外部 assembler/linker/libc/libm/libgcc 等依赖以后必须实名记录。

## 8. 本轮实际验证

所有以下标准命令 exit=0：

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --manifest-path tools/chip-lint/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/chip-lint/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/chip-lint/Cargo.toml
cargo fmt --manifest-path compiler/Cargo.toml -- --check
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path compiler/Cargo.toml
cargo fmt --manifest-path tools/torture/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/torture/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/torture/Cargo.toml
bash tools/torture/probe/tests/run-tests.sh
git diff --check
```

| 项目 | 本轮结果 |
|---|---|
| root | 15 integration + 6 doctests 通过 |
| chip-lint | 8 tests 通过 |
| compiler | c01=7、c02=11、c03=46、c04=15、c05=21、c06=15、c07=18、c08=21、freeze=14；另 2 doctests 通过 |
| torture verifier | 15 unit + 7 CLI + 51 integration tests 通过 |
| probe self-tests | 57 passed, 0 failed；不是实际 target probe |
| 真实 compiler chips scan | exit=1，两个 import 诊断，PCR-10；不属于 fmt/clippy failure |
| 仓库外异常观察 | 12 passed；所有对应异常成立，不是 correctness PASS |
| 仓库外恶意 Worker AST scan | exit=0，漏检 `std::env::var`，PCR-10 |

原始本机日志：
`/private/var/folders/qp/1943jsm56652bhm60grnl74m0000gn/T/opencode/cc-silicon-pre-chip-review/`，
`results.json` 保存标准命令／exit／`check-01.log`–`check-15.log`。
该路径是临时本机辅助证据，可能被清理；以下持久源文件用于独立复跑。

交付前再次用外部 crate 的 `[lib].path` 指向本报告所附 `.rs`，运行同一
`cargo test --manifest-path .../repro/Cargo.toml -- --nocapture`：12 个观察测试通过。
`rustfmt --edition 2021 --check docs/reviews/2026-10-05_PRE_CHIP_READINESS_REPRO.rs`
及 `git diff --check` 均 exit=0。另用本地 Python 内容检查验证了 14 个链接引用的文件存在、
PCR-01–13 标题连续唯一、代码围栏成对、两份新增文件无 CRLF／行尾空白、复现文件恰有 12 个 test。
该检查不验证旧文档语义或 Markdown anchors。最终 Git status 仍保留原 14 个 modified 文件，
本轮新增项仅为两份 review 文件。

### 复跑 12 个观察测试

[2026-10-05_PRE_CHIP_READINESS_REPRO.rs](2026-10-05_PRE_CHIP_READINESS_REPRO.rs) 是本轮仓库外测试源的持久副本，不接入生产 crate／CI，不修改 compiler。

在仓库根设置一个**新建的外部目录**（macOS 优先使用 OpenCode 批准的临时目录），创建 standalone crate，避免写入现有目录：

```sh
REPO="$PWD"
REPRO="$REVIEW_TMP/pre-chip-review-repro"
mkdir -p "$REPRO/src"
cat > "$REPRO/Cargo.toml" <<EOF
[package]
name = "pre-chip-review-repro"
version = "0.0.0"
edition = "2021"
[dependencies]
cc-silicon-compiler = { path = "$REPO/compiler" }
EOF
cp docs/reviews/2026-10-05_PRE_CHIP_READINESS_REPRO.rs "$REPRO/src/lib.rs"
cargo test --manifest-path "$REPRO/Cargo.toml" -- --nocapture
```

`REVIEW_TMP` 必须预先指向有权限的新临时父目录。当前 `/8` 预期为 12 passed；协议修复后有关 observation 应失败，届时把期望反转并移入各 owner 的正式 regression suite，不能把“保持这些观察通过”当作修复目标。

### 复跑计算体扫描漏检（只解析，不执行）

在独立 `$LINT_REPRO` 目录保存一个 `worker.rs`：

```rust
struct ExampleChip;
impl Worker for ExampleChip {
    fn handle(&self) {}
}
impl ExampleChip {
    fn compute(&self) {
        let _ = std::env::var("REVIEW_SYNTHETIC_KEY");
    }
}
```

```sh
cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- "$LINT_REPRO"
cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- compiler/src/chips
```

本轮前者 `strict chip lint passed`（漏检），后者两个 import diagnostic（保守误报）。两者一起证明不能用当前扫描结果给 Worker 模板背书。

## 9. 最终判断

已有可靠的通用 framework 和越来越完整的 compiler foundation，`/8` 的修复有价值；无需推倒重来，也无需先实现整个 C 标准。

**最后准备工作的正确终点是：一个可被强制检查、能失败恢复、能完整重放、且接口唯一的模板 + 逐 slice 冻结的 work orders。** 当前还没到这个终点。先关闭上述基础反例，再开始下一批 chips；不要把 green build、hash 一致或 shell 数量当成放行依据。
