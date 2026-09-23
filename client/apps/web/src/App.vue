<template>
  <el-config-provider :locale="elLocale">
    <!-- authReady：initAuth 完成后才挂载路由视图，确保子组件 onMounted 里的
         load() 一定发生在 currentUser 就绪之后，避免首次 get_projects 带空 userId
         且 isLoggedIn 由 false→true 重复触发一次 load 的竞态问题。 -->
    <router-view v-if="authReady" v-slot="{ Component, route }">
      <component
        :is="Component"
        :project-id="route.params.projectId as string || null"
      />
    </router-view>
  </el-config-provider>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import zhTw from 'element-plus/es/locale/lang/zh-tw'
import en from 'element-plus/es/locale/lang/en'
import { i18n } from '@/i18n'
import { initAuth } from '@/composables/useSync'

const elLocaleMap = { 'zh-CN': zhCn, 'zh-TW': zhTw, en } as const
const elLocale = computed(() => elLocaleMap[i18n.global.locale.value as keyof typeof elLocaleMap] ?? zhCn)

const authReady = ref(false)
onMounted(async () => {
  await initAuth()
  authReady.value = true
})
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
html,
body,
#app {
  height: 100%;
  width: 100%;
  background: var(--bg);
}
</style>
