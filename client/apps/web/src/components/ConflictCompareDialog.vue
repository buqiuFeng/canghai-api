<script setup lang="ts">
import { useConflict } from '../composables/useConflict'

const { conflict, resolving, closeConflict, setResolving } = useConflict()

async function confirm() {
  if (!conflict.value) return
  setResolving(true)
  try {
    await conflict.value.onConfirm()
    closeConflict()
  } catch (e: unknown) {
    window.alert('保存失败：' + (e instanceof Error ? e.message : String(e)))
  } finally {
    setResolving(false)
  }
}

function cancel() {
  closeConflict()
}

function val(obj: Record<string, unknown> | undefined, key: string): string {
  if (!obj) return ''
  const v = obj[key]
  if (v === null || v === undefined) return ''
  if (typeof v === 'object') return JSON.stringify(v)
  return String(v)
}
</script>

<template>
  <div v-if="conflict" class="conflict-mask" @click.self="cancel">
    <div class="conflict-dialog">
      <div class="conflict-header">
        <span class="conflict-title">⚠ 数据冲突 - {{ conflict.title }}</span>
      </div>
      <div class="conflict-tip">
        服务器上的数据已被其他人修改（更新时间比本地新）。请对比左右两侧，确认是否仍要用你的版本覆盖服务器。
      </div>
      <table class="conflict-table">
        <thead>
          <tr>
            <th class="col-field">字段</th>
            <th class="col-server">服务器最新版本</th>
            <th class="col-local">你正在编辑的版本</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in conflict.fields" :key="f.key"
              :class="{ 'row-diff': val(conflict.server, f.key) !== val(conflict.local, f.key) }">
            <td class="col-field">{{ f.label }}</td>
            <td class="col-server">{{ val(conflict.server, f.key) }}</td>
            <td class="col-local">{{ val(conflict.local, f.key) }}</td>
          </tr>
        </tbody>
      </table>
      <div class="conflict-footer">
        <button class="btn-cancel" :disabled="resolving" @click="cancel">取消</button>
        <button class="btn-confirm" :disabled="resolving" @click="confirm">
          {{ resolving ? '保存中…' : '仍要保存（覆盖服务器）' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.conflict-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}
.conflict-dialog {
  width: 720px;
  max-width: 92vw;
  max-height: 86vh;
  background: var(--panel-bg, var(--surface));
  border-radius: 10px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.conflict-header {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border-color, #eee);
}
.conflict-title {
  font-size: var(--fs-xl);
  font-weight: 600;
  color: #d97706;
}
.conflict-tip {
  padding: 10px 18px;
  font-size: var(--fs-md);
  color: #666;
  line-height: 1.6;
}
.conflict-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-md);
  overflow: auto;
  flex: 1;
}
.conflict-table th,
.conflict-table td {
  border: 1px solid var(--border-color, #eee);
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
  word-break: break-all;
}
.conflict-table th {
  background: var(--surface-2);
  font-weight: 600;
}
.col-field {
  width: 110px;
  color: #888;
}
.col-server {
  width: 45%;
  background: var(--tint-warning);
}
.col-local {
  width: 45%;
  background: var(--brand-soft);
}
.row-diff .col-server,
.row-diff .col-local {
  font-weight: 600;
}
.row-diff {
  background: var(--tint-warning);
}
.conflict-footer {
  padding: 12px 18px;
  border-top: 1px solid var(--border-color, #eee);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.btn-cancel,
.btn-confirm {
  padding: 7px 16px;
  border-radius: 6px;
  border: 1px solid var(--border-color, #ccc);
  cursor: pointer;
  font-size: var(--fs-md);
}
.btn-cancel {
  background: var(--surface-2);
}
.btn-confirm {
  background: var(--brand-1);
  color: #fff;
  border-color: #2563eb;
}
.btn-confirm:disabled,
.btn-cancel:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
