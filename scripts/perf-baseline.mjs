#!/usr/bin/env node
/**
 * 性能基线采集（Phase 6.4）。
 *
 * 背景：Phase 6.4 的埋点代码早已就绪，但「基线报告」一直缺一个可执行的采集口径，
 * 导致 Phase 7 的收益无法对照（验收纪律要求「基线 → 优化后」两组数据）。
 * 本脚本把两端已有的埋点日志聚合成 P50/P95/P99，产出可粘贴进文档的 Markdown 表格。
 *
 * 支持的日志来源（三端埋点已统一为同一套字段名）：
 *   - Java  `REQ_TIMING`：`method=GET uri=/api/v1/... status=200 cost=12ms`
 *   - Rust  HTTP 代理：`HTTP POST <url> status=200 cost=18ms success=true`
 *   - Rust  同步：`run_sync team=x cost=812ms ...` / `run_pull team=x cost=430ms ...`
 *
 * 用法：
 *   node scripts/perf-baseline.mjs <日志文件...>            # 打印 Markdown 表格
 *   node scripts/perf-baseline.mjs <日志文件...> --json     # 输出 JSON（便于后续对比）
 *   node scripts/perf-baseline.mjs <日志文件...> --out=docs/PERF_BASELINE.md
 *
 * 采集方式：
 *   后端  docker compose logs server > server.log
 *   桌面端 运行时开启 tauri 日志输出（`log` 插件），重定向到 desktop.log
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')

const args = process.argv.slice(2)
const files = args.filter((a) => !a.startsWith('--'))
const asJson = args.includes('--json')
const outArg = args.find((a) => a.startsWith('--out='))

if (!files.length) {
  console.error('用法：node scripts/perf-baseline.mjs <日志文件...> [--json] [--out=路径]')
  process.exit(2)
}

/** 埋点解析规则：一条日志行 → { group, cost } */
const PATTERNS = [
  {
    // Java 后端：按 HTTP 方法与 URI 分组
    re: /method=(\w+)\s+uri=(\S+)\s+status=(\d+)\s+cost=(\d+)ms/,
    pick: (m) => ({ group: `server ${m[1]} ${m[2]}(status=${m[3]})`, cost: Number(m[4]) }),
  },
  {
    // Rust 云桥代理：url → 仅取 path，避免 server 地址/端口变化导致分组碎裂
    re: /HTTP POST (\S+)\s+status=(\d+)\s+cost=(\d+)ms/,
    pick: (m) => ({ group: `desktop http POST ${pathOf(m[1])}(status=${m[2]})`, cost: Number(m[3]) }),
  },
  {
    // Rust 云桥代理失败路径（连接/读取失败）
    re: /HTTP POST (\S+).*?cost=(\d+)ms\s+err=/,
    pick: (m) => ({ group: `desktop http POST ${pathOf(m[1])}(error)`, cost: Number(m[2]) }),
  },
  { re: /run_sync team=\S+ cost=(\d+)ms/, pick: (m) => ({ group: 'desktop run_sync', cost: Number(m[1]) }) },
  { re: /run_pull team=\S+ cost=(\d+)ms/, pick: (m) => ({ group: 'desktop run_pull', cost: Number(m[1]) }) },
]

function pathOf(url) {
  try {
    const u = new URL(url)
    return u.pathname
  } catch {
    return url
  }
}

/** 最近秩百分位（nearest-rank）：小样本下比插值法更贴近「第 95 个百分位」的直观含义 */
function percentile(sorted, p) {
  if (!sorted.length) return 0
  const idx = Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)
  return sorted[Math.min(idx, sorted.length - 1)]
}

const buckets = new Map()
let matched = 0
let scanned = 0

for (const file of files) {
  const text = readFileSync(resolve(root, file), 'utf8')
  for (const line of text.split('\n')) {
    scanned += 1
    for (const p of PATTERNS) {
      const m = line.match(p.re)
      if (!m) continue
      const { group, cost } = p.pick(m)
      if (!Number.isFinite(cost)) continue
      if (!buckets.has(group)) buckets.set(group, [])
      buckets.get(group).push(cost)
      matched += 1
      break
    }
  }
}

const rows = [...buckets.entries()]
  .map(([group, costs]) => {
    const sorted = [...costs].sort((a, b) => a - b)
    const sum = sorted.reduce((a, b) => a + b, 0)
    return {
      group,
      count: sorted.length,
      min: sorted[0],
      mean: Math.round(sum / sorted.length),
      p50: percentile(sorted, 50),
      p95: percentile(sorted, 95),
      p99: percentile(sorted, 99),
      max: sorted[sorted.length - 1],
    }
  })
  .sort((a, b) => b.p95 - a.p95)

if (asJson) {
  console.log(JSON.stringify({ scanned, matched, at: new Date().toISOString(), rows }, null, 2))
  process.exit(0)
}

const lines = []
lines.push('### 性能基线（自动生成）')
lines.push('')
lines.push(`- 采样时间：${new Date().toISOString()}`)
lines.push(`- 日志行数：${scanned}（命中埋点 ${matched} 条）`)
lines.push(`- 百分位口径：nearest-rank；单位 ms`)
lines.push('')
if (!rows.length) {
  lines.push('> 未解析到埋点行：请确认后端日志包含 `REQ_TIMING` 的 `method=.. uri=.. status=.. cost=..ms`，')
  lines.push('> 或桌面端已开启 log 插件输出（`HTTP POST ... cost=..ms` / `run_sync ... cost=..ms`）。')
} else {
  lines.push('| 指标 | 次数 | min | P50 | P95 | P99 | max | mean |')
  lines.push('|---|---|---|---|---|---|---|---|')
  for (const r of rows) {
    lines.push(
      `| \`${r.group}\` | ${r.count} | ${r.min} | ${r.p50} | ${r.p95} | ${r.p99} | ${r.max} | ${r.mean} |`,
    )
  }
}

const md = lines.join('\n')
console.log(md)

if (outArg) {
  const target = resolve(root, outArg.slice('--out='.length))
  writeFileSync(target, md + '\n', 'utf8')
  console.error(`\n已写入 ${outArg.slice('--out='.length)}`)
}
