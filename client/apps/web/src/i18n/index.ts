import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN'
import en from './locales/en'

export type LangKey = 'zh-CN' | 'en'

const stored = (localStorage.getItem('canghai-api-lang') as LangKey) || 'zh-CN'

export const i18n = createI18n({
  legacy: false,
  locale: stored,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    en,
  },
})

export const langOptions: { value: LangKey; label: string }[] = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en', label: 'English' }
]

export function setLang(lang: LangKey) {
  i18n.global.locale.value = lang
  localStorage.setItem('canghai-api-lang', lang)
  document.querySelector('html')?.setAttribute('lang', lang)
}
