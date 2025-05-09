# sk-runtime 命令行工具及 .hs 文件格式指南

本文档介绍了 `sk-runtime` 命令行工具的用法以及其处理的 `.hs` 数据文件的格式。

## 1. `sk-runtime` 命令行工具

`sk-runtime` 是一个用于对 `.hs` 文件中定义的数据执行查询和更新操作的命令行实用程序。当通过 `-f` 参数指定文件并执行了修改数据的操作（如更新或添加）后，所做的更改将会写回到原始输入文件中。

### 1.1 构建与运行

可以使用 Cargo 构建和运行 `sk-runtime`：

- **运行 (开发模式)**：
  ```bash
  cargo run --bin sk-runtime -- [OPTIONS] [STATEMENTS...]
  ```
- **构建 (发布模式)**：
  ```bash
  cargo build --release
  ```
  然后可以直接运行 `target/release/sk-runtime` 可执行文件。

### 1.2 命令格式

```bash
sk-runtime -f <input_file.hs> [STATEMENT_1] [STATEMENT_2] ...
```

### 1.3 参数说明

- `-f, --file <FILE_PATH>` (必需):
  指定要处理的输入 `.hs` 文件的路径。

- `STATEMENTS...` (必需, 至少一个):
  一个或多个要按顺序执行的 DSL (领域特定语言) 语句。这些语句可以是查询或数据修改操作。

### 1.4 支持的语句类型 (通过命令行)

`sk-runtime` 通过命令行参数主要支持以下几种类型的语句：

1.  **查询 (Query)**:

    - 格式: 以 `"#."` 开头。
    - 示例:
      - `"#.tablename[0].fieldName"`
      - `"#.tablename{primary_key_value}.fieldName"`
      - `"#.tablename"` (当前可能返回 "Not found" 或错误)
    - 输出: 查询结果会打印到标准输出。

2.  **更新 (Update)**:

    - 格式: 以 `"#."` 开头，并包含一个等号 `=`。
    - 示例: `"#.tablename[0].fieldName = New Value"`
    - 输出: 成功或失败的状态信息。更改会写回文件。

3.  **添加 (Add)**:
    - 格式: 以 `.` 开头，并以 `.add()` 结尾。
    - 示例: `".tablename.add()"`
    - 输出: 成功或失败的状态信息。更改会写回文件。

### 1.5 示例用法 (简单表)

假设 `data.hs` 内容为:

```hs
user:
/id::sindex/name/age::integer/
0,Alice,30
~
```

查询年龄:
`cargo run --bin sk-runtime -- -f data.hs "#.user[0].age"`
预期输出 (核心): `Query result: Integer(30)`

### 1.6 文件回写机制的重要说明

当通过 `-f` 参数指定输入文件时，`sk-runtime` 会在处理完所有命令行语句后，将内存中数据的最终状态写回到该输入文件，覆盖其原始内容。
**重要**: 此回写过程目前只序列化表定义和数据。原始文件中的**注释行**和通过参数模式识别的**指令行**（如 `source_table from "path"`）在文件被回写后将会**丢失**。为了重复测试包含这些结构的文件，用户必须在每次运行前手动恢复原始文件内容。

## 2. `.hs` 文件格式

### 2.1 基本结构

文件由注释、指令和表定义组成。

### 2.2 注释

- 任何以 `#` 或 `##` 开头的行都被视为注释，并在解析时被忽略。
- 行内注释（在有效内容之后，以 `#` 或 `//` 开始）在解析表定义时会被移除。

### 2.3 表定义

一个标准的表定义结构如下：

```hs
表名:
/字段名1::类型1/字段名2/字段名3::类型3/
值1_1,值1_2,值1_3
...
~
```

(详细说明同前，包括表名行、表头行、数据行、表结束符)

### 2.4 数据类型

(说明同前：String, Integer, Boolean, Reference, Null/Empty)

### 2.5 指令 (Directives)

指令用于组织和引用数据。它们通过其参数的结构模式被识别，并且**不以 `#` 开头**。任何以 `#` 开头的行都是纯注释。

