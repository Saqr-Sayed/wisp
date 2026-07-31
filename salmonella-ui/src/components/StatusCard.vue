<script setup lang="ts">
import { computed } from 'vue'
import { formatDuration, type LogEntry } from '../lib/dbus'

const props = defineProps<{ logs: LogEntry[] }>()

const total = computed(() => props.logs.reduce((s, l) => s + (l.duration ?? 0), 0))
const media = computed(() =>
  props.logs.filter(l => l.event_type === 'media').reduce((s, l) => s + (l.duration ?? 0), 0)
)
</script>

<template>
  <div class="stats">
    <div class="stat"><span>إجمالي اليوم</span><b>{{ formatDuration(total) }}</b></div>
    <div class="stat"><span>وسائط</span><b>{{ formatDuration(media) }}</b></div>
  </div>
</template>

<style scoped>
.stats { display: flex; gap: 1rem; margin: 1rem 0; }
.stat { background: #111; border: 1px solid #333; border-radius: 8px; padding: 0.75rem 1rem; display: flex; flex-direction: column; min-width: 110px; }
.stat span { color: #888; font-size: 0.8rem; }
.stat b { font-size: 1.2rem; }
</style>
