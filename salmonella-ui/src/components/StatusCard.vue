<script setup lang="ts">
import { computed } from 'vue'
import { formatTime, formatDuration, type LogEntry } from '../lib/dbus'

const props = defineProps<{
  status: [boolean, number, string, string]
  logs: LogEntry[]
}>()

const total = computed(() => props.logs.reduce((s, l) => s + (l.duration ?? 0), 0))
const media = computed(() =>
  props.logs.filter(l => l.event_type === 'media').reduce((s, l) => s + (l.duration ?? 0), 0)
)
</script>

<template>
  <div class="status">
    <div class="now">
      <span class="label">النشاط الحالي</span>
      <template v-if="status[0]">
        <span class="app">{{ status[2] }}</span>
        <span class="title">{{ status[3] }}</span>
        <span class="since">منذ {{ formatTime(status[1]) }}</span>
      </template>
      <span v-else class="idle">لا نشاط</span>
    </div>
    <div class="stats">
      <div class="stat"><span>إجمالي اليوم</span><b>{{ formatDuration(total) }}</b></div>
      <div class="stat"><span>وسائط</span><b>{{ formatDuration(media) }}</b></div>
    </div>
  </div>
</template>

<style scoped>
.status { display: flex; gap: 1rem; margin: 1rem 0; flex-wrap: wrap; }
.now { flex: 1; min-width: 280px; background: #111; border: 1px solid #333; border-radius: 8px; padding: 0.75rem 1rem; display: flex; flex-direction: column; gap: 0.25rem; }
.label { color: #888; font-size: 0.8rem; }
.app { font-weight: bold; }
.title { color: #ccc; font-size: 0.9rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.since { color: #e94560; font-size: 0.8rem; }
.idle { color: #666; }
.stats { display: flex; gap: 1rem; }
.stat { background: #111; border: 1px solid #333; border-radius: 8px; padding: 0.75rem 1rem; display: flex; flex-direction: column; min-width: 110px; }
.stat span { color: #888; font-size: 0.8rem; }
.stat b { font-size: 1.2rem; }
</style>
