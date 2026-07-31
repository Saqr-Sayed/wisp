import { createApp } from 'vue'
import '@fontsource/cairo/400.css'
import '@fontsource/cairo/600.css'
import '@fontsource/cairo/700.css'
import '@fontsource/cairo/900.css'
import './style.css'
import App from './App.vue'

const saved = localStorage.getItem('salmonella-theme')
document.documentElement.setAttribute('data-theme', saved === 'dark' ? 'dark' : 'light')
createApp(App).mount('#app')
