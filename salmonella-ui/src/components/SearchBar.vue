<script setup lang="ts">
import { ref, watch } from 'vue'
import { search, formatTime, formatDuration, categoryColor, categoryLabel, type LogEntry } from '../lib/dbus'

const query = ref('')
const results = ref<LogEntry[]>([])

watch(query, async (q) => {
  if (!q.trim()) {
    results.value = []
    return
  }
  results.value = await search(q)
})
</script>

<template>
  <div class="search">
    <input v-model="query" placeholder="ابحث في سجل النشاط..." />
    <div v-for="r in results" :key="'s' + r.id" class="entry">
      <span class="time">{{ formatTime(r.start_time) }}</span>
      <span class="badge" :style="{ background: categoryColor(r.category) }">{{ categoryLabel(r.category) }}</span>
      <span class="app">{{ r.friendly_name || r.app_name }}</span>
      <span v-if="r.site" class="site">{{ r.site }}</span>
      <span class="title">{{ r.window_title }}</span>
      <span class="duration">{{ formatDuration(r.duration) }}</span>
    </div>
  </div>
</template>

<style scoped>
.search { margin-bottom: 1rem; }
.search input { width: 100%; background: #111; border: 1px solid #333; border-radius: 8px; padding: 0.5rem 0.75rem; color: #eee; }
.entry { display: flex; gap: 0.5rem; padding: 0.5rem; border-bottom: 1px solid #222; align-items: center; }
.time { color: #888; min-width: 4rem; }
.badge { padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; color: #fff; }
.app { color: #aaa; }
.site { color: #e94560; font-size: 0.85rem; }
.title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.duration { color: #888; min-width: 4rem; text-align: left; }
</style>
