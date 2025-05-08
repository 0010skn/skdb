# FeatherScaleDB (skdb) - 一个基础的键值存储数据库

## 项目简介

FeatherScaleDB (skdb) 是一个使用 Rust 实现的基础键值存储数据库项目。它旨在提供一个轻量级、高效且支持事务和回滚功能的存储解决方案。

## 主要特性

- **模块化设计**: 项目采用模块化架构，主要包括以下几个核心组件：
  - API 层 (`src/api/mod.rs`): 提供与数据库交互的接口。
  - 事务管理 (`src/transaction/mod.rs`): 负责处理事务的开始、提交和回滚。
  - 存储引擎 (`src/storage/mod.rs`): 管理数据的实际存储和检索，并实现了基本的数据版本控制和快照回滚概念。
- **事务支持**: 完整的事务生命周期管理，包括：
  - 开始事务
  - 提交事务
  - 回滚事务
- **基本的数据版本控制和快照回滚**: 存储引擎内部实现了数据的版本控制，允许在需要时回滚到之前的快照。
- **用法演示**: 项目通过 [`src/main.rs`](src/main.rs:1) 文件提供了一个基本的使用示例，展示了如何与数据库进行交互。

## 如何构建和运行

### 环境要求

- 确保您已安装 Rust 开发环境。您可以从 [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) 获取安装指南。

### 构建项目

使用以下命令构建项目：

```bash
cargo build
```

### 运行演示

使用以下命令运行 [`src/main.rs`](src/main.rs:1) 中的演示程序：

```bash
cargo run
```

## 项目结构

- `src/`: 包含项目的核心源代码。
  - `api/`: API 接口实现。
  - `transaction/`: 事务管理逻辑。
  - `storage/`: 存储引擎实现。
  - `log/`: 日志记录模块 (如果适用)。
  - `query_processor/`: 查询处理模块 (如果适用)。
  - [`lib.rs`](src/lib.rs:1): 库的入口点。
  - [`main.rs`](src/main.rs:1): 可执行程序的入口点和用法演示。
- `docs/`: 包含项目相关的文档。
  - [`architecture.md`](docs/architecture.md): 提供了更详细的系统架构说明。
- `examples/`: (未来可能添加) 包含更多的使用示例。
- `tests/`: 包含项目的集成测试和单元测试。

更多关于项目架构的详细信息，请参考 [`docs/architecture.md`](docs/architecture.md)。
