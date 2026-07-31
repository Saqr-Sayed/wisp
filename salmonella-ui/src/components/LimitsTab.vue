<script setup lang="ts">
import { ref, computed } from 'vue'
import { setLimit, removeLimit, eventDuration, categoryLabel, type LogEntry } from '../lib/dbus'

const props = defineProps<{ limits: [string, string, number][]; logs: LogEntry[] }>()
const emit = defineEmits<{ changed: [] }>()

const kind = ref<'app' | 'category'>('category')
const target = ref('')
const minutes = ref(60)

const usedMap = computed(() => {
  const m = new Map<string, number>()
  for (const l of props.logs) {
    const d = eventDuration(l)
    m.set(`category:${l.category}`, (m.get(`category:${l.category}`) ?? 0) + d)
    m.set(`app:${l.app_name}`, (m.get(`app:${l.app_name}`) ?? 0) + d)
  }
  return m
})

function label(target: string, kind: string): string {
  if (kind === 'category') return categoryLabel(target)
  const hit = props.logs.find(l => l.app_name === target)
  return hit?.friendly_name ?? target
}

function usedOf(kind: string, target: string): number {
  return Math.round((usedMap.value.get(`${kind}:${target}`) ?? 0) / 60)
}

async function add() {
  if (!target.value.trim() || !minutes.value) return
  await setLimit(target.value.trim(), kind.value, minutes.value)
  target.value = ''
  emit('changed')
}
</script>

<template>
  <div class="limits">
    <div class="add-form">
      <select v-model="kind">
        <option value="category">فئة</option>
        <option value="app">تطبيق (معرف)</option>
      </select>
      <input v-model="target" :placeholder="kind === 'category' ? 'مثال: media / productivity' : 'مثال: org.mozilla.firefox.desktop'" />
      <input v-model.number="minutes" type="number" min="1" placeholder="دقيقة/يوم" />
      <button @click="add">إضافة</button>
    </div>
    <div v-for="[t, k, m] in limits" :key="k + t" class="limit-row">
      <span class="lname">{{ label(t, k) }}</span>
      <span class="lused" :class="{ over: usedOf(k, t) > m }">{{ usedOf(k, t) }} / {{ m }} دقيقة</span>
      <button @click="removeLimit(t).then(() => emit('changed'))">حذف</button>
    </div>
    <div v-if="limits.length === 0" class="empty">لا حدود — أضف حداً يومياً لفئة أو تطبيق</div>
  </div>
</template>

<style scoped>
.limits { margin-top: 1rem; }
.add-form { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
.add-form input, .add-form select { background: #111; border: 1px solid #333; border-radius: 6px; padding: 0.4rem 0.6rem; color: #eee; }
.add-form input:first-of-type { flex: 1; }
.add-form button { background: #e94560; border: none; border-radius: 6px; padding: 0.4rem 1rem; color: #fff; cursor: pointer; }
.limit-row { display: flex; gap: 1rem; align-items: center; padding: 0.5rem; border-bottom: 1px solid #222; }
.lname { flex: 1; }
.lused { color: #888; }
.lused.over { color: #e94560; font-weight: bold; }
.limit-row button { background: #222; border: 1px solid #444; border-radius: 6px; color: #aaa; cursor: pointer; }
.empty { color: #666; text-align: center; padding: 2rem; }
</style>
