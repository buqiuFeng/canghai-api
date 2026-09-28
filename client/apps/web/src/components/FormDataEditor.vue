<template>
  <div class="formdata-editor">
    <div v-for="(row, idx) in modelValue" :key="idx" class="fd-row">
      <el-checkbox
        :model-value="row.enabled"
        @update:model-value="(v: boolean | string | number) => updateRow(idx, { enabled: Boolean(v) })"
      />
      <el-select
        :model-value="row.type"
        size="small"
        class="fd-type"
        @update:model-value="(v: 'text' | 'file') => switchType(idx, v)"
      >
        <el-option label="文本" value="text" />
        <el-option label="文件" value="file" />
      </el-select>
      <el-input
        :model-value="row.key"
        placeholder="字段名"
        class="fd-key"
        @update:model-value="(v: string) => updateRow(idx, { key: v })"
      />
      <template v-if="row.type !== 'file'">
        <el-input
          :model-value="row.value"
          placeholder="文本值"
          class="fd-val"
          @update:model-value="(v: string) => updateRow(idx, { value: v })"
        />
      </template>
      <template v-else>
        <div class="fd-file" @click="pickFile">
          <el-button size="small" class="fd-file-btn">{{ row.fileName || '选择文件' }}</el-button>
          <input type="file" class="fd-file-input" @change="(e: Event) => onFileChange(idx, e)" />
          <span v-if="row.fileName" class="fd-file-meta" :title="row.fileName">{{ row.fileName }} · {{ formatSize(row.fileSize || 0) }}</span>
          <span v-if="row.fileName" class="fd-file-clear" title="清除文件" @click.stop="clearFile(idx)">✕</span>
        </div>
      </template>
      <el-input
        :model-value="row.description"
        placeholder="描述"
        class="fd-desc"
        @update:model-value="(v: string) => updateRow(idx, { description: v })"
      />
      <el-button
        link
        type="danger"
        size="small"
        :disabled="isOnlyEmptyRow(idx)"
        @click="removeRow(idx)"
      >删除</el-button>
    </div>
    <!-- 显式新增入口：导入的空数组（0 行）也能添加字段 -->
    <div class="fd-add-row">
      <el-button link type="primary" size="small" :icon="Plus" @click="addRow">添加字段</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'
import { formatSize } from '@/utils'
import type { FormDataPart } from '@/types'

const props = defineProps<{
  modelValue: FormDataPart[]
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: FormDataPart[]): void
}>()

function emptyPart(): FormDataPart {
  return { enabled: true, key: '', type: 'text', value: '', description: '' }
}

/** 追加一行空字段。用于「导入后为空数组（0 行）」或需显式新增的场景。 */
function addRow() {
  emit('update:modelValue', [...props.modelValue, emptyPart()])
}

function updateRow(idx: number, patch: Partial<FormDataPart>) {
  const next = props.modelValue.map((it, i) => (i === idx ? { ...it, ...patch } : it))
  const cur = next[idx]
  const hasContent = !!cur.key.trim() || (cur.type === 'file' && !!cur.fileName) || !!cur.description?.trim()
  if (idx === next.length - 1 && hasContent) {
    next.push(emptyPart())
  }
  emit('update:modelValue', next)
}

function switchType(idx: number, type: 'text' | 'file') {
  // 切换类型时清空另一种类型的残留数据，避免文本值与文件信息混存
  const patch: Partial<FormDataPart> = { type }
  if (type === 'file') {
    patch.value = ''
  } else {
    patch.fileData = ''
    patch.fileName = ''
    patch.fileMime = undefined
    patch.fileSize = 0
  }
  updateRow(idx, patch)
}

function removeRow(idx: number) {
  const next = props.modelValue.filter((_, i) => i !== idx)
  if (!next.length) next.push(emptyPart())
  emit('update:modelValue', next)
}

function isOnlyEmptyRow(idx: number): boolean {
  const row = props.modelValue[idx]
  const noOther = props.modelValue.length === 1
  return noOther && !row.key.trim() && !(row.type === 'file' && row.fileName)
}

/**
 * 显式触发同容器内的 file input。
 * 不能用 <label> 关联：el-button 渲染为 <button>，属于 labelable 元素，
 * 点击它会触发自身行为而不会激活 label 关联的 file input，导致「点选文件无反应」。
 */
function pickFile(e: MouseEvent) {
  const container = (e.currentTarget as HTMLElement).closest('.fd-file')
  container?.querySelector<HTMLInputElement>('input[type="file"]')?.click()
}

function onFileChange(idx: number, e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    const result = reader.result as string
    // data URL 形如 "data:image/png;base64,xxxx"，去掉前缀仅保留 base64 主体，
    // 与 Rust `FormPart.data` 的约定一致（不含 data: 前缀）。
    const base64 = result.includes(',') ? result.split(',')[1] : result
    updateRow(idx, {
      fileName: file.name,
      fileMime: file.type || undefined,
      fileSize: file.size,
      fileData: base64,
    })
  }
  reader.readAsDataURL(file)
  // 重置 input，使选择同一文件也能再次触发 change
  input.value = ''
}

function clearFile(idx: number) {
  updateRow(idx, { fileName: '', fileMime: undefined, fileSize: 0, fileData: '' })
}
</script>

<style scoped>
.formdata-editor {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.fd-add-row {
  padding: 2px 6px;
}
.fd-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border-radius: 6px;
  transition: background 0.12s cubic-bezier(0.16, 1, 0.3, 1);
}
.fd-row:hover {
  background: rgba(99,102,241,0.03);
}
.fd-type {
  width: 78px;
  flex: 0 0 78px;
}
.fd-key { flex: 1 1 0; min-width: 0; }
.fd-key :deep(input) {
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-sm);
  letter-spacing: -0.01em;
}
.fd-val { flex: 2 1 0; min-width: 0; }
.fd-val :deep(input) {
  font-size: var(--fs-sm);
}
.fd-desc { flex: 2 1 0; min-width: 0; }
.fd-desc :deep(input) {
  font-size: var(--fs-sm);
}
/* 删除按钮始终可见，不参与收缩 */
.fd-row > .el-button {
  flex: 0 0 auto;
}
.fd-file {
  flex: 2;
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  min-width: 0;
}
/* 视觉隐藏但保留在 DOM 中，保证 programmatic click() 能可靠唤起文件选择框
   （display:none 的 input 在部分嵌入 WebView 下无法被 click() 触发） */
.fd-file-input {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  overflow: hidden;
  pointer-events: none;
}
.fd-file-meta {
  font-size: var(--fs-xs, 12px);
  color: var(--c-text-secondary, #8a8f99);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 220px;
}
.fd-file-clear {
  cursor: pointer;
  color: var(--c-danger, #f56c6c);
  font-size: 12px;
  flex: 0 0 auto;
}
.fd-file-clear:hover {
  opacity: 0.75;
}
</style>
