# cc-silicon

面向 **硅基软件架构范式（Silicon-Based Software Architecture Paradigm）** 的可复用 Rust 框架。

`cc-silicon` 不采用面向对象的调用链，而是把软件建模为一个同步数字电路：

- **输入引脚（Input Pins）** 每个 tick 采样一次外部信号
- 扁平的 **系统总线（System Bus）** 保存全部状态（寄存器 + 瞬时导线）
- 无状态的 **逻辑芯片（Logic Chips）** 执行彼此隔离的推理
- **主板时钟（Motherboard Clock）** 驱动确定性滴答

本 crate 刻意不包含任何业务领域。它提供的是范式本身，而不是某个产品。
应用方自行定义 pins、wires、bus 与 chips，然后把芯片接入主板。
一个完整的可运行电路见 [`examples/counter.rs`](examples/counter.rs)。

---

## 为什么需要这个框架

传统代码常围绕类、可变对象图和隐式控制流组织。这种方式可以扩展，但往往带来：

- 隐藏的状态迁移
- 难以重放的 bug
- 模块之间的强耦合
- 对人类与 AI 协作者都很高的上下文负担

`cc-silicon` 借鉴 FPGA/ASIC 设计思想，探索另一种组织方式。其结果是高度确定、易于推理、并对 AI 辅助开发友好的代码库。

---

## 术语约定

| 术语 | 含义 |
|---|---|
| **输入引脚（Input Pins）** | 仅当前 tick 采样并冻结的外部输入 |
| **系统总线（System Bus）** | 单一扁平状态载体：寄存器 + 导线 |
| **寄存器（Registers）** | 跨 tick 持久保存的状态 |
| **导线（Wires）** | 每个 tick 有效、在 tick 开始复位的瞬时信号 |
| **逻辑芯片（Logic Chips）** | 无状态的状态变换单元 |
| **主板（Motherboard）** | 按固定层序调度芯片的确定性执行器 |
| **后端（Backend）** | 语义核心在某基底上的实现（CPU/GPU/HDL…） |
| **滴答（Tick）** | 一次完整的 采样 → 传播 → 锁存 循环 |

在讨论可移植性时，**SFL（Silicon Formal Language）** 是语义真值来源，
Rust/C/CUDA/HDL 代码只是后端实现。完整的多后端契约见
[docs/architecture/SFL_CONTRACT.md](docs/architecture/SFL_CONTRACT.md)。

---

## 一页看懂范式

```text
外部 I/O ──▶ InputPins（冻结快照）
                │
                ▼
     ┌───────────────────────┐
     │       Motherboard     │
     │  layer[0..N] 流水线    │
     └───────────────────────┘
                │
                ▼
     SystemBus · 寄存器 + 导线
                │
                ▼
       Backend（cpu / gpu / …）
```

每个 tick 的生命周期：

1. **采样阶段** —— 主机在边界处把 I/O 采样为 `pins`（位于核心语义之外）。
2. **组合传播阶段** —— 芯片读取 pins/bus，写入 wires。
3. **时序锁存阶段** —— 主板提交边沿状态供下一 tick 使用。

第一阶段与第三阶段是显式的钩子（[`Bus`]），中间阶段由后端负责。

---

## 框架 API

### `Bus` —— 唯一真值来源

```rust
pub trait Bus: 'static {
    type Pins: Clone + 'static;
    type Wires: Default + Clone + 'static;

    fn wires(&self) -> &Self::Wires;
    fn wires_mut(&mut self) -> &mut Self::Wires;

    // 带默认实现的钩子
    fn reset_wires(&mut self) { *self.wires_mut() = Self::Wires::default(); }
    fn latch(&mut self, _pins: &Self::Pins) {}
    fn tick_count(&self) -> u64 { 0 }
    fn advance_tick(&mut self) {}
}
```

应用在一个扁平结构体上实现 `Bus`，其中包含全部寄存器以及内嵌的导线束。

### `LogicChip` —— 无状态变换单元

```rust
pub trait LogicChip<B: Bus> {
    fn tick(&self, pins: &B::Pins, bus: &mut B);
}
```

芯片是零字段单元结构体，彼此不互相调用，数据只通过总线流动。
`tick` 返回 `()`；错误以总线上的“熔断信号”建模，而不用 panic。

### `Motherboard` —— 时钟驱动

```rust
let mut mb = Motherboard::<MyBus>::new(2);
mb.install(0, DecodeChip);
mb.install(1, MutateChip);
mb.clock_tick(&pins, &mut bus);
```

`clock_tick` 按顺序执行三个阶段，且是唯一允许调用芯片、复位或锁存总线的主体。

### `Backend` —— 实现层

`CpuBackend` 是确定性的标量参考实现。通过 `Motherboard::with_backend(...)`
可以替换为批处理、融合、卸载或仿真实现，只要保持可观测语义不变。

### `Clock` —— 墙钟采样

位于主机边界的辅助工具，测量相邻 tick 之间的纳秒差，并对进程挂起等异常做钳制。

### `Testbench` / `simulate` —— 无头验证

输入确定性 pin 序列，对最终总线状态断言性质。

---

## 快速开始

完整程序在 [`examples/counter.rs`](examples/counter.rs)，运行：

```bash
cargo run --example counter
```

核心步骤：定义 `Pins`、`Wires` 与 `Bus`；为每个单元结构体实现 `LogicChip`；
组装层序；开始 tick。完整代码见文档，此处不再重复。

---

## 目录结构

```text
src/
  lib.rs            crate 根与公开导出
  bus.rs            Bus trait（系统总线：寄存器 + 导线）
  chip.rs           LogicChip trait
  motherboard.rs    Motherboard 流水线与 clock_tick 驱动
  backend.rs        Backend trait + CpuBackend 参考实现
  clock.rs          墙钟采样辅助（主机边界）
  sim.rs            simulate() + Testbench（无头验证）
  prelude.rs        便捷重导出
examples/
  counter.rs        一个完整的最小电路
tests/
  paradigm.rs       基于中立域的框架级测试
docs/
  architecture/     范式规范 + SFL 契约/模式
  design/           蓝图 + 入门指南
```

---

## 设计规则

为保持硅基语义，应用应遵守以下物理纯度规则：

- **无全局可变状态**（滴答期间总线之外）。
- **芯片之间不互相调用** —— 只通过总线字段通信。
- **无隐式控制流** —— 错误用熔断导线建模，而非 panic。
- **无权限越界** —— 芯片只触碰与其职责相关的字段。
- **导线仅限当前 tick** —— 不要假设导线会跨越 tick 边界。
- **优先使用测试台与属性测试**，而非零散的经验性单元测试。

其中许多规则由 Rust 在结构上保证：`&mut Bus` 提供唯一写者，`&Pins` 只读，
零字段单元结构体无法隐藏状态。因此 `rustc` 是*局部的设计规则检查器*——它
无法验证业务逻辑，后者仍需测试，必要时还需形式化方法。

---

## 测试与验证

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --example counter
```

该范式可干净地映射为 Mealy/Moore 状态机：

```text
S(t+1) = F(S(t), I(t))
```

其中 `S` 是完整总线快照，`F` 是固定层序的芯片流水线。由于 `F` 是纯函数且
全域可定义，系统可被完整刻画为数学函数，因此确定性重放、模糊测试和属性测试
都是自然选择。

---

## 后端展望

CPU 后端已完整实现，并作为所有行为的参考。SFL 契约定义了后续实现的边界：

| 后端 | 状态 | 说明 |
|---|---|---|
| CPU 标量（参考） | 稳定 | 即本 crate |
| SIMD / 加速 CPU | 未来 | 必须保持语义等价 |
| GPU | 未来 | 可批处理/融合；其余芯片仿真或拒绝 |
| FPGA / ASIC（HLS） | 研究 | 仅限定宽、可综合子集 |
| 量子导向 | 前瞻 | 接口层编排与可逆子电路 |

规则始终不变：后端只实现语义，绝不重新定义语义。后端无法表示的行为必须
被显式拒绝或仿真——**禁止静默的语义漂移**。

---

## 文档

| 文档 | 用途 |
|---|---|
| [docs/architecture/SILICON_PARADIGM_SPEC.md](docs/architecture/SILICON_PARADIGM_SPEC.md) | 范式、原则与基本元件 |
| [docs/architecture/SFL_CONTRACT.md](docs/architecture/SFL_CONTRACT.md) | 多后端语义契约 |
| [docs/architecture/SFL_SCHEMA_DRAFT.md](docs/architecture/SFL_SCHEMA_DRAFT.md) | 可供工具处理的结构化文档形态 |
| [docs/design/ARCHITECTURAL_BLUEPRINT.md](docs/design/ARCHITECTURAL_BLUEPRINT.md) | 如何在 cc-silicon 上构建系统 |
| [docs/design/GETTING_STARTED.md](docs/design/GETTING_STARTED.md) | 分步入门指南 |
| [README.md](README.md) | English README |

---

## 许可证

MIT —— 详见 [LICENSE](LICENSE)。
