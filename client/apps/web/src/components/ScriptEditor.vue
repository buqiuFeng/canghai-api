<template>
  <div class="script-editor" :data-script-editor="name || undefined">
    <div ref="hostRef" class="script-editor__host" />
    <!-- API 速查：按分类列出可用 API，点击插入到脚本光标处 -->
    <div v-if="apiRef" class="script-editor__ref">
      <button type="button" class="script-editor__ref-toggle" @click="refOpen = !refOpen">
        <span class="ref-caret">{{ refOpen ? '▾' : '▸' }}</span>
        API 速查
        <span class="ref-hint">（点击条目插入到光标处）</span>
      </button>
      <div v-if="refOpen" class="script-editor__ref-body">
        <div v-for="g in grouped" :key="g.group" class="ref-group">
          <div class="ref-group-title">{{ g.group }}</div>
          <button
            v-for="item in g.items"
            :key="item.insert"
            type="button"
            class="ref-item"
            :title="item.doc || item.detail"
            @mousedown.prevent="insertAtCursor(item.insert)"
          >
            <code class="ref-code">{{ item.insert }}</code>
            <span class="ref-detail">{{ item.detail }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { EditorState } from '@codemirror/state'
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  drawSelection,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  placeholder as cmPlaceholder,
} from '@codemirror/view'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle } from '@codemirror/language'
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
  acceptCompletion,
} from '@codemirror/autocomplete'
import { javascript } from '@codemirror/lang-javascript'
import { tags as t } from '@lezer/highlight'
import { SCRIPT_API, SCRIPT_API_GROUPS, scriptCompletions } from '@/utils/scriptApi'

const props = defineProps<{
  modelValue: string
  placeholder?: string
  /** 是否展示「API 速查」面板 */
  apiRef?: boolean
  /** 标识（写入 data-script-editor，供父组件识别插入目标） */
  name?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: string): void
  (e: 'focus'): void
}>()

const hostRef = ref<HTMLElement | null>(null)
const refOpen = ref(false)
let view: EditorView | null = null

const grouped = computed(() =>
  SCRIPT_API_GROUPS.map(group => ({ group, items: SCRIPT_API.filter(e => e.group === group) })),
)

/** 深色主题：与原先 script-textarea 的代码块观感保持一致 */
const theme = EditorView.theme(
  {
    '&': {
      height: '260px',
      fontSize: 'var(--fs-sm)',
      backgroundColor: 'var(--code-bg, #0f172a)',
      color: '#cbd5e1',
      borderRadius: '10px',
      border: '1px solid rgba(99,102,241,.12)',
    },
    '&.cm-focused': {
      outline: 'none',
      borderColor: 'var(--brand-1, #6366f1)',
      boxShadow: '0 0 0 1px var(--brand-1, #6366f1)',
    },
    '.cm-scroller': {
      fontFamily: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      lineHeight: '1.7',
      overflow: 'auto',
    },
    '.cm-content': { padding: '10px 0', caretColor: '#e2e8f0' },
    '.cm-gutters': {
      backgroundColor: 'transparent',
      color: 'rgba(148,163,184,.55)',
      border: 'none',
    },
    '.cm-activeLine': { backgroundColor: 'rgba(148,163,184,.07)' },
    '.cm-activeLineGutter': { backgroundColor: 'rgba(148,163,184,.07)' },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': {
      backgroundColor: 'rgba(99,102,241,.35)',
    },
    '.cm-tooltip': {
      border: '1px solid var(--border, #334155)',
      backgroundColor: 'var(--surface, #1e293b)',
      color: '#cbd5e1',
    },
    '.cm-tooltip-autocomplete > ul > li[aria-selected]': {
      backgroundColor: 'rgba(99,102,241,.35)',
      color: '#fff',
    },
    '.cm-tooltip-autocomplete > ul > li': { padding: '2px 8px' },
    '.cm-completionDetail': { color: 'rgba(148,163,184,.85)', fontStyle: 'normal', marginLeft: '8px' },
  },
  { dark: true },
)

