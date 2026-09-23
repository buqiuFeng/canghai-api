import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import 'element-plus/dist/index.css'
// Element Plus 暗色变量（以 html.dark 为选择器）
import 'element-plus/theme-chalk/dark/css-vars.css'
import '@/static/main.css'
import App from './App.vue'
import { i18n } from '@/i18n'
import router from '@/router'
import { pinia } from '@/stores/pinia'
import { initTheme } from '@/composables/useTheme'

const app = createApp(App)

for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

app.use(pinia)
app.use(ElementPlus)
app.use(i18n)
app.use(router)
// 挂载前应用主题，避免首帧闪白
initTheme()
app.mount('#app')
