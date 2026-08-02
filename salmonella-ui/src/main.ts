import { createApp } from 'vue'
import '@fontsource/cairo/400.css'
import '@fontsource/cairo/600.css'
import '@fontsource/cairo/700.css'
import '@fontsource/cairo/900.css'
import './style.css'
import App from './App.vue'
import { currentMode, setMode, listenSystemTheme } from './lib/theme'

setMode(currentMode()) // applies stored mode (default 'system') — re-save is harmless
listenSystemTheme()

window.addEventListener('wheel', (e) => { if (e.ctrlKey) e.preventDefault() }, { passive: false })
window.addEventListener('keydown', (e) => {
  if (e.ctrlKey && (e.key === '+' || e.key === '=' || e.key === '-' || e.key === '0')) e.preventDefault()
})
for (const t of ['gesturestart', 'gesturechange', 'gestureend']) {
  window.addEventListener(t, (e) => e.preventDefault(), { passive: false })
}

createApp(App).mount('#app')
