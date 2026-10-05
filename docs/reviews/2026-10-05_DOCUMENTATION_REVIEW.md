# 文档 Review：2026-10-05

## 状态与范围

- 状态：审查记录，尚未逐项修复或关闭；本文不是 ADR、契约批准、接口冻结或实现授权。
- 审查日期：2026-10-05。
- 基线：审查时的工作区，包含已有未提交和未跟踪的文档，不限于 Git HEAD。行号对应审查时版本，后续修改可能导致漂移。
- 范围：根 README、架构与设计文档、编译器任务目录、编译器契约与 README、torture 工具及目标探针文档；必要时核对相关实现和测试源码。
- 方法：第一轮 5 个子 agent 分域并行审查；第二轮 3 个子 agent 定向审查，排除第一轮已记录问题。两轮均由主 agent 交叉核对关键发现。
- 本轮 review 未修改被审查文件，也未提交代码。本文仅保存审查结果。

总体结论：文档对“已实现、已接受、提案”的区分总体清楚，但仍有契约保证与实现不符、验收步骤自相矛盾的问题。建议修正以下问题后再冻结 `/6`。明确标为开放的提案缺口不应被理解为已实现编译器的故障。

## 优先修复

### DOC-01：提交权限保证强于实际检查

- 严重程度：高。
- 状态：待处理。
- 位置：`compiler/contracts/COMPILER_SFL_MANIFEST.md:97–110`。
- 问题：文档声称生产者未声明任务 kind 就会被拒绝；实际只有 `StorePatch` 检查生产者注册及 kind，`Complete`、`Fail`、`AwaitHost` 没有同等检查。`Enqueue` 检查的是目标芯片注册及目标 kind。
- 证据：`compiler/src/commit.rs:384–475,608–617`；`compiler/tests/c03_task.rs:104–118` 在没有注册生产者 manifest 的情况下完成基础任务。
- 建议：统一所有 proposal 的生产者校验；若基础任务允许例外，应明确例外范围，并为每种 proposal 增加未注册或不接受 kind 的负例。

### DOC-02：Part A 验收命令违反目标探针门禁

- 严重程度：高。
- 状态：待处理。
- 位置：`docs/tasks/M1_TARGET_ACCEPTANCE.md:466–479`。
- 问题：Part A 要求运行 `candidate … -S -o main.s`，但同文 `330–341` 明确规定未验证目标时必须拒绝目标代码生成，Part A 不生成目标代码。照此步骤执行，要么 Part A 失败，要么违反门禁。
- 建议：Part A 仅生成快照/IR 并解释执行；将 `-S` 和 assembly hash 比较移到探针完成后的 Part B，再组装与链接。

## 其他实质性问题

### DOC-03：负例的 C 语言判定错误

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md:306`。
- 问题：`return x+3;` 中未声明的对象 `x`，不能套用 GNU89 的隐式函数声明规则。当前验收 oracle 可能把错误接受当作通过。
- 建议：要求未声明标识符诊断；隐式函数声明另用调用表达式测试，并显式声明 dialect 策略。

### DOC-04：T02 权限上限不足以完成自身任务

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/tasks/T02_CONTROL_CHIPS.md:3,99,108,111`。
- 问题：包级上限只允许读取 `config/control/tasks`、写自己的 control records，但 CT02 导入 source，CT11 处理 diagnostics，CT14 读取 artifacts。T01 将包级 envelope 视为权限上限，因此仅细化逐芯片字段不能消除矛盾。
- 建议：补齐包级权限上限，并继续用逐芯片 manifest 限定具体字段，保留 commit-only mutation。

### DOC-05：宏展开依赖图把粘贴放在参数替换之前

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/tasks/T03_PREPROCESS_CHIPS.md:19–23,38`。
- 问题：依赖图为 `…11/13/14→12→15`，但 PP14 必须使用 PP12 替换后的参数 token/placemarker。按此顺序实现 `CAT(a,b)` 容易粘贴形参名而不是实参 token。
- 状态边界：同文 `42–44` 将上述表格和调度文字标为历史；本项是历史指导仍可能误导实施者的问题，不是当前实现的预处理器故障。
- 建议：明确 `##` 位置的原始参数替换 → 粘贴/placemarker 处理 → 重扫描；区分普通实参的预展开。为历史错误标注替代依赖图。