const highlight = HighlightStyle.define([
  { tag: t.comment, color: '#6b7f99', fontStyle: 'italic' },
  { tag: [t.string, t.special(t.string)], color: '#a5d6a7' },
  { tag: [t.number, t.bool, t.null], color: '#f0b37e' },
  { tag: [t.keyword, t.operatorKeyword], color: '#c792ea' },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: '#82aaff' },
  { tag: t.propertyName, color: '#89ddff' },
  { tag: [t.variableName, t.name], color: '#cbd5e1' },
  { tag: t.operator, color: '#89ddff' },
  { tag: t.punctuation, color: '#93a4b8' },
  { tag: t.definition(t.variableName), color: '#ffcb6b' },
])

function buildState(doc: string): EditorState {
  return EditorState.create({
    doc,
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      drawSelection(),
      dropCursor(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      bracketMatching(),
      closeBrackets(),
      autocompletion({ override: [scriptCompletions], activateOnTyping: true }),
      rectangularSelection(),
      crosshairCursor(),
      highlightActiveLine(),
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...historyKeymap,
        ...completionKeymap,
        { key: 'Tab', run: acceptCompletion },
      ]),
      javascript(),
      cmPlaceholder(props.placeholder ?? ''),
      theme,
      syntaxHighlighting(highlight),
      EditorView.lineWrapping,
      EditorView.updateListener.of(u => {
        if (u.docChanged) emit('update:modelValue', u.state.doc.toString())
      }),
      EditorView.domEventHandlers({
        focusin: () => emit('focus'),
      }),
    ],
  })
}

onMounted(() => {
  if (!hostRef.value) return
  view = new EditorView({ state: buildState(props.modelValue ?? ''), parent: hostRef.value })
})

watch(
  () => props.modelValue,
  v => {
    if (!view) return
    const cur = view.state.doc.toString()
    const next = v ?? ''
    if (next !== cur) {
      view.dispatch({ changes: { from: 0, to: cur.length, insert: next } })
    }
  },
)

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

/** 在脚本光标处插入文本；返回是否成功（编辑器未就绪时返回 false） */
function insertAtCursor(text: string): boolean {
  if (!view) return false
  const { from, to } = view.state.selection.main
  view.dispatch({
    changes: { from, to, insert: text },
    selection: { anchor: from + text.length },
  })
  view.focus()
  return true
}

defineExpose({ insertAtCursor })
</script>

<style scoped>
.script-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.script-editor__host {
  width: 100%;
}
.script-editor__ref {
  border: 1px solid var(--border, #334155);
  border-radius: 10px;
  overflow: hidden;
  background: var(--surface, transparent);
}
.script-editor__ref-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 6px 10px;
  border: none;
  background: transparent;
  color: var(--text-2, #334155);
  font-size: var(--fs-sm);
  font-weight: 500;
  text-align: left;
  cursor: pointer;
}
.script-editor__ref-toggle:hover {
  background: rgba(99, 102, 241, 0.06);
}
.ref-caret {
  color: var(--text-4, #94a3b8);
}
.ref-hint {
  color: var(--text-4, #94a3b8);
  font-weight: 400;
  font-size: var(--fs-xs);
}
.script-editor__ref-body {
  max-height: 260px;
  overflow-y: auto;
  padding: 6px 8px 10px;
  border-top: 1px solid var(--border, #334155);
}
.ref-group {
  margin-bottom: 6px;
}
.ref-group-title {
  font-size: var(--fs-xs);
  font-weight: 600;
  color: var(--text-4, #94a3b8);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin: 4px 0 2px;
}
.ref-item {
  display: flex;
  align-items: baseline;
  gap: 8px;
  width: 100%;
  padding: 3px 6px;
  border: none;
  border-radius: 6px;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.ref-item:hover {
  background: rgba(99, 102, 241, 0.08);
}
.ref-code {
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-xs);
  color: var(--brand-1, #6366f1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 60%;
}
.ref-detail {
  font-size: var(--fs-xs);
  color: var(--text-3, #64748b);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
