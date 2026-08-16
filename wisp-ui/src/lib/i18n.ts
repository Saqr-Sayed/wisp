import { ref } from 'vue'
import ar from '../i18n/ar.json' with { type: 'json' }
import en from '../i18n/en.json' with { type: 'json' }

export type Locale = 'ar' | 'en'
export type Setting = 'auto' | Locale

const DICTS: Record<Locale, Record<string, string>> = { ar: ar as any, en: en as any }

export const locale = ref<Locale>(detectSystem())

function detectSystem(): Locale {
  const l = (typeof navigator !== 'undefined' && navigator.language || 'ar').slice(0, 2)
  return l === 'en' ? 'en' : 'ar'
}

export function t(key: string, params?: Record<string, string | number>): string {
  const s = DICTS[locale.value][key] ?? DICTS.ar[key] ?? key
  if (!params) return s
  return Object.entries(params).reduce((acc, [k, v]) => acc.replace(`{${k}}`, String(v)), s)
}

export function setLocale(l: string) {
  // Trust boundary: backend/settings may hand us anything (e.g. '' for unset).
  // Anything that isn't a real locale falls back to the system detection —
  // an invalid locale.value would crash every t() render.
  const next: Locale = (l === 'ar' || l === 'en') ? l : detectSystem()
  locale.value = next
  if (typeof document !== 'undefined') {
    document.documentElement.dir = next === 'ar' ? 'rtl' : 'ltr'
    document.documentElement.lang = next
  }
}
