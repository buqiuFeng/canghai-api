<template>
  <div class="json-editor">
    <!-- 高亮层：纯展示，不接收指针事件，由 <pre> 原生保留换行与缩进。
         直接 v-html 到 pre 上（不再套 code：其 UA 等宽字体会覆盖 pre 的字体，导致与输入层错位） -->
    <pre ref="hlRef" class="json-editor__hl" aria-hidden="true" v-html="highlighted" />
    <textarea
      ref="taRef"
      class="json-editor__ta"
      :value="modelValue"
      :placeholder="placeholder"
      rows="8"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      @input="onInput"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { highlightJson } from '@/utils/jsonHighlight'

const props = defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{ (e: 'update:modelValue', value: string): void }>()

const taRef = ref<HTMLTextAreaElement | null>(null)
const hlRef = ref<HTMLPreElement | null>(null)

/**
 * 高亮 HTML 必须由 highlightJson 生成：它内部已做 HTML 转义，
 * 直接 v-html 用户原文会带来 XSS 风险（WebView 内可升级为 RCE）。
 */
const highlighted = computed(() => highlightJson(props.modelValue))

/**
 * 两层（高亮层 / 输入层）必须逐字符对齐，靠三点保证：
 * 1. 两层的宽度、内边距、字体、行高、换行策略在样式里完全一致；
 * 2. 只有输入层参与布局（高亮层绝对定位覆盖其上），因此不会出现
 *    「textarea 内滚动条占宽 → 两层换行位置错开」的经典错位；
 * 3. 高度取两层中较大者，避免高亮层多出一行时被截断。
 */
function syncHeight() {
  const ta = taRef.value
  if (!ta) return
  ta.style.height = 'auto'
  const taHeight = ta.scrollHeight
  const hlHeight = hlRef.value?.scrollHeight ?? 0
  ta.style.height = `${Math.max(taHeight, hlHeight)}px`
}

function onInput(e: Event) {
  const el = e.target as HTMLTextAreaElement
  emit('update:modelValue', el.value)
  syncHeight()
}

watch(() => props.modelValue, () => nextTick(syncHeight))
onMounted(() => nextTick(syncHeight))
</script>

<style scoped>
.json-editor {
  position: relative;
  width: 100%;
  min-height: 180px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  background: var(--surface);
  transition: border-color var(--duration-fast) var(--ease-out),
    box-shadow var(--duration-fast) var(--ease-out);
}
.json-editor:hover {
  border-color: var(--text-4);
}
.json-editor:focus-within {
  border-color: var(--brand-1);
  box-shadow: 0 0 0 1px var(--brand-1);
}

/* 两层共用同一套排版参数：任何一处不一致都会让高亮文字与真实光标错位 */
.json-editor__hl,
.json-editor__ta {
  display: block;
  width: 100%;
  margin: 0;
  padding: 10px 12px;
  border: 0;
  box-sizing: border-box;
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-md);
  line-height: 1.6;
  letter-spacing: normal;
  tab-size: 2;
  white-space: pre-wrap;
  overflow-wrap: break-word;
  word-break: break-word;
}

.json-editor__hl {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  color: var(--text-2);
  pointer-events: none;
}

.json-editor__ta {
  position: relative;
  overflow: hidden;
  resize: none;
  outline: none;
  background: transparent;
  /* 文字由高亮层绘制，输入层只保留光标与选区 */
  color: transparent;
  caret-color: var(--text-1);
}
.json-editor__ta::placeholder {
  color: var(--text-4);
}

/* v-html 生成的内容不带 scoped 属性，需用 :deep 命中 */
.json-editor__hl :deep(.tok-key) { color: var(--json-key); }
.json-editor__hl :deep(.tok-str) { color: var(--json-str); }
.json-editor__hl :deep(.tok-num) { color: var(--json-num); }
.json-editor__hl :deep(.tok-bool) { color: var(--json-bool); }
.json-editor__hl :deep(.tok-var) { color: var(--json-var); }
.json-editor__hl :deep(.tok-comment) {
  color: var(--json-comment);
  font-style: italic;
}
.json-editor__hl :deep(.tok-punct) { color: var(--json-punct); }
</style>