- **复制结构 (Copy Structure)**:

  - 格式: `源表名 from "文件路径" as 目标表名`
  - 描述: 用于从另一个 `.hs` 文件中复制一个表的结构（表头定义），但不复制数据。
  - 关键字: `from`, `as` (均为英文)。
  - 文件路径应为带引号的字符串。

- **引用 (Reference)**:

  - 格式: `源表名 from "文件路径"`
  - 描述: 用于从另一个 `.hs` 文件中完整地引用一个表，包括其结构和数据。被引用的表在内存中将保持其 `源表名`。
  - 关键字: `from` (英文)。
  - 文件路径应为带引号的字符串。

- **Pack**:
  - 格式: `pack 表名1 表名2 ...`
  - 描述: 此指令用于将多个表的定义打包。**`sk-runtime` 在其当前实现中会忽略此指令。**

### 2.6 使用指令的复杂示例

本节演示如何结合使用“引用”和“复制结构”模式的指令。

**重要提示**:

- 以下示例中的指令行（如 `department from ...`）不以 `#` 开头。
- 由于文件回写机制，为了重复测试，用户必须在每次运行 `sk-runtime` 前手动恢复 `main_test.hs` 到其包含指令的原始状态。

假设项目根目录下有 `examples/runtime_tests/` 文件夹，包含：

1.  **`examples/runtime_tests/source_dept.hs`**:

    ```hs
    department:
    /dept_id::index/dept_name/location/
    D101,Engineering,Building A
    D102,Marketing,Building B
    D103,HR,Building C
    ~
    ```

2.  **`examples/runtime_tests/source_proj_structure.hs`**:

    ```hs
    project_structure:
    /proj_id::sindex/proj_name/status/deadline/
    ~
    ```

3.  **`examples/runtime_tests/main_test.hs`** (运行前确保是此内容):

    ```hs
    # 这是一个注释行
    # main_test.hs - 主测试文件

    department from "examples/runtime_tests/source_dept.hs"
    project_structure from "examples/runtime_tests/source_proj_structure.hs" as active_projects

    employee:
    /emp_id::sindex/emp_name/dept_ref::department/project_ids::string/
    0,John Doe,D101,"P001 P002"
    ~

    active_projects:
    /proj_id::sindex/proj_name/status/deadline/
    0,Alpha Project,In Progress,2025-12-31
    ~
    ```

#### 命令行调用示例

1.  **查询被引用表 (`department`) 中的数据**:
    _(确保 `main_test.hs` 是原始版本)_

    ```bash
    cargo run --bin sk-runtime -- -f examples/runtime_tests/main_test.hs "#.department{D101}.location"
    ```

    预期输出 (核心部分):

    ```
    --- Parsing input file: examples/runtime_tests/main_test.hs ---
    CLI: Processing 复制结构: source_table='project_structure', source_path='examples/runtime_tests/source_proj_structure.hs', target_table='active_projects'
    CLI: Processing 引用: source_table='department', source_path='examples/runtime_tests/source_dept.hs', target_table='department'
    CLI Warning: Table 'active_projects' (not an empty shell from #复制结构) encountered a subsequent definition block.
    CLI: Replacing table 'active_projects' with new definition.
    --- Executing Command Line Statements ---
    Executing: #.department{D101}.location
    Query result: String("Building A")
    --- Persisting changes to examples/runtime_tests/main_test.hs ---
    Successfully wrote changes to examples/runtime_tests/main_test.hs
    ```

    **注意**: 运行此命令后，`examples/runtime_tests/main_test.hs` 将被覆盖，原有的指令行和注释会丢失。

2.  **查询被复制结构并填充了数据的表 (`active_projects`)**:
    _(确保 `main_test.hs` 已恢复到包含指令的原始版本)_
    ```bash
    cargo run --bin sk-runtime -- -f examples/runtime_tests/main_test.hs "#.active_projects[0].status"
    ```
    预期输出 (核心部分): `Query result: String("In Progress")`

这个指南应该能帮助用户理解如何使用 `sk-runtime` 工具以及如何构造 `.hs` 数据文件。
