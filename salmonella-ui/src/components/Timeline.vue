<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getTimeline, formatTime, formatDuration, type LogEntry } from '../lib/dbus'

const logs = ref<LogEntry[]>([])

onMounted(async () => {
  const now = Math.floor(Date.now() / 1000)
  const today = now - (now % 86400)
  logs.value = await getTimeline(today, now)
})
</script>

<template>
  <div class="timeline">
    <h2>الخط الزمني</h2>
    <div v-if="logs.length === 0" class="empty">لا توجد أحداث اليوم</div>
    <div v-for="log in logs" :key="log.id" class="entry">
      <span class="time">{{ formatTime(log.start_time) }}</span>
      <span class="badge" :class="log.event_type">{{ log.event_type }}</span>
      <span class="app">{{ log.app_name }}</span>
      <span class="title">{{ log.window_title }}</span>
      <span class="duration">{{ formatDuration(log.duration) }}</span>
    </div>
  </div>
</template>

<style scoped>
.timeline { margin-top: 1rem; }
.entry { display: flex; gap: 0.5rem; padding: 0.5rem; border-bottom: 1px solid #333; align-items: center; }
.time { color: #888; min-width: 4rem; }
.badge { padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
.badge.media { background: #e94560; }
.badge.app { background: #0f3460; }
.badge.system { background: #533483; }
.app { color: #aaa; }
.title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.duration { color: #888; min-width: 4rem; text-align: left; }
.empty { color: #666; text-align: center; padding: 2rem; }
</style>
