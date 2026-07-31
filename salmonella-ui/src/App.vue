<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import Timeline from './components/Timeline.vue'
import StatusCard from './components/StatusCard.vue'
import SearchBar from './components/SearchBar.vue'
import { getStatus, getTimeline, type LogEntry } from './lib/dbus'

const logs = ref<LogEntry[]>([])

async function refresh() {
  const now = Math.floor(Date.now() / 1000)
  const today = now - (now % 86400)
  logs.value = await getTimeline(today, now)
}

let timer: number | undefined
onMounted(async () => {
  await refresh()
  timer = window.setInterval(refresh, 5000)
})
onUnmounted(() => window.clearInterval(timer))
</script>

<template>
  <div id="shell" class="rtl">
    <header><h1>Salmonella</h1></header>
    <main>
      <StatusCard :logs="logs" />
      <SearchBar />
      <Timeline :logs="logs" />
    </main>
  </div>
</template>

<style>
@import './style.css';
</style>
