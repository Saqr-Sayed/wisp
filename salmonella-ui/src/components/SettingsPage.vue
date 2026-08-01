<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  setLimit, removeLimit, eventDuration, categoryLabel,
  getNameOverrides, setNameOverride, removeNameOverride,
  getSetting, setSetting,
  type LogEntry,
} from '../lib/dbus'
import { listCustomCategories, addCustomCategory, removeCustomCategory, type CustomCategory } from '../lib/dbus'
import { currentMode, setMode, type ThemeMode } from '../lib/theme'
import { t, setLocale } from '../lib/i18n'

const props = defineProps<{ limits: [string, string, number][]; todayLogs: LogEntry[] }>()
const emit = defineEmits<{ back: []; changed: [] }>()

const theme = ref<ThemeMode>(currentMode())
function toggleTheme(m: ThemeMode) {
  theme.value = m
  setMode(m)
}

const lang = ref<'auto' | 'ar' | 'en'>('auto')
onMounted(async () => {
  const v = await getSetting('language').catch(() => 'auto')
  if (v === 'ar' || v === 'en' || v === 'auto') lang.value = v
  overrides.value = await getNameOverrides()
})
async function setLang(v: 'auto' | 'ar' | 'en') {
  lang.value = v
  await setSetting('language', v)
  setLocale(v)
}

const kind = ref<'app' | 'category'>('category')
const target = ref('')
const minutes = ref(60)

const usedMap = computed(() => {
  const m = new Map<string, number>()
  for (const l of props.todayLogs) {
    const d = eventDuration(l)
    m.set(`category:${l.category}`, (m.get(`category:${l.category}`) ?? 0) + d)
    m.set(`app:${l.app_name}`, (m.get(`app:${l.app_name}`) ?? 0) + d)
  }
  return m
})

function label(target: string, kind: string): string {
  if (kind === 'category') return categoryLabel(target)
  const hit = props.todayLogs.find(l => l.app_name === target)
  return hit?.friendly_name ?? target
}

function usedOf(kind: string, target: string): number {
  return Math.round((usedMap.value.get(`${kind}:${target}`) ?? 0) / 60)
}

async function addLimit() {
  if (!target.value.trim() || !minutes.value) return
  await setLimit(target.value.trim(), kind.value, minutes.value)
  target.value = ''
  emit('changed')
}

const overrides = ref<[string, string][]>([])
const appId = ref('')
const friendly = ref('')

async function addOverride() {
  if (!appId.value.trim() || !friendly.value.trim()) return
  await setNameOverride(appId.value.trim(), friendly.value.trim())
  appId.value = ''
  friendly.value = ''
  overrides.value = await getNameOverrides()
  emit('changed')
}

const cats = ref<CustomCategory[]>([])
const catKind = ref<'app' | 'site'>('app')
const catTarget = ref('')
const catName = ref('')

async function refreshCats() { cats.value = await listCustomCategories() }
onMounted(refreshCats)

async function addCat() {
  if (!catTarget.value.trim() || !catName.value.trim()) return
  await addCustomCategory(catKind.value, catTarget.value.trim(), catName.value.trim())
  catTarget.value = ''; catName.value = ''
  await refreshCats()
}

async function delCat(id: number) {
  await removeCustomCategory(id)
  await refreshCats()
}
</script>

