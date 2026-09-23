#!/usr/bin/env node
/**
 * 前端依赖图校验（Phase 8.3）。
 *
 * 目标：让「循环依赖」可被机器发现 —— 原验收标准写的是 `madge --circular` 输出为空，
 * 但引入 madge 需要额外依赖；本脚本零依赖实现同等能力，可直接进 CI。
 *
 * 校验内容：
 *   1. `client/apps/web/src` 内部模块的 import 图**无环**（DFS 三色标记）；
 *   2. 分层规则：`repositories/**` 不得 import `composables/**`
 *      （反向依赖会让 repository 无法脱离 Vue 组件上下文单测）。
 *
 * 用法：
 *   node scripts/dep-graph-check.mjs            # 严格模式，发现环即失败（CI 用）
 *   node scripts/dep-graph-check.mjs --report   # 仅报告，始终退出 0
 */
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve, relative, extname, sep } from 'node:path'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const SRC = resolve(root, 'client/apps/web/src')
const REPORT_ONLY = process.argv.includes('--report')

const posix = (p) => p.split(sep).join('/')

/** 递归收集源码文件 */
function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = resolve(dir, name)
    const st = statSync(full)
    if (st.isDirectory()) walk(full, out)
    else if (['.ts', '.tsx', '.vue', '.js', '.mjs'].includes(extname(name))) out.push(full)
  }
  return out
}

const files = walk(SRC)

/** 解析 import 说明符（静态 import / export from / 动态 import()） */
function extractSpecifiers(source) {
  // 先剥离注释：JSDoc 里常写 `import { x } from '@/...'` 的用法示例，
  // 不剥离会把注释当成真实依赖，制造自引用等假环。
  const code = source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '')
  const specs = new Set()
  const patterns = [
    /\bfrom\s+['"]([^'"]+)['"]/g,
    /\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
    /\bimport\s+['"]([^'"]+)['"]/g,
  ]
  for (const re of patterns) {
    let m
    while ((m = re.exec(code))) specs.add(m[1])
  }
  return [...specs]
}

const CANDIDATE_EXT = ['.ts', '.tsx', '.vue', '.d.ts', '/index.ts', '/index.vue']

/** 把 import 说明符解析为 src 内的真实文件；非内部模块返回 null */
function resolveSpec(fromFile, spec) {
  let base
  if (spec.startsWith('@/')) base = resolve(SRC, spec.slice(2))
  else if (spec.startsWith('.')) base = resolve(dirname(fromFile), spec)
  else return null // 裸模块（vue / pinia / @tauri-apps/...）不参与图
  if (extname(base) && CANDIDATE_EXT.includes(extname(base))) {
    try {
      if (statSync(base).isFile()) return base
    } catch {
      /* 继续尝试补扩展名 */
    }
  }
  for (const ext of CANDIDATE_EXT) {
    const cand = base + ext
    try {
      if (statSync(cand).isFile()) return cand
    } catch {
      /* 下一个候选 */
    }
  }
  return null
}

const graph = new Map()
const layerViolations = []

for (const file of files) {
  const code = readFileSync(file, 'utf8')
  const deps = new Set()
  for (const spec of extractSpecifiers(code)) {
    const target = resolveSpec(file, spec)
    if (!target) continue
    deps.add(target)
    const fromRepo = posix(relative(SRC, file)).startsWith('repositories/')
    const toComposable = posix(relative(SRC, target)).startsWith('composables/')
    if (fromRepo && toComposable) {
      layerViolations.push(
        `${posix(relative(SRC, file))} → ${posix(relative(SRC, target))}（repository 反向依赖 composable）`,
      )
    }
  }
  graph.set(file, [...deps])
}

// ---- 环检测：DFS 三色（白=未访问，灰=在栈上，黑=已完成）----
const WHITE = 0
const GRAY = 1
const BLACK = 2
const color = new Map()
const cycles = []

function dfs(node, stack) {
  color.set(node, GRAY)
  stack.push(node)
  for (const next of graph.get(node) ?? []) {
    const c = color.get(next) ?? WHITE
    if (c === GRAY) {
      const start = stack.indexOf(next)
      cycles.push([...stack.slice(start), next].map((f) => posix(relative(SRC, f))))
    } else if (c === WHITE) {
      dfs(next, stack)
    }
  }
  stack.pop()
  color.set(node, BLACK)
}

for (const file of files) if ((color.get(file) ?? WHITE) === WHITE) dfs(file, [])

// ---- 输出 ----
const edgeCount = [...graph.values()].reduce((n, d) => n + d.length, 0)
console.log('=== 前端依赖图校验（Phase 8.3） ===\n')
console.log(`模块 ${files.length} 个，内部依赖边 ${edgeCount} 条\n`)

if (layerViolations.length) {
  console.log('分层违规：')
  for (const v of layerViolations) console.log(`  ✘ ${v}`)
} else {
  console.log('  ✔ 分层规则通过：repositories 未反向依赖 composables')
}

if (cycles.length) {
  console.log('\n循环依赖：')
  for (const c of cycles) console.log(`  ✘ ${c.join(' → ')}`)
} else {
  console.log('  ✔ 无循环依赖（DFS 三色标记结果为空）')
}

const failed = cycles.length + layerViolations.length
console.log(failed ? `\n结论：不通过（${failed} 项）` : '\n结论：通过')
process.exit(REPORT_ONLY || failed === 0 ? 0 : 1)