### DOC-06：注释替换任务遗漏字面量保护约束

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/tasks/T03_PREPROCESS_CHIPS.md:11–12,38`。
- 问题：PP03 在 token 扫描前替换注释，却未明确要求保护字符串、字符字面量及相关 header-name 上下文。简单按字节替换可能破坏 `"https://example"` 或 `"/*not a comment*/"`。
- 建议：明确扫描状态与 PP04 的协作契约，增加上述字面量、转义引号、include 上下文及行拼接测试。

### DOC-07：已接受 ADR 错称整个框架无堆分配

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md:5–8,50–51`。
- 问题：ADR 将根框架及例子描述为 heap-free，但实际 `Motherboard` 使用 `Vec` 和 `Box`，构造/安装阶段会分配。
- 证据：`src/motherboard.rs:20–38,57–71`。
- 建议：区分普通应用 bus 的固定布局约束、拓扑初始化分配，以及默认 tick 调度自身不分配。不要将 bus 存储约束扩大为整个框架的资源保证。

### DOC-08：探针的 fail-closed 描述未覆盖行为自检失败

- 严重程度：中。
- 状态：待处理。
- 位置：`tools/torture/probe/README.md:23–34`。
- 问题：ABI 自检失败仅输出 `.ok=0`，程序仍返回 0；normalizer 只是复制这些字段，harness 不据此拒绝报告。
- 证据：`tools/torture/probe/src/abi-args.c:139–150`、`tools/torture/probe/normalize/normalize.py:362–375`；子 agent 以合成 capture 将 `abi.gp_ten.ok` 改为 `0`，观察到 `build_report()` 仍接受。
- 建议：自检失败返回非零，并验证必需 `.ok` 字段；否则明确报告生成成功不代表自检通过。合成 capture 检查不等于真实 AArch64 探针执行。

