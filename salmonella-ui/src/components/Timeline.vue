<script setup lang="ts">
import { ref, computed } from 'vue'
import { formatTime, formatDuration, eventDuration, CATEGORY_COLORS, categoryLabel, type LogEntry } from '../lib/dbus'

const props = defineProps<{ logs: LogEntry[] }>()
const total = computed(() => props.logs.reduce((s, l) => s + eventDuration(l), 0))
const selected = ref<LogEntry | null>(null)
</script>

<template>
  <div class="timeline">
    <h2>الخط الزمني</h2>
    <div class="segments">
      <div v-for="l in logs" :key="l.id" class="seg" :title="l.window_title"
        :style="{ width: (total ? eventDuration(l) / total * 100 : 0) + '%', background: CATEGORY_COLORS[l.category] ?? '#555' }"
        @click="selected = l"></div>
    </div>
    <div v-if="selected" class="detail">
      <b>{{ selected.friendly_name || selected.app_name }}</b>
      <span v-if="selected.site" class="site">{{ selected.site }}</span>
      <span>{{ selected.window_title }}</span>
      <span class="time">{{ formatTime(selected.start_time) }} · {{ formatDuration(eventDuration(selected)) }}</span>
    </div>
    <div v-if="logs.length === 0" class="empty">لا توجد أحداث في هذه الفترة</div>
    <div v-for="log in logs" :key="'r' + log.id" class="entry" @click="selected = log" :class="{ on: selected?.id === log.id }">
      <span class="time">{{ formatTime(log.start_time) }}</span>
      <span class="badge" :style="{ background: CATEGORY_COLORS[log.category] ?? '#555' }">{{ categoryLabel(log.category) }}</span>
      <span class="app">{{ log.friendly_name || log.app_name }}</span>
      <span v-if="log.site" class="site">{{ log.site }}</span>
      <span class="title">{{ log.window_title }}</span>
      <span class="duration">{{ formatDuration(eventDuration(log)) }}</span>
    </div>
  </div>
</template>

<style scoped>
.timeline { margin-top: 1rem; }
.segments { display: flex; height: 1.5rem; border-radius: 4px; overflow: hidden; margin-bottom: 1rem; gap: 1px; background: #111; }
.seg { cursor: pointer; min-width: 2px; }
.detail { background: #111; border: 1px solid #333; border-radius: 8px; padding: 0.6rem 1rem; margin-bottom: 1rem; display: flex; flex-direction: column; gap: 0.2rem; }
.site { color: #e94560; font-size: 0.85rem; }
.entry { display: flex; gap: 0.5rem; padding: 0.5rem; border-bottom: 1px solid #333; align-items: center; cursor: pointer; }
.entry.on { background: #161616; }
.time { color: #888; min-width: 4rem; }
.badge { padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; color: #fff; }
.app { color: #eee; font-weight: bold; }
.site { color: #e94560; }
.title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #aaa; }
.duration { color: #888; min-width: 4rem; text-align: left; }
.empty { color: #666; text-align: center; padding: 2rem; }
</style>
