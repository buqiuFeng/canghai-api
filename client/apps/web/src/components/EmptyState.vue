<template>
  <div class="empty-state" :class="{ 'empty-state--compact': compact }">
    <div class="empty-state__icon">
      <el-icon><component :is="icon" /></el-icon>
    </div>
    <div class="empty-state__title">{{ title }}</div>
    <div v-if="description" class="empty-state__desc">{{ description }}</div>
    <div v-if="$slots.default" class="empty-state__action">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Component } from 'vue'
import { Box } from '@element-plus/icons-vue'

/**
 * 统一空状态占位：一致的图标底、标题、描述与可选操作区。
 * 用法：
 *   <EmptyState title="暂无历史" />
 *   <EmptyState compact title="暂无成员" description="点击 + 添加成员" />
 */
withDefaults(defineProps<{
  title: string
  description?: string
  /** 图标（Element Plus 图标组件），默认 Box */
  icon?: Component
  /** 紧凑模式：用于侧栏/抽屉/弹窗内的窄区域 */
  compact?: boolean
}>(), {
  description: '',
  icon: () => Box,
  compact: false,
})
</script>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 28px 16px;
  text-align: center;
  user-select: none;
}
.empty-state--compact {
  padding: 16px 12px;
  gap: 4px;
}
.empty-state__icon {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  background: var(--brand-soft, rgba(99, 102, 241, 0.1));
  color: var(--brand-1, #6366f1);
  margin-bottom: 2px;
}
.empty-state__icon .el-icon {
  font-size: var(--icon-lg, 20px);
}
.empty-state--compact .empty-state__icon {
  width: 34px;
  height: 34px;
  border-radius: 10px;
}
.empty-state--compact .empty-state__icon .el-icon {
  font-size: var(--icon-sm, 14px);
}
.empty-state__title {
  font-size: var(--fs-md, 13px);
  font-weight: 600;
  color: var(--text-2, #334155);
}
.empty-state__desc {
  font-size: var(--fs-sm, 12px);
  line-height: var(--lh-snug, 1.45);
  color: var(--text-3, #64748b);
  max-width: 320px;
}
.empty-state__action {
  margin-top: 6px;
}
</style>