### DOC-09：SFL 完整示例违反自身字段规则

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/architecture/SFL_SCHEMA_DRAFT.md:295–324`。
- 问题：`EdgeDetect` 读取 `prev_pulse`，但 bus 没有声明该寄存器，也未描述它的 latch 更新，与同文字段引用规则不一致。
- 建议：补齐声明、初值及更新语义，使示例能被未来 validator 接受。本项是草案示例的内部不一致，不是已实现 validator 的故障。

## 第二轮新增发现

本轮新增 9 项，编号接续第一轮。候选契约中的矛盾不代表相应语言芯片已实现，也不构成修改已接受决策或冻结 `/6` 的授权。

### DOC-10：字面量解码存在前置依赖闭环

- 严重程度：高。
- 状态：待处理；T04 `/6` 候选契约问题。
- 位置：`docs/tasks/T04_LEX_CHIPS.md:84–86,115,158`。
- 问题：literal decode 子请求要求已有 committed C `TokenId`，但 token 与解码后的 literal 又必须在同一任务 append batch 发布。解码前没有该 committed token；先发布 token 又不符合同批要求。
- 证据：同文 `87–90` 的整批 ID 预测机制只能解决提交时的 reciprocal links，不能提前提供下一子任务所需的 committed 输入。
- 建议：从 committed PP token 的 spelling/kind 解码，再在发布时解析 C token 回链；或显式设计 token-first 的发布/更新协议。增加覆盖解码输入可用性和发布顺序的集成 fixture。

### DOC-11：全局资源上限可被公共 mutation API 绕过

- 严重程度：高。
- 状态：待处理；当前文档保证与实现边界不一致。
- 位置：`compiler/README.md:130–133`；`docs/tasks/T01_COMPILER_CONTRACT.md:152`。
- 问题：文档承诺所有配置上限在 mutation 前检查，但公开的 `bus.arenas` 可直接分配；arena 仅检查单 arena 容量，不检查总记录数、source bytes、任务或诊断总预算。分别向多个 arena 分配可以在各自容量内超过全局记录预算；已有 source 的字节也可通过公开可变访问增长。
- 证据：`compiler/src/bus.rs:101–147,355`；`compiler/src/arena.rs:275–287,305–310,377–389`。`compiler/tests/c07_limits.rs:55–106` 覆盖受检包装路径，不建立所有公共修改入口的预算保证。
- 建议：将预算感知的 mutation 集中到封装入口；或明确保证只适用于受检 bus/commit 路径，并记录可信 integration 代码可绕过的边界。不可仅用包装路径测试支持无条件全局保证。

### DOC-12：声明可见性的起点取错

- 严重程度：中。
- 状态：待处理；T06 `/6` 候选语义规则问题，当前 M1 正例不暴露该错误。
- 位置：`docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:37–41`；`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1300–1319`。
- 问题：候选规则以标识符叶节点的位置作为 point of declaration。C11 §6.2.1 ¶7 对普通标识符规定其作用域从完整 declarator 结束后开始，而非名字出现后立即开始。
- 反例：下例数组界限应查询外层 `n`，因为内层 declarator 尚未完成；按 identifier 位置判断会过早暴露内层 `n`。

```c
int n = 3;
void f(void) {
    int n[n];
}
```

- 建议：保留 identifier leaf 作为声明身份与诊断位置，另从完整 declarator 推导语义可见性边界；增加数组界限与 initializer 查询的对比测试。若规则仅适用于受限 M1，应明确不将其作为通用 C lookup 规则。

### DOC-13：作用域唯一性规则会拒绝合法兄弟作用域

- 严重程度：中。
- 状态：待处理；T06 `/6` 候选结构不变量问题。
- 位置：`docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:39`；`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1236–1241`。
- 问题：禁止重复 live `(parent, kind)`，但两个函数体或兄弟 block 可以具有相同 parent 和 `Block` kind。关闭作用域保留其记录，因此用 live arena record 判定重复不能解决这一碰撞。
- 证据：`docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:72` 要求关闭作用域后保留记录；`int f(void){return 1;} int g(void){return 2;}` 的两个 block 需要独立作用域。
- 建议：以所属词法节点或其他显式 identity 区分作用域，拒绝同一 owner 的重复创建，而不是相同 `(parent, kind)`。若仅限制 M1 单函数基数，应将 fixture 限制与通用结构规则分开。

### DOC-14：所有合法位数预算都容纳 M1 常量的声明不成立

- 严重程度：中。
- 状态：待处理；T08 M1 候选断言问题。
- 位置：`docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md:10,39–41,46`。
- 问题：文档允许小于等于 128 的预算，却声称 `2`、`3`、`5` 对每个合法 `max_const_bits` 都可表示。预算为 1 时，`2` 的 magnitude 已需要 2 位，`5` 需要 3 位，尚未计入符号表示。
- 状态边界：通用 signed-range 公式仍是未选定草案；本项不以该公式为已接受规则，而是指出撤回公式不能建立替代的普遍可表示性保证。
- 建议：将成功 M1 验收限定到足够预算（例如默认 128）；不足预算验证 typed overflow/capacity 结果。不得未经批准自行提高配置最小值。

### DOC-15：可选 artifact 的 source 策略未同步到提交算法

- 严重程度：中。
- 状态：待处理；跨文件 `/6` 候选一致性问题。
- 位置：`docs/tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md:136`；`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:2460–2462,2540–2544`。
- 问题：CDR rev 47 已选候选默认允许 map-optional artifact 携带有效 `Some(source)` 且要求空 offsets；但 proposal 的提交算法仍拒绝可选 kind 的任何 source，并要求 `source=None`。
- 反例：`Trace` artifact 携带有效 source 和空 `raw_offsets` 符合候选默认，却被算法拒绝。
- 建议：同步算法与 draft 校验表：optional kinds 要求空 offsets；`Some(source)` 应验证有效性，而非一律拒绝。Trace 不在 M1 正例生产范围内，但已声明总 schema 的合成契约测试仍须一致。

### DOC-16：ABI 探针聚合分类和 HVA 覆盖声明不准确

- 严重程度：中。
- 状态：待处理。
- 位置：`tools/torture/probe/src/abi-args.c:38–42,64–68`；`tools/torture/probe/README.md:23–26`。
- 问题：`struct { long i; double d; }` 非同质 16-byte aggregate 被描述为 GP+FP split。在冻结目标的 AAPCS64 分类下，其参数使用连续 GP registers，返回使用 `x0/x1`，包含 double 的表示也不因此改用 FP register。README 还声称覆盖 HVA，但源码没有 short-vector aggregate fixture。
- 建议：修正聚合分类说明，区分混合 scalar 参数与 aggregate classification；补真实 HVA fixture，或明确 HVA 未测试。不能以此错误描述解释 assembly evidence 或宣称对应 ABI class 已覆盖。

### DOC-17：纯函数与 totality 被写成无条件保证

- 严重程度：中。
- 状态：待处理。
- 位置：`README.md:325–328`。
- 问题：`Because F is pure and total` 被表述为既有保证，但公共 API 接受任意 `LogicChip`，并不保证纯、终止或不 panic。README 例子的普通整数加法在边界状态下也未给出明确溢出结果。
- 证据：`src/chip.rs:29–36`；`src/backend.rs:31–40`；`README.md:231,244`。同文 `129–131` 已承认 Rust 不能证明不存在 I/O 或 nondeterminism。
- 建议：将数学模型表述为遵守 purity、determinism、termination 和显式错误/溢出契约的 chips、hooks、adapters 与 backend 所满足的条件性性质；区分 replay 测试与证明。

### DOC-18：H00 状态文档夸大 provenance 检查强度

- 严重程度：中。
- 状态：待处理。
- 位置：`docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md:107–110`。
- 问题：状态记录将 provenance revision token 描述为已检查条件；实际缺少 token 只产生 warning，即使 frozen lock 也不因此验证失败。
- 证据：`tools/torture/src/lock.rs:1337–1347`、`tools/torture/src/verify.rs:68–95`；`tools/torture/README.md:112–116` 正确描述为 policy nudge。
- 建议：明确 warning-only，并说明成功 frozen verification 不建立逐 asset revision linkage；若需要更强 freeze gate，另行批准、实现并测试。

## `/6` 冻结前仍需闭合的设计问题

以下项目记录提案的未闭合路径（OPEN-01/OPEN-02 已记录解决方向与测试要求，见下；其余仍未闭合），不声称相应编译器行为已经实现，也不因“待设计”本身判定代码存在故障。

### OPEN-01：作用域启动顺序

- 状态：已提交候选解决方向与测试要求（提案 §5/§24.14；T05 item F point 6；T06 item 8）；仍待 T01/T05/T06 选择/冻结，尚未关闭。
- 位置：`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1241–1248`（原审查行号）；`docs/tasks/T05_PARSE_CHIPS.md:3–5`。
- 风险：提案要求 TU node 提交后打开文件作用域，而 T05 解析依赖作用域查询。必须明确 TU root 是否提前提交；如果等完整 TU 解析完成才提交，则可能形成启动依赖环。
- 建议：明确早期提交 TU root 的顺序，或显式限定 M1 的无作用域解析路径，并测试不预建 scope 的完整启动过程。
- 处理记录（2026-10-05，doc-only）：提案 §24.14 记录选定候选“早期 TU root 提交顺序”——词法阶段提交完整 token 流（含 EOF）后，注册的 `parse.TranslationUnit` 任务在首个可见提交批次提交 TU root（`parent: None`，范围为首个已提交 token 至已提交 EOF；空 TU 为 EOF 处空范围），已接受的 `parse.TranslationUnit -> symbol_type.scope-enter` 边在该已提交 root 上恰好一次触发文件作用域 Enter；解析任务在 `symbol_type` 任务可调度期间保持 `Waiting`（`(stage ordinal, …)` 调度序下 ready 的 parse 任务会先于 symbol_type 任务，必须等待），文件 Enter 提交后再恢复解析并携带已提交的文件 `ScopeId`，从而保留 T05 的解析期作用域/typedef 可见性，无需无作用域特例。回退方案为显式限定的 M1 无作用域解析路径（parse 期不做解析决策所依赖的 scope/typedef 查询；块作用域请求延后到文件 Enter 之后；不得读作通用无作用域解析器）。两条路径均以新增必需待注册 fixture `M1-START-01`（不预建 scope 的完整启动测试）验证：TU root 提交前零 scope/事件、恰好一个文件 scope 与一个 Enter（`at` 为已提交 TU root、无 Exit）、边恰好一次且重观察不产生第二次、replay 一致、文件 Enter 前的作用域/typedef 查询为 typed unsupported/diagnostic 而非猜测。该候选同时限定 T05 item F point 3 与 T06 item 7 的触发仅适用于 File-Enter 边（触发为已提交 root append，而非任务终态；恰好一次由单次 root append 结构性保证，`ResultRecord.consumed` 不再为该边所必需，OB-11/OB-50 仍开放）。具体 edge 入队实现（同批 link 或 T01 commit-apply hook）、重复 root 校验、root 范围/后代不变量与 bootstrap task kind/stage 仍为 T01/T05/T06 `/6` co-freeze（OB-53）；`M1-PA-04`、`M1-PA-05`/`M1-TY-05`、`M1-TY-06` 均未被削弱。

### OPEN-02：join 后结果交付

- 状态：已记录解决方向与测试要求（提案 §5/§7/§13/§18.3 与 §24 OB-4）；仍待 T01/T02/T05 选择具体机制并冻结，尚未关闭。
- 位置：`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:2621–2631`（原审查行号）。
- 风险：commit-apply 在恢复父任务时已经消费子结果，但父任务下一 tick 如何持久获取并处理结果仍未闭合。恰好一次消费不等于恰好一次语义处理。
- 建议：将消费保留给父任务的原子提交，或在 join 时转移到明确归属父任务的持久状态；测试 join 后、父任务执行前的 replay/retry。
- 处理记录（2026-10-05，doc-only）：提案 §7 join realization 与 §5 已删除“join 时消费子结果”的未闭合表述，改为选定方向——join 只判定子任务终态、不消费结果；消费保留给父任务自身下一 tick 的原子提交（结果保持 committed/unconsumed，父任务提交时以 `ResultAlreadyConsumed` 守卫恰好一次消费），仅在 `/6` 冻结要求 join 消费时才转移为明确归属父任务的持久状态；两种实现都必须保证 join 与父任务执行之间的 replay/retry 不重复消费、不丢失结果，并新增必需测试 `join_then_replay_retry`（提案 §13；T13 H6-M15）。具体载体与消费/提交封装仍为 T01/T05/T02 `/6` co-freeze（OB-4）。

### OPEN-03：`2+3` 的常量求值输入

- 状态：待 T01/T07/T08 co-freeze。
- 位置：`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1527–1535`；`docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md:49`。
- 风险：示例 request 强制携带单个 literal，二元表达式的 operator/operand 协议仍开放；固定结果 `5` 尚不足以定义真实 handoff。
- 建议：冻结 committed expression/operand/operator 输入路径及任务/结果类型，增加真实 T07→T08→T09 fixture，不能以手工构造 `5` 替代求值。

## 验证记录与限制

- 使用只读 Python 检查根 README、AGENTS、`docs/`、`compiler/`、`tools/` 共 36 个 Markdown 文件的本地内联链接目标是否存在：未发现缺失目标。
- 该检查未验证链接锚点、引用式链接或文档语义。
- 两轮其余结论来自文档、实现和测试源码核对；DOC-08 另有第一轮子 agent 的合成 capture 检查。第二轮未运行额外执行测试。
- 未运行 Cargo fmt/clippy/test，未执行真实 Linux/AArch64 探针，未运行候选编译器或 GCC torture 验收。
- 本文不报告编译器能力、目标验证结果、GCC 通过率或形式化证明。
- 本文共保存 18 项问题及 3 项冻结前待闭合设计，不宣称已穷尽所有文档缺陷；除 OPEN-02 已记录解决方向与测试要求（仍未冻结/关闭）外，所有条目仍待处理，没有因保存报告而被认定已修复或关闭。
