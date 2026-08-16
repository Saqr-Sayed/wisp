import { createApp } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import '@fontsource/cairo/400.css'
import '@fontsource/cairo/600.css'
import '@fontsource/cairo/700.css'
import '@fontsource/cairo/900.css'
import './style.css'
import App from './App.vue'
import { currentMode, setMode, listenSystemTheme } from './lib/theme'

// Diagnostics: every JS error lands in <data_local>/wisp/frontend.log
// (via the log_frontend command), so blank-screen issues are debuggable
// from the user's machine without a console.
function flog(msg: string) { invoke('log_frontend', { msg }).catch(() => {}) }
window.addEventListener('error', (e) => flog(`[error] ${e.message} @${e.filename || ''}:${e.lineno || 0}`))
window.addEventListener('unhandledrejection', (e) => flog(`[rejection] ${(e.reason && (e.reason.stack || e.reason.message)) || e.reason}`))

setMode(currentMode()) // applies stored mode (default 'system') — re-save is harmless
listenSystemTheme()

window.addEventListener('contextmenu', (e) => e.preventDefault())
window.addEventListener('wheel', (e) => { if (e.ctrlKey) e.preventDefault() }, { passive: false })
window.addEventListener('keydown', (e) => {
  if (e.ctrlKey && (e.key === '+' || e.key === '=' || e.key === '-' || e.key === '0')) e.preventDefault()
})
for (const t of ['gesturestart', 'gesturechange', 'gestureend']) {
  window.addEventListener(t, (e) => e.preventDefault(), { passive: false })
}

flog('[boot] main.ts evaluated')

const app = createApp(App)
app.config.errorHandler = (err, _instance, info) => flog(`[vue] ${info}: ${(err as Error)?.message || err}`)
app.mount('#app')
flog('[boot] app mounted')