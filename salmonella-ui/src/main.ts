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
createApp(App).mount('#app')