<template>
  <div class="settings-page">
    <header class="s-head">
      <button class="icon-btn" :aria-label="t('settings.back')" @click="emit('back')">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
      </button>
      <b class="s-title">{{ t('settings.title') }}</b>
    </header>

    <div class="s-layout">
      <aside class="s-nav">
        <a href="#sec-general">{{ t('settings.section.general') }}</a>
        <a href="#sec-limits">{{ t('settings.section.limits') }}</a>
        <a href="#sec-overrides">{{ t('settings.section.overrides') }}</a>
        <a href="#sec-categories">{{ t('settings.section.categories') }}</a>
      </aside>

      <div class="s-content">
        <section id="sec-general" class="card">
          <h3>{{ t('settings.section.general') }}</h3>
          <div class="row">
            <div class="s-lang">
              <h4>{{ t('settings.language.label') }}</h4>
              <div class="theme-toggle">
                <button class="pill" :class="{ on: lang === 'auto' }" @click="setLang('auto')">{{ t('settings.language.auto') }}</button>
                <button class="pill" :class="{ on: lang === 'ar' }" @click="setLang('ar')">العربية</button>
                <button class="pill" :class="{ on: lang === 'en' }" @click="setLang('en')">English</button>
              </div>
            </div>
            <div class="s-theme">
              <h4>{{ t('settings.section.theme') }}</h4>
              <div class="theme-toggle">
                <button class="pill" :class="{ on: theme === 'system' }" @click="toggleTheme('system')">{{ t('settings.theme.system') }}</button>
                <button class="pill" :class="{ on: theme === 'light' }" @click="toggleTheme('light')">{{ t('settings.theme.light') }}</button>
                <button class="pill" :class="{ on: theme === 'dark' }" @click="toggleTheme('dark')">{{ t('settings.theme.dark') }}</button>
              </div>
            </div>
          </div>
        </section>

        <section id="sec-limits" class="card s-limits">
          <h3>{{ t('settings.section.limits') }}</h3>
          <div class="add-form">
            <select v-model="kind">
              <option value="category">{{ t('settings.limits.kind.category') }}</option>
              <option value="app">{{ t('settings.limits.kind.app') }}</option>
            </select>
            <input v-model="target" :placeholder="kind === 'category' ? t('settings.limits.placeholder.category') : t('settings.limits.placeholder.app')" />
            <input v-model.number="minutes" type="number" min="1" :placeholder="t('settings.limits.minutes')" />
            <button class="btn primary" @click="addLimit">{{ t('settings.limits.add') }}</button>
          </div>
          <div v-for="[t0, k, m] in limits" :key="k + ':' + t0" class="limit-row">
            <span class="lname">{{ label(t0, k) }}</span>
            <span class="lused" :class="{ over: usedOf(k, t0) > m }">{{ t('settings.limits.used', { used: usedOf(k, t0), max: m }) }}</span>
            <span v-if="usedOf(k, t0) > m" class="over-label">{{ t('settings.limits.exceeded') }}</span>
            <button class="btn ghost small" @click="removeLimit(t0).then(() => emit('changed'))">{{ t('settings.limits.delete') }}</button>
          </div>
          <div v-if="limits.length === 0" class="empty">{{ t('settings.limits.empty') }}</div>
        </section>

        <section id="sec-overrides" class="card s-overrides">
          <h3>{{ t('settings.section.overrides') }}</h3>
          <p class="hint">{{ t('settings.overrides.hint') }}</p>
          <div class="add-form">
            <input v-model="appId" :placeholder="t('settings.overrides.placeholder.appId')" />
            <input v-model="friendly" :placeholder="t('settings.overrides.placeholder.friendly')" />
            <button class="btn primary" @click="addOverride">{{ t('settings.overrides.add') }}</button>
          </div>
          <div v-for="[id, f] in overrides" :key="id" class="override-row">
            <code>{{ id }}</code><span class="arrow">→</span><b>{{ f }}</b>
            <button class="btn ghost small" @click="removeNameOverride(id).then(() => overrides = overrides.filter(o => o[0] !== id)).then(() => emit('changed'))">{{ t('settings.overrides.delete') }}</button>
          </div>
          <div v-if="overrides.length === 0" class="empty">{{ t('settings.overrides.empty') }}</div>
        </section>

        <section id="sec-categories" class="card s-cats">
          <h3>{{ t('settings.section.categories') }}</h3>
          <p class="hint">{{ t('settings.categories.hint') }}</p>
          <div class="add-form">
            <select v-model="catKind">
              <option value="app">{{ t('settings.categories.kind.app') }}</option>
              <option value="site">{{ t('settings.categories.kind.site') }}</option>
            </select>
            <input v-model="catTarget" :placeholder="t('settings.categories.placeholder.target')" />
            <input v-model="catName" :placeholder="t('settings.categories.placeholder.name')" />
            <button class="btn primary" @click="addCat">{{ t('settings.categories.add') }}</button>
          </div>
          <div v-for="c in cats" :key="c.id" class="override-row">
            <span class="pill small">{{ t('settings.categories.kind.' + c.kind) }}</span>
            <code>{{ c.target }}</code><span class="arrow">→</span><b>{{ c.display_name }}</b>
            <button class="btn ghost small" @click="delCat(c.id)">{{ t('settings.categories.delete') }}</button>
          </div>
          <div v-if="cats.length === 0" class="empty">{{ t('settings.categories.empty') }}</div>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-page { display: flex; flex-direction: column; gap: 0.9rem; flex: 1; min-height: 0; }
.s-head { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 0 0.2rem; }
.s-title { font-size: 1.05rem; font-weight: 900; }
.s-layout { display: flex; gap: 14px; flex: 1; min-height: 0; }
.s-nav {
  width: 180px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px;
  position: sticky; top: 0; align-self: flex-start; padding-top: 0.6rem;
}
.s-nav a {
  color: var(--ink-muted); font-weight: 700; font-size: 0.9rem;
  padding: 0.45rem 0.7rem; border-radius: 8px; text-decoration: none;
}
.s-nav a:hover { background: var(--surface-soft); color: var(--ink); }
.s-content { flex: 1; display: flex; flex-direction: column; gap: 14px; min-width: 0; overflow-y: auto; padding-inline-end: 0.3rem; }
.s-content .card { padding: 1.1rem 1.2rem; max-width: 760px; }
.s-content .card h3 { font-size: 1rem; margin-bottom: 0.7rem; }
.s-content .card h4 { font-size: 0.85rem; font-weight: 800; margin-bottom: 0.4rem; color: var(--ink-muted); }
.row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.add-form { display: flex; gap: 0.5rem; margin-bottom: 0.7rem; flex-wrap: wrap; }
.add-form input, .add-form select {
  background: var(--surface-soft); border: 1px solid var(--border); border-radius: 8px;
  padding: 0.45rem 0.6rem; color: var(--ink); font-family: inherit; font-size: 0.85rem; min-width: 0;
}
.add-form input { flex: 1 1 180px; }
.limit-row, .override-row {
  display: flex; gap: 0.7rem; align-items: center; padding: 0.55rem 0;
  border-bottom: 1px solid var(--border); flex-wrap: wrap;
}
.lname { flex: 1; font-weight: 600; min-width: 0; }
.lused { color: var(--ink-muted); font-size: 0.85rem; }
.lused.over { color: var(--danger); font-weight: 700; }
.over-label { color: var(--danger); font-size: 0.75rem; font-weight: 700; }
.btn.small { padding: 0.25rem 0.7rem; font-size: 0.75rem; }
.theme-toggle { display: flex; gap: 5px; flex-wrap: wrap; }
.hint { color: var(--ink-muted); font-size: 0.8rem; margin-bottom: 0.7rem; }
.hint code { background: var(--surface-soft); border-radius: 4px; padding: 0 4px; }
.override-row code { font-size: 0.78rem; }
.override-row b { flex: 1; }
.pill.small { font-size: 0.7rem; padding: 0.15rem 0.55rem; }
.arrow { color: var(--ink-muted); }
.empty { color: var(--ink-muted); font-size: 0.85rem; padding: 0.6rem 0; }
</style>
