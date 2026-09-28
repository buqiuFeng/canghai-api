/**
 * JSON 语法高亮（供请求体编辑器的高亮层使用）。
 *
 * 这里刻意用「宽松分词」而不是 `JSON.parse` 校验结构：请求体允许 `//`、`/* *​/` 注释，
 * 以及 `{{变量}}` 占位符（发送前由环境变量替换，见 ApiDebuggerView 的 localResolve）。
 * 严格解析会把这类内容判为非法，导致编辑过程中大片内容无法着色；分词只保证
 * 「字符串 / 注释整体不被误切」，因此半成品 JSON 也能正常高亮。
 */

/** HTML 转义：高亮结果会交给 v-html，必须先把用户输入的 < > & 转义掉 */
function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

/**
 * 把 JSON 文本转成带 `<span class="tok-*">` 的 HTML。
 *
 * 备选分支的顺序即优先级：注释 / 字符串必须排在关键字、数字、标点之前，
 * 否则字符串内部的 `true`、数字、`//` 会被当成普通 token 切开。
 */
export function highlightJson(src: string): string {
  const tokenRe =
    /(\/\/[^\n]*|\/\*[\s\S]*?\*\/)|("(?:\\.|[^"\\])*")|(\{\{[^{}\n]*\}\})|(\btrue\b|\bfalse\b|\bnull\b)|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|([{}[\],:])/g

  let out = ''
  let last = 0
  let m: RegExpExecArray | null
  while ((m = tokenRe.exec(src)) !== null) {
    out += escapeHtml(src.slice(last, m.index))
    const [raw, comment, str, variable, bool, num] = m
    if (comment) {
      out += `<span class="tok-comment">${escapeHtml(raw)}</span>`
    } else if (str) {
      // 紧随可选空白后是冒号的字符串即为键名，与字符串值分色（贴近 JSON 查看器习惯）
      const isKey = /^\s*:/.test(src.slice(m.index + raw.length))
      out += `<span class="${isKey ? 'tok-key' : 'tok-str'}">${escapeHtml(raw)}</span>`
    } else if (variable) {
      out += `<span class="tok-var">${escapeHtml(raw)}</span>`
    } else if (bool) {
      out += `<span class="tok-bool">${escapeHtml(raw)}</span>`
    } else if (num) {
      out += `<span class="tok-num">${escapeHtml(raw)}</span>`
    } else {
      out += `<span class="tok-punct">${escapeHtml(raw)}</span>`
    }
    last = m.index + raw.length
  }
  out += escapeHtml(src.slice(last))
  return out
}
