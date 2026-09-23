#!/usr/bin/env node
/**
 * 三端契约一致性校验（Phase 6.1）。
 *
 * 目标：让「Java DTO / Rust struct / TS type」的字段漂移可被机器发现，而不是靠人肉 review。
 * 校验内容：
 *   1. 实体字段集比对：TS 接口 vs Rust struct（Rust 侧按 serde rename_all=camelCase 归一）。
 *      语义：**TS 字段必须是 Rust 的子集** —— TS 不得「发明」Rust 不存在的字段（那必然读到 undefined）；
 *      Rust 独有字段（deleted / currentUserRole 等服务端内部列）允许 TS 不声明，仅作提示。
 *   2. 响应信封：TS `ApiResult<T>` 必须为 { success, code, msg, data }。
 *   3. 同步上传键名契约：Java `SyncData` 必须用 @JsonAlias("requests") 兼容客户端键名。
 *   4. 错误码表：Java `ErrorCode` 枚举与 docs/API_CONTRACT.md 表格保持一致。
 *   5. MyBatis-Plus 实体：含 `typeHandler` 字段时必须开启 `autoResultMap = true`，
 *      否则 typeHandler 只在写入生效、查询回读恒为 null（见 B3）。
 *   6. JSON 列写路径：`UpdateWrapper.set("json列", ...)` 必须显式序列化为字符串
 *      （`jsonOrNull(...)` 等），直接传 List/POJO 会因无法绑定参数而抛异常（见 B4）。
 *
 * 用法：
 *   node scripts/contract-check.mjs            # 严格模式，任何未登记差异即失败（CI 用）
 *   node scripts/contract-check.mjs --report   # 仅报告，始终退出 0
 *
 * `KNOWN_TS_ONLY` 为「已识别但尚未修复」的 TS 侧多余字段白名单（Phase 6 遗留）。
 * 修复后必须从白名单移除，使 CI 恢复严格拦截；白名单只影响退出码，不影响报告输出。
 */
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const REPORT_ONLY = process.argv.includes('--report')

const TS_TYPES = 'client/apps/web/src/types/index.ts'
const RUST_DB_DIR = 'client/apps/desktop/src/db'
const JAVA_SYNC_DATA = 'spring-boot-server/src/main/java/com/canghai/api/model/SyncData.java'
const JAVA_ERROR_CODE = 'spring-boot-server/src/main/java/com/canghai/api/common/ErrorCode.java'
const CONTRACT_DOC = 'docs/API_CONTRACT.md'
const JAVA_ENTITY_DIR = 'spring-boot-server/src/main/java/com/canghai/api/entity'
const JAVA_SERVICE_DIR = 'spring-boot-server/src/main/java/com/canghai/api/service'

/** 参与比对的实体：TS 接口名（与 Rust struct 同名） */
const ENTITIES = [
  'Project',
  'Category',
  'SavedRequest',
  'Environment',
  'EnvironmentGroup',
  'EnvironmentVariable',
  'HistoryItem',
]

/**
 * 已识别、待修复的 TS 侧多余字段白名单。
 *
 * **当前为空**：Phase 6.2 #5「排序字段命名不统一」已修复 ——
 * `Project.order` / `Category.order` 统一为 `sortOrder`（与 Rust `sort_order` 同形），
 * `Environment.sortOrder` 经核查为从未落库、也从未被 UI 读取的死字段，已删除。
 * 后续若确有「已识别但暂不能修」的差异，在此登记。
 */
const KNOWN_TS_ONLY = {}

const read = (rel) => readFileSync(resolve(root, rel), 'utf8')

const toCamel = (snake) => snake.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase())

/** 解析 TS `export interface X<T> { ... }`，返回 { name: string[] } */
function parseTsInterfaces(src) {
  const out = {}
  const re = /export interface (\w+)(?:<[^>{]*>)?\s*\{([\s\S]*?)\n\}/g
  let m
  while ((m = re.exec(src))) {
    const [, name, body] = m
    const fields = []
    for (const line of body.split('\n')) {
      const fm = line.match(/^\s*(?:readonly\s+)?([A-Za-z_$][\w$]*)\??\s*:/)
      if (fm) fields.push(fm[1])
    }
    out[name] = fields
  }
  return out
}

/** 解析 Rust `pub struct X { ... }`，字段按 camelCase 归一 */
function parseRustStructs(src) {
  const out = {}
  const re = /pub struct (\w+)\s*\{([\s\S]*?)\n\}/g
  let m
  while ((m = re.exec(src))) {
    const [, name, body] = m
    const fields = []
    for (const line of body.split('\n')) {
      const fm = line.match(/^\s*pub (\w+)\s*:/)
      if (fm) fields.push(toCamel(fm[1]))
    }
    out[name] = fields
  }
  return out
}

const sameSet = (a, b) => a.length === b.length && a.every((x) => new Set(b).has(x))

const problems = []
const known = []
const notes = []

/** 递归收集 Rust 数据层源码并拼接：Phase 8.1 拆分后实体分散在 db/*.rs 中，不能再只读单文件。 */
function readRustDbSource() {
  const base = resolve(root, RUST_DB_DIR)
  if (!existsSync(base)) {
    problems.push(`Rust 数据层目录缺失：${RUST_DB_DIR}（拆分后路径应为 db/,不再是 db.rs）`)
    return ''
  }
  const walk = (dir) =>
    readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
      const full = resolve(dir, e.name)
      if (e.isDirectory()) return walk(full)
      return e.name.endsWith('.rs') ? [full] : []
    })
  // 排序保证拼接顺序稳定（错误信息可复现）
  return walk(base)
    .sort()
    .map((f) => readFileSync(f, 'utf8'))
    .join('\n')
}

const tsInterfaces = parseTsInterfaces(read(TS_TYPES))
const rustStructs = parseRustStructs(readRustDbSource())

// ---- 1. 实体字段集比对（TS ⊆ Rust）----
for (const name of ENTITIES) {
  const ts = tsInterfaces[name]
  const rs = rustStructs[name]
  if (!ts) {
    problems.push(`[${name}] TS 接口缺失（${TS_TYPES}）`)
    continue
  }
  if (!rs) {
    problems.push(`[${name}] Rust struct 缺失（${RUST_DB_DIR}）`)
    continue
  }
  const rsSet = new Set(rs)
  const tsOnly = ts.filter((f) => !rsSet.has(f))
  const tsSet = new Set(ts)
  const rustOnly = rs.filter((f) => !tsSet.has(f))

  if (tsOnly.length) {
    const wl = KNOWN_TS_ONLY[name]
    if (wl && sameSet(tsOnly, wl)) {
      known.push(`[${name}] TS 多余字段 [${tsOnly.join(', ')}]`)
    } else {
      problems.push(`[${name}] TS 声明了 Rust 不存在的字段 [${tsOnly.join(', ')}]（运行时必为 undefined）`)
    }
  } else {
    notes.push(`[${name}] TS 字段全部命中 Rust（${ts.length} 项；Rust 独有 ${rustOnly.length} 项已忽略）`)
  }
}

// ---- 2. 响应信封 ----
const envelope = tsInterfaces.ApiResult || []
for (const f of ['success', 'code', 'msg', 'data']) {
  if (!envelope.includes(f)) problems.push(`[ApiResult] 信封缺少字段 ${f}`)
}

// ---- 3. 同步上传键名契约 ----
if (!/@JsonAlias\(\s*"requests"\s*\)/.test(read(JAVA_SYNC_DATA))) {
  problems.push('[SyncData] 缺少 @JsonAlias("requests")：客户端上报键名 requests 将被丢弃')
}

// ---- 4. 错误码表一致性 ----
const javaEnumCodes = [...read(JAVA_ERROR_CODE).matchAll(/^\s*([A-Z_]+)\((\d+),\s*"([^"]*)"\)/gm)].map(
  (m) => ({ name: m[1], code: Number(m[2]) }),
)
const doc = read(CONTRACT_DOC)
for (const { name, code } of javaEnumCodes) {
  if (!new RegExp(`\\|\\s*${code}\\s*\\|\\s*${name}\\s*\\|`).test(doc)) {
    problems.push(`[ErrorCode] ${name}(${code}) 未在 ${CONTRACT_DOC} 错误码表中体现`)
  }
}

// ---- 5. MyBatis-Plus 实体：typeHandler 必须配 autoResultMap ----
const snake = (s) => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)

