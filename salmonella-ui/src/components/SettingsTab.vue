<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getNameOverrides, setNameOverride, removeNameOverride } from '../lib/dbus'

const emit = defineEmits<{ changed: [] }>()
const overrides = ref<[string, string][]>([])
const appId = ref('')
const friendly = ref('')

onMounted(async () => { overrides.value = await getNameOverrides() })

async function add() {
  if (!appId.value.trim() || !friendly.value.trim()) return
  await setNameOverride(appId.value.trim(), friendly.value.trim())
  appId.value = ''
  friendly.value = ''
  overrides.value = await getNameOverrides()
  emit('changed')
}
</script>

<template>
  <div class="settings">
    <h2>أسماء التطبيقات</h2>
    <p class="hint">أضف اسماً ودياً لمعرف تطبيق — يظهر في كل التقارير. مثال: <code>org.mozilla.firefox.desktop</code> → <code>فايرفوكس</code></p>
    <div class="add-form">
      <input v-model="appId" placeholder="معرف التطبيق (app id)" />
      <input v-model="friendly" placeholder="الاسم الودي" />
      <button @click="add">إضافة / تعديل</button>
    </div>
    <div v-for="[id, f] in overrides" :key="id" class="row">
      <code>{{ id }}</code>
      <span class="arrow">→</span>
      <b>{{ f }}</b>
      <button @click="removeNameOverride(id).then(() => overrides.value = overrides.value.filter(o => o[0] !== id)).then(() => emit('changed'))">حذف</button>
    </div>
    <div v-if="overrides.length === 0" class="empty">لا تعديلات</div>
  </div>
</template>

<style scoped>
.settings { margin-top: 1rem; }
.hint { color: #888; font-size: 0.85rem; }
.add-form { display: flex; gap: 0.5rem; margin: 0.5rem 0 1rem; }
.add-form input { flex: 1; background: #111; border: 1px solid #333; border-radius: 6px; padding: 0.4rem 0.6rem; color: #eee; }
.add-form button, .row button { background: #e94560; border: none; border-radius: 6px; padding: 0.4rem 0.8rem; color: #fff; cursor: pointer; }
.row { display: flex; gap: 0.5rem; align-items: center; padding: 0.4rem; border-bottom: 1px solid #222; }
.row b { flex: 1; }
.row button { background: #222; border: 1px solid #444; color: #aaa; }
.arrow { color: #666; }
.empty { color: #666; text-align: center; padding: 1rem; }
</style>
