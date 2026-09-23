<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { useWebSocket } from '@/composables/useWebSocket'

const { t } = useI18n()
const { connected, url, logs, error, connect, send, disconnect, clearLogs } = useWebSocket()
const inputMsg = ref('')

async function onConnect() {
  try {
    await connect()
    ElMessage.success(t('ws.connect') + ' ✔')
  } catch (e: any) {
    ElMessage.error(t('ws.connect') + ' ✘: ' + (e?.toString?.() ?? e))
  }
}

async function onDisconnect() {
  await disconnect()
  ElMessage.info(t('ws.disconnect') + ' ✔')
}

async function onSend() {
  if (!inputMsg.value.trim()) return
  await send(inputMsg.value)
  inputMsg.value = ''
}
</script>

<template>
  <div class="ws-tester">
    <div class="ws-bar">
      <el-input v-model="url" :placeholder="t('ws.url')" :disabled="connected" class="ws-url" />
      <el-button v-if="!connected" type="primary" @click="onConnect">{{ t('ws.connect') }}</el-button>
      <el-button v-else type="danger" @click="onDisconnect">{{ t('ws.disconnect') }}</el-button>
      <el-button @click="clearLogs">清空</el-button>
    </div>
    <div v-if="error" class="ws-error">{{ error }}</div>
    <div class="ws-logs">
      <div
        v-for="log in logs"
        :key="log.id"
        class="ws-log"
        :class="`ws-log--${log.direction}`"
      >
        <span class="ws-log-time">{{ log.time }}</span>
        <span class="ws-log-dir">{{ log.direction === 'recv' ? '接收' : log.direction === 'sent' ? '发送' : '系统' }}</span>
        <span class="ws-log-text">{{ log.text }}</span>
      </div>
      <EmptyState v-if="!logs.length" compact title="暂无消息" description="连接后发送或接收数据将显示在此" />
    </div>
    <div class="ws-send">
      <el-input
        v-model="inputMsg"
        type="textarea"
        :rows="3"
        :placeholder="t('ws.placeholder')"
        :disabled="!connected"
        @keydown.enter.exact.prevent="onSend"
      />
      <el-button type="primary" :disabled="!connected" @click="onSend">{{ t('ws.send') }}</el-button>
    </div>
  </div>
</template>

<style scoped>
.ws-tester {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  padding: 12px;
  box-sizing: border-box;
}
.ws-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.ws-url {
  flex: 1;
}
.ws-error {
  color: #f56c6c;
  font-size: var(--fs-md);
}
.ws-logs {
  flex: 1;
  overflow-y: auto;
  border: 1px solid var(--el-border-color, #dcdfe6);
  border-radius: 4px;
  padding: 8px;
  background: var(--surface-2);
  font-family: monospace;
  font-size: var(--fs-md);
}
.ws-log {
  display: flex;
  gap: 8px;
  padding: 2px 0;
  line-height: 1.5;
}
.ws-log--recv .ws-log-dir { color: #67c23a; }
.ws-log--sent .ws-log-dir { color: #409eff; }
.ws-log--system .ws-log-dir { color: var(--text-3); }
.ws-log-time { color: var(--text-3); }
.ws-log-dir { font-weight: bold; min-width: 32px; }
.ws-log-text { white-space: pre-wrap; word-break: break-all; }
.ws-empty {
  color: var(--text-3);
  text-align: center;
  padding: 24px;
}
.ws-send {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}
.ws-send .el-button {
  flex-shrink: 0;
}
</style>