/** 解析实体：返回 { table, autoResultMap, jsonColumns:Set<column> } */
function parseEntity(src) {
  const tableMatch = src.match(/@TableName\(([\s\S]*?)\)\s*\n/)
  const table = tableMatch
    ? (tableMatch[1].match(/"([^"]+)"/) || [, ''])[1]
    : ''
  const autoResultMap = /autoResultMap\s*=\s*true/.test(tableMatch ? tableMatch[1] : '')
  const jsonColumns = new Set()
  // 先剥离注释：Javadoc 里也常出现 @TableField(typeHandler = ...) 的说明文字
  const code = src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '')
  // 逐字段扫描：记录前一条 @TableField 注解，再绑定到其后的字段声明
  const lines = code.split('\n')
  let pendingField = null
  for (const line of lines) {
    const ann = line.match(/@TableField\(([^)]*)\)/)
    if (ann) pendingField = ann[1]
    const decl = line.match(/private\s+[\w.<>\[\],\s]+\s+(\w+)\s*;/)
    if (decl) {
      if (pendingField && /typeHandler\s*=/.test(pendingField)) {
        const col = (pendingField.match(/"([^"]+)"/) || [, snake(decl[1])])[1]
        jsonColumns.add(col)
      }
      pendingField = null
    }
  }
  return { table, autoResultMap, jsonColumns }
}

const entityFiles = readdirSync(resolve(root, JAVA_ENTITY_DIR)).filter((f) => f.endsWith('.java'))
const jsonColumnsByCol = new Map()
let entityCount = 0
for (const file of entityFiles) {
  const src = read(`${JAVA_ENTITY_DIR}/${file}`)
  if (!/@TableName\(/.test(src)) continue
  entityCount += 1
  const { table, autoResultMap, jsonColumns } = parseEntity(src)
  const hasTypeHandler = jsonColumns.size > 0
  if (hasTypeHandler && !autoResultMap) {
    problems.push(
      `[${file.replace('.java', '')}] 声明了 typeHandler 但 @TableName 未开 autoResultMap：查询结果不走 typeHandler（${
        table || '未知表'
      }）`,
    )
  }
  for (const col of jsonColumns) jsonColumnsByCol.set(col, file.replace('.java', ''))
}

// ---- 6. JSON 列写路径必须显式序列化 ----
/** JSON 列允许的 .set 取值形态：字符串字面量 / null / JSON 序列化助手调用 */
const isSerializedValue = (v) =>
  /^"[\s\S]*"\s*\)*\s*$/.test(v) || /^null\s*\)*\s*$/.test(v) || /\bjson\w*\s*\(/.test(v)

let setCallCount = 0
for (const file of readdirSync(resolve(root, JAVA_SERVICE_DIR)).filter((f) => f.endsWith('.java'))) {
  const lines = read(`${JAVA_SERVICE_DIR}/${file}`).split('\n')
  lines.forEach((line, i) => {
    const m = line.match(/\.set\(\s*"([a-z0-9_]+)"\s*,\s*(.+)$/)
    if (!m) return
    const [, col, value] = m
    if (!jsonColumnsByCol.has(col)) return
    setCallCount += 1
    if (!isSerializedValue(value.trim())) {
      problems.push(
        `[${file}:${i + 1}] JSON 列 ${col} 的 .set 值未显式序列化（${value.slice(0, 60)}）—— ` +
          `UpdateWrapper.set 不应用 typeHandler，传 List/POJO 会绑定失败（见 B4）`,
      )
    }
  })
}

// ---- 输出 ----
console.log('=== 契约一致性校验（Phase 6.1） ===\n')
console.log(`实体比对：${ENTITIES.length} 个，错误码：${javaEnumCodes.length} 条`)
console.log(
  `静态检查：MyBatis-Plus 实体 ${entityCount} 个（JSON 列 ${jsonColumnsByCol.size} 个：` +
    `${[...jsonColumnsByCol.keys()].join(', ')}），JSON 列 set 调用 ${setCallCount} 处\n`,
)
for (const n of notes) console.log(`  ✔ ${n}`)
if (known.length) {
  console.log('\n已登记差异（白名单，待修复）：')
  for (const k of known) console.log(`  ⚠ ${k}`)
}
if (problems.length) {
  console.log('\n发现问题：')
  for (const p of problems) console.log(`  ✘ ${p}`)
  console.log(`\n结论：不通过（${problems.length} 项）`)
} else {
  console.log('\n结论：通过（无未登记差异）')
}

process.exit(REPORT_ONLY || problems.length === 0 ? 0 : 1)
