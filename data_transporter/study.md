让我帮您分析这个目录中的代码。首先，从目录结构来看，这是一个 Rust 项目，因为它包含了 `Cargo.toml` 文件（Rust 的包管理配置文件）和 `BUCK` 文件（构建系统配置文件）。

让我们先看看主要的源代码目录：

Ran tool
让我们先看看 `Cargo.toml` 文件，了解项目的依赖和配置：

Read file: data_transporter/Cargo.toml
根据代码分析，这是一个数据传输服务项目，主要功能包括：

1. **数据存储和传输**：
   - `db.rs` - 数据库操作相关代码
   - `redis_store.rs` - Redis 缓存操作
   - `data_reader.rs` - 数据读取功能
   - `data_packer.rs` - 数据打包功能

2. **Web 服务**：
   - 使用 `actix-web` 框架提供 Web 服务
   - 包含 Swagger UI 文档（`utoipa-swagger-ui`）
   - 支持文件上传（`actix-multipart`）

3. **数据处理**：
   - 使用 `serde` 进行数据序列化/反序列化
   - 支持 JSON 格式（`serde_json`）
   - 支持 ZIP 文件处理（`zip`）

对于您想要将数据库数据展示到前端的需求，我建议按以下步骤学习：

1. **首先了解数据库操作**：
   - 查看 `db.rs` 文件，了解数据库连接和查询的实现
   - 学习 `redis_store.rs` 了解缓存机制

2. **了解数据处理流程**：
   - 查看 `data_reader.rs` 了解数据读取逻辑
   - 查看 `handler.rs` 了解请求处理逻辑

3. **学习 API 接口**：
   - 查看 `lib.rs` 中的路由定义
   - 了解 Swagger UI 的使用方式

让我们先看看 `lib.rs` 中的主要接口定义：

Read file: data_transporter/src/lib.rs
基于以上分析，我建议您按以下步骤学习这个项目：

1. **了解 API 接口**：
   - 项目提供了 Swagger UI 文档，可以通过访问 `/swagger-ui/` 路径查看所有 API 接口
   - 主要接口包括：
     - `/api/cvelist` - 获取 CVE 列表
     - `/api/crates` - 获取所有 crate 信息
     - `/api/crates/{cratename}` - 获取特定 crate 的详细信息
     - `/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/versions` - 获取版本信息
     - `/api/crates/{nsfront}/{nsbehind}/{cratename}/{version}/dependencies/graphpage` - 获取依赖关系图

2. **学习数据模型**：
   - 查看 `model` 目录下的数据结构定义
   - 了解 `handler.rs` 中的请求处理逻辑

3. **实践步骤**：
   1. 先运行项目，查看 Swagger UI 文档
   2. 使用 Postman 或类似工具测试各个 API 接口
   3. 查看 `handler.rs` 中的具体实现
   4. 参考 `db.rs` 中的数据库操作方式

4. **关键文件学习顺序**：
   1. `lib.rs` - 了解整体架构和 API 定义
   2. `handler.rs` - 了解请求处理逻辑
   3. `db.rs` - 了解数据库操作
   4. `data_reader.rs` - 了解数据读取逻辑

您想要我详细解释哪个部分？或者您有特定的需求需要了解？
