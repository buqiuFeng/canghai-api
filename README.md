<div align="center">

# 沧海·API 调试工具

**轻量级跨平台 HTTP 调试桌面应用，基于 Tauri 2 + Vue 3 构建**

无 CORS 限制 · 多标签页 · 环境变量分组 · 前后置脚本 · Mock 响应 · 导入导出 · 工作区 · 云同步

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Vue](https://img.shields.io/badge/Vue-3.5-42b883.svg)](https://vuejs.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-ffc131.svg)](https://tauri.app/)
[![Element Plus](https://img.shields.io/badge/Element%20Plus-2.9-409eff.svg)](https://element-plus.org/)
[![Spring Boot](https://img.shields.io/badge/Spring%20Boot-4.x-6db33f.svg)](https://spring.io/projects/spring-boot)

[English](./readme_en.md)

</div>

---

## 简介

沧海·API 调试工具是一款运行于桌面端的轻量级 HTTP 请求调试工具。依托 Tauri 原生后端代理发送请求，从根本上规避了浏览器的跨域（CORS）限制，让调试本地服务、内网接口和生产环境 API 同样流畅。

服务端采用 **Spring Boot 4**（webmvc + MyBatis-Plus + Druid + Fastjson2），集成 JWT（RS256 非对称）鉴权，提供工作区协作与数据云同步能力。

---

## 功能特性

### 核心调试
- **多标签页**：同时打开多个接口，自由切换，互不干扰；标签支持双击重命名、右键菜单（关闭其他/关闭所有）
- **全 HTTP 方法**：GET / POST / PUT / PATCH / DELETE / HEAD / OPTIONS
- **请求体类型**：JSON（支持注释）、form-urlencoded、纯文本、none
- **请求参数 & 请求头**：可视化键值编辑器，支持行级启用 / 禁用
- **响应展示**：Pretty / Raw 双视图，响应头列表，请求详情，状态码、耗时、Body 大小一览
- **快捷键**：支持 `Ctrl+S` 快速保存当前请求

### 环境变量
- 创建多套环境（开发 / 测试 / 生产），一键切换
- **环境分组**：支持按项目或用途对环境进行分组管理
- 使用 `{{变量名}}` 语法在 URL、Header、Body、脚本中引用变量
- 环境变量支持运行时写入（通过脚本 `env.set()`）
- 脚本写入的变量可即时参与当前请求的 `{{var}}` 解析

### 前置 & 后置脚本
- 在请求发送前 / 收到响应后执行自定义 JavaScript（支持 async/await）
- 前置脚本可动态修改 `req.url`、`req.headers`、`req.body`
- 后置脚本可读取 `res.status`、`res.body`、`res.json()` 并写入环境变量
- **内置 CryptoJS**：支持 MD5、SHA256、HMAC、AES、Base64 等加密算法
- 内置 `console`、`test()`、`assert()` 工具函数，脚本日志在响应区独立展示

### 接口管理
- 按分类树保存常用接口，支持多级嵌套分类
- **分类树拖拽**：支持拖拽移动分类节点，灵活调整接口组织结构
- **导入 / 导出**：支持将接口以 JSON 格式导出保存到本地，或从 JSON 文件批量导入接口
- 一键保存 / 覆盖更新当前请求
- 历史记录自动记录每次请求，支持快速回溯

### 工作区与云同步
- **多工作区**：创建多个独立工作区，隔离不同项目或团队的接口数据
- **云同步**：登录账号后可将环境变量、接口、分类等数据同步至服务端
- **权限控制**：支持所有者、管理员、成员、只读四种角色，只读成员不可修改数据
- **服务端**：提供 Spring Boot 服务端（端口 8092），支持数据持久化与团队协作

### 三种工作模式

| 模式 | 说明 |
|------|------|
| **调试模式** | 主力模式，发送请求并实时查看响应 |
| **预览模式** | 查看解析后的 URL、参数、请求头，一键生成 cURL 命令 |
| **设计模式** | 编写 Markdown 接口文档，配置 Mock 场景，无需真实后端即可前端联调 |

### 用户体验
- **侧边栏宽度可调**：拖拽侧边栏边缘自由调整宽度，支持本地持久化
- **智能页签复用**：点击已打开的接口自动复用现有页签，避免重复打开
- **面包屑导航**：显示当前接口的分类路径和编辑状态

### 无 CORS 限制
通过 Tauri Rust 后端发起真实 HTTP 请求，完全绕过浏览器同源策略，可调试任意域名接口。

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | [Tauri 2](https://tauri.app/) |
| 前端框架 | [Vue 3.5](https://vuejs.org/) + [TypeScript 5.7](https://www.typescriptlang.org/) |
| UI 组件库 | [Element Plus 2.9](https://element-plus.org/) |
| 构建工具 | [Vite 6](https://vitejs.dev/) |
| Markdown 渲染 | [marked 18](https://marked.js.org/) |
| 加密库 | [crypto-js 4.2](https://github.com/brix/crypto-js) |
| HTTP 客户端 | [reqwest 0.12](https://github.com/seanmonstar/reqwest)（rustls-tls） |
| 数据库 | [rusqlite 0.31](https://github.com/rusqlite/rusqlite)（SQLite） |
| 包管理 | [pnpm](https://pnpm.io/)（monorepo workspace） |
| 服务端框架 | [Spring Boot 4](https://spring.io/projects/spring-boot)（webmvc + MyBatis-Plus + Druid + Fastjson2） |
| 服务端数据库 | MySQL |
| JSON 序列化 | [Fastjson2](https://github.com/alibaba/fastjson2) |
| 连接池 | [Druid](https://github.com/alibaba/druid) |

---

## 项目结构

```
canghai-api-doc/
├── client/
│   ├── apps/
│   │   ├── web/                        # Vue 3 前端应用
│   │   │   └── src/
│   │   │       ├── components/         # 可复用组件
│   │   │       │   ├── CategoryTree.vue        # 分类树（含导入导出）
│   │   │       │   ├── CategoryNode.vue        # 分类节点（支持拖拽）
│   │   │       │   ├── KeyValueEditor.vue      # 键值编辑器
│   │   │       │   ├── EnvironmentManager.vue  # 环境管理弹窗（支持分组）
│   │   │       │   └── TeamManager.vue          # 团队管理弹窗
│   │   │       ├── composables/        # Vue 3 组合式逻辑
│   │   │       │   ├── useHttpRequest.ts       # HTTP 请求（调用 Tauri）
│   │   │       │   ├── useScriptEngine.ts      # 前后置脚本引擎（含 CryptoJS）
│   │   │       │   ├── useEnvironments.ts      # 环境变量与分组管理
│   │   │       │   ├── useTabs.ts              # 多标签管理（含右键菜单）
│   │   │       │   ├── useHistory.ts           # 历史记录
│   │   │       │   ├── useSavedRequests.ts     # 已保存接口（含导入导出）
│   │   │       │   ├── useCategories.ts        # 分类管理（含拖拽）
│   │   │       │   ├── useSettings.ts          # 全局设置
│   │   │       │   ├── useSync.ts              # 云同步与用户认证
│   │   │       │   ├── useSidebar.ts           # 侧边栏宽度调整
│   │   │       │   └── useTeams.ts              # 团队管理
│   │   │       ├── views/
│   │   │       │   └── ApiDebuggerView.vue     # 主视图
│   │   │       └── types/index.ts              # 全局类型定义
│   │   └── desktop/                    # Tauri 桌面应用
│   │       └── src/                    # Rust 后端
│   │           ├── commands/           # Tauri 命令模块
│   │           │   ├── http.rs         # HTTP 请求代理
│   │           │   ├── category.rs     # 分类 CRUD
│   │           │   ├── saved_request.rs # 已保存接口 CRUD
│   │           │   ├── history.rs      # 历史记录 CRUD
│   │           │   ├── environment.rs  # 环境变量与分组 CRUD
│   │           │   ├── sync.rs         # 数据同步
│   │           │   └── workspace.rs    # 工作区 CRUD
│   │           ├── db.rs               # SQLite 数据库初始化
│   │           ├── models.rs           # 数据模型定义
│   │           ├── lib.rs              # 库入口
│   │           └── main.rs             # 应用入口
│   ├── package.json
│   └── pnpm-workspace.yaml

```

---

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/) 8+
- [Rust](https://www.rust-lang.org/) 1.77+（Tauri 桌面端必需）
- [Tauri 系统依赖](https://tauri.app/start/prerequisites/)（因平台而异，详见官方文档）

### 安装依赖

```bash
cd client
pnpm install
```

### 开发模式

```bash
# 仅启动 Web 前端（浏览器预览，存在 CORS 限制）
pnpm web:dev

# 启动完整桌面应用（推荐，无 CORS 限制）
pnpm dev
```

### 生产构建

```bash
pnpm build
```

构建产物位于 `client/apps/desktop/target/release/bundle/`。

### 启动服务端（可选）

服务端用于云同步与团队协作，非必需。

**准备数据库**：Flyway 会在启动时自动建表与迁移，但不负责创建数据库，需先手工创建：

```sql
CREATE DATABASE IF NOT EXISTS canghai_api
  DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
```

```bash
cd spring-boot-server
mvn package -DskipTests
java -jar target/spring-boot-server.jar
```

服务端默认运行在 `http://localhost:8092`：

- Swagger UI：`http://localhost:8092/swagger-ui.html`
- OpenAPI 文档：`http://localhost:8092/v3/api-docs`
- 健康检查：`http://localhost:8092/actuator/health`

> 表结构的唯一事实源是 `client-spring-boot-server/src/main/resources/db/migration/V1__init_schema.sql`（Flyway 版本化迁移）：
> 空库首次启动自动建表，已有库自动 baseline（version 1）后按版本号增量迁移，请勿修改已提交的迁移脚本。
> 数据库连接通过环境变量注入：`DB_URL` / `DB_USERNAME` / `DB_PASSWORD`。

---

## 使用指南

### 发送第一个请求

1. 在 URL 栏输入接口地址，选择请求方法
2. 在 **Params** / **Headers** / **Body** 标签页填写参数
3. 点击 **发送** 按钮，响应内容展示在下方响应区
4. 使用 `Ctrl+S` 快速保存当前请求

### 使用环境变量

1. 点击右上角环境选择器旁的编辑图标，打开环境管理器
2. 可先创建环境分组（如"项目A"、"项目B"），再在分组下新建环境
3. 新建环境并添加变量（如 `baseUrl = https://api.example.com`）
4. 在 URL 中使用 `{{baseUrl}}/users` 引用变量
5. 切换环境时，所有引用自动替换为对应值

### 编写脚本

**前置脚本**——自动注入 Token 并计算签名：
```javascript
const token = env.get('token')
req.headers['Authorization'] = 'Bearer ' + token

// 使用 CryptoJS 计算请求签名
const sign = CryptoJS.MD5(req.body + env.get('secret')).toString()
req.headers['X-Sign'] = sign

// 设置时间戳变量，可在当前请求中通过 {{timestamp}} 引用
env.set('timestamp', Date.now().toString())
req.params['ts'] = '{{timestamp}}'
```

**后置脚本**——提取响应并写入变量：
```javascript
const data = res.json()
if (data.token) {
  env.set('token', data.token)
  console.log('Token 已更新')
}
test('请求成功', () => assert(res.status === 200))
```

### 使用 Mock 响应（设计模式）

1. 切换至 **设计** 模式
2. 在 Markdown 编辑器中编写接口文档
3. 点击 **+ 添加 Mock 场景**，配置状态码、响应头和响应体
4. 点击 **模拟发送** 即可在无后端情况下预览响应结果

### 管理分类树

- **拖拽移动**：拖拽分类节点到目标分类上，即可移动分类位置
- **右键菜单**：点击分类节点右侧的 `···` 按钮，可添加接口、添加子分类、编辑或删除

### 导入 / 导出接口

- 点击分类树标题旁的 **更多操作**（`···`）按钮，选择 **导出接口** 即可将所有接口导出为 JSON 文件
- 选择 **导入接口** 可从 JSON 文件批量导入接口，同名接口自动跳过

### 工作区与同步

- 点击顶部工具栏的工作区按钮，打开工作区管理器，可创建、切换多个工作区
- 登录账号后，点击 **同步** 按钮，可将当前工作区数据推送至服务端或从服务端拉取

---

## 参与贡献

1. Fork 本仓库
2. 新建 `feat/your-feature` 分支
3. 提交代码并确保 `pnpm build` 通过
4. 发起 Pull Request

---

---
## 致谢开源框架

感谢以下开源项目为本工具提供核心支撑：

- [Tauri](https://tauri.app/) — Rust 驱动的跨平台桌面框架
- [Vue 3](https://vuejs.org/) — 渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) — Vue 3 组件库
- [Vite](https://vitejs.dev/) — 下一代前端构建工具
- [marked](https://marked.js.org/) — Markdown 解析器
- [crypto-js](https://github.com/brix/crypto-js) — JavaScript 加密算法库
- [reqwest](https://github.com/seanmonstar/reqwest) — Rust HTTP 客户端
- [rusqlite](https://github.com/rusqlite/rusqlite) — Rust SQLite 绑定
- [Spring Boot](https://spring.io/projects/spring-boot) — 后端服务框架（webmvc + MyBatis-Plus + Druid + Fastjson2）
- [Undertow](https://undertow.io/) — 高性能 Java Web 服务器
- [Fastjson2](https://github.com/alibaba/fastjson2) — 高性能 JSON 库
- [Druid](https://github.com/alibaba/druid) — 数据库连接池

---
## License

[MIT](LICENSE)
