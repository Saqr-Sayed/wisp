<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  setLimit, removeLimit, eventDuration,
  setNameOverride, removeNameOverride,
  getSetting, setSetting,
  getKnownApps, getKnownSites, setSiteOverride, removeSiteOverride,
  listIgnored, ignoreTarget, unignoreTarget,
  listCustomCategories, addCustomCategory, removeCustomCategory,
  type LogEntry, type KnownApp, type KnownSite, type CustomCategory,
} from '../lib/dbus'
import { currentMode, setMode, type ThemeMode } from '../lib/theme'
import { t, setLocale } from '../lib/i18n'

const props = defineProps<{ limits: [string, string, number][]; todayLogs: LogEntry[] }>()
const emit = defineEmits<{ back: []; changed: [] }>()

const theme = ref<ThemeMode>(currentMode())
function toggleTheme(m: ThemeMode) { theme.value = m; setMode(m) }

const lang = ref<'auto' | 'ar' | 'en'>('auto')
onMounted(async () => {
  const v = await getSetting('language').catch(() => 'auto')
  if (v === 'ar' || v === 'en' || v === 'auto') lang.value = v
  await Promise.all([refreshKnown(), refreshCats(), refreshIgnored()])
})
async function setLang(v: 'auto' | 'ar' | 'en') {
  lang.value = v
  await setSetting('language', v)
  setLocale(v)
}

const usedMap = computed(() => {
  const m = new Map<string, number>()
  for (const l of props.todayLogs) {
    const d = eventDuration(l)
    m.set(`category:${l.category}`, (m.get(`category:${l.category}`) ?? 0) + d)
    m.set(`app:${l.app_name}`, (m.get(`app:${l.app_name}`) ?? 0) + d)
    if (l.site) m.set(`site:${l.site}`, (m.get(`site:${l.site}`) ?? 0) + d)
  }
  return m
})
function usedOf(kind: string, target: string): number {
  return Math.round((usedMap.value.get(`${kind}:${target}`) ?? 0) / 60)
}
const limitsBy = computed(() => {
  const m = new Map<string, number>() // `kind:target` -> minutes
  for (const [target, kind, minutes] of props.limits) m.set(`${kind}:${target}`, minutes)
  return m
})
function limitOf(kind: string, target: string): number | undefined {
  return limitsBy.value.get(`${kind}:${target}`)
}

// ── التصنيفات ────────────────────────────────────────
const cats = ref<CustomCategory[]>([])
const knownApps = ref<KnownApp[]>([])
const knownSites = ref<KnownSite[]>([])
const catKind = ref<'app' | 'site'>('app')
const catTarget = ref('')
const catName = ref('')
const catSuggestions = ref<(KnownApp | KnownSite)[]>([])

async function refreshCats() { cats.value = await listCustomCategories() }
async function refreshKnown() {
  knownApps.value = await getKnownApps()
  knownSites.value = await getKnownSites()
}
function suggestCat(q: string) {
  const s = q.trim().toLowerCase()
  const pool: (KnownApp | KnownSite)[] = catKind.value === 'app' ? knownApps.value : knownSites.value
  catSuggestions.value = s
    ? pool.filter(a => ('id' in a ? a.id : a.site).toLowerCase().includes(s) || a.display.toLowerCase().includes(s)).slice(0, 8)
    : []
}
function pickCatSuggestion(a: KnownApp | KnownSite) {
  catTarget.value = 'id' in a ? a.id : a.site
  catSuggestions.value = []
}
async function addCat() {
  if (!catTarget.value.trim() || !catName.value.trim()) return
  await addCustomCategory(catKind.value, catTarget.value.trim(), catName.value.trim())
  catTarget.value = ''; catName.value = ''
  await refreshCats()
}

const editCatId = ref<number | null>(null)
const editCatName = ref('')
function startEditCat(c: CustomCategory) { editCatId.value = c.id; editCatName.value = c.display_name }
async function saveEditCat() {
  if (editCatId.value == null || !editCatName.value.trim()) return
  const c = cats.value.find(x => x.id === editCatId.value)
  if (c) {
    const oldLimit = limitOf('category', c.display_name)
    await addCustomCategory(c.kind, c.target, editCatName.value.trim())
    if (oldLimit !== undefined) {
      await removeLimit(c.display_name)
      await setLimit(editCatName.value.trim(), 'category', oldLimit)
    }
  }
  editCatId.value = null
  await refreshCats()
  emit('changed')
}
async function delCat(c: CustomCategory) {
  await removeLimit(c.display_name).catch(() => {})
  await removeCustomCategory(c.id)
  await refreshCats()
  emit('changed')
}

const catLimitInput = ref<Record<number, number>>({})
async function setCatLimit(c: CustomCategory) {
  const m = catLimitInput.value[c.id]
  if (!m) return
  await setLimit(c.display_name, 'category', m)
  delete catLimitInput.value[c.id]
  emit('changed')
}
async function clearLimit(target: string) {
  await removeLimit(target)
  emit('changed')
}

// ── التطبيقات والمواقع ───────────────────────────────
const tab = ref<'apps' | 'sites'>('apps')
const q = ref('')
const ignored = ref(new Set<string>()) // `kind:target`
async function refreshIgnored() {
  const rows = await listIgnored()
  ignored.value = new Set(rows.map(([k, trg]) => `${k}:${trg}`))
}
function isIgnored(kind: 'app' | 'site', target: string) {
  return ignored.value.has(`${kind}:${target}`)
}

const filteredApps = computed(() => {
  const s = q.value.trim().toLowerCase()
  return knownApps.value.filter(a => !s || a.id.toLowerCase().includes(s) || a.display.toLowerCase().includes(s))
})
const filteredSites = computed(() => {
  const s = q.value.trim().toLowerCase()
  return knownSites.value.filter(x => !s || x.site.toLowerCase().includes(s) || x.display.toLowerCase().includes(s))
})

const editingApp = ref<string | null>(null)
const editAppName = ref('')
const editingSite = ref<string | null>(null)
const editSiteName = ref('')
function startEditApp(a: KnownApp) { editingApp.value = a.id; editAppName.value = a.display }
async function saveEditApp() {
  if (!editingApp.value || !editAppName.value.trim()) return
  await setNameOverride(editingApp.value, editAppName.value.trim())
  editingApp.value = null
  await refreshKnown()
  emit('changed')
}
function startEditSite(x: KnownSite) { editingSite.value = x.site; editSiteName.value = x.display }
async function saveEditSite() {
  if (!editingSite.value || !editSiteName.value.trim()) return
  await setSiteOverride(editingSite.value, editSiteName.value.trim())
  editingSite.value = null
  await refreshKnown()
  emit('changed')
}
async function revertApp(a: KnownApp) { await removeNameOverride(a.id); await refreshKnown(); emit('changed') }
async function revertSite(x: KnownSite) { await removeSiteOverride(x.site); await refreshKnown(); emit('changed') }

const rowCat = ref<{ kind: 'app' | 'site'; target: string } | null>(null)
const rowCatName = ref('')
async function saveRowCat() {
  if (!rowCat.value || !rowCatName.value.trim()) return
  await addCustomCategory(rowCat.value.kind, rowCat.value.target, rowCatName.value.trim())
  rowCat.value = null
  rowCatName.value = ''
  await refreshCats()
}

async function removeTarget(kind: 'app' | 'site', target: string) {
  await ignoreTarget(kind, target)
  await Promise.all([refreshIgnored(), refreshKnown()])
}
async function restoreTarget(kind: 'app' | 'site', target: string) {
  await unignoreTarget(kind, target)
  await Promise.all([refreshIgnored(), refreshKnown()])
}

const appLimitInput = ref<Record<string, number>>({})
const siteLimitInput = ref<Record<string, number>>({})
async function setAppLimit(a: KnownApp) {
  const m = appLimitInput.value[a.id]
  if (!m) return
  await setLimit(a.id, 'app', m)
  delete appLimitInput.value[a.id]
  emit('changed')
}
async function setSiteLimit(x: KnownSite) {
  const m = siteLimitInput.value[x.site]
  if (!m) return
  await setLimit(x.site, 'site', m)
  delete siteLimitInput.value[x.site]
  emit('changed')
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

    <div class="s-body">
      <section class="card">
        <h3>{{ t('settings.section.general') }}</h3>
        <div class="row">
          <div>
            <h4>{{ t('settings.language.label') }}</h4>
            <div class="theme-toggle">
              <button class="pill" :class="{ on: lang === 'auto' }" @click="setLang('auto')">{{ t('settings.language.auto') }}</button>
              <button class="pill" :class="{ on: lang === 'ar' }" @click="setLang('ar')">العربية</button>
              <button class="pill" :class="{ on: lang === 'en' }" @click="setLang('en')">English</button>
            </div>
          </div>
          <div>
            <h4>{{ t('settings.section.theme') }}</h4>
            <div class="theme-toggle">
              <button class="pill" :class="{ on: theme === 'system' }" @click="toggleTheme('system')">{{ t('settings.theme.system') }}</button>
              <button class="pill" :class="{ on: theme === 'light' }" @click="toggleTheme('light')">{{ t('settings.theme.light') }}</button>
              <button class="pill" :class="{ on: theme === 'dark' }" @click="toggleTheme('dark')">{{ t('settings.theme.dark') }}</button>
            </div>
          </div>
        </div>
      </section>

      <section class="card s-cats">
        <h3>{{ t('settings.section.categories') }}</h3>
        <p class="hint">{{ t('settings.categories.hint') }}</p>
        <div class="add-form">
          <select v-model="catKind">
            <option value="app">{{ t('settings.categories.kind.app') }}</option>
            <option value="site">{{ t('settings.categories.kind.site') }}</option>
          </select>
          <div class="suggest-wrap">
            <input v-model="catTarget" :placeholder="t('settings.categories.placeholder.target')" @input="suggestCat(catTarget)" />
            <ul v-if="catSuggestions.length" class="suggest">
              <li v-for="a in catSuggestions" :key="'id' in a ? a.id : a.site" @mousedown.prevent="pickCatSuggestion(a)">
                <code>{{ 'id' in a ? a.id : a.site }}</code> ← {{ a.display }}
              </li>
            </ul>
          </div>
          <input v-model="catName" :placeholder="t('settings.categories.placeholder.name')" />
          <button class="btn primary" @click="addCat">{{ t('settings.categories.add') }}</button>
        </div>

        <div v-for="c in cats" :key="c.id" class="cat-row">
          <span class="pill small">{{ t('settings.categories.kind.' + c.kind) }}</span>
          <code class="owid">{{ c.target }}</code>
          <span class="arrow">→</span>
          <template v-if="editCatId === c.id">
            <input v-model="editCatName" class="edit-input" />
            <button class="btn primary small" @click="saveEditCat">✓</button>
            <button class="btn ghost small" @click="editCatId = null">✕</button>
          </template>
          <template v-else>
            <b class="cname">{{ c.display_name }}</b>
            <button class="btn ghost small" @click="startEditCat(c)">{{ t('settings.categories.rename') }}</button>
          </template>
          <div class="row-limit">
            <input v-model.number="catLimitInput[c.id]" type="number" min="1" class="limit-input" :placeholder="t('settings.categories.limitPlaceholder')" />
            <button class="btn ghost small" @click="setCatLimit(c)">{{ t('settings.categories.setLimit') }}</button>
            <template v-if="limitOf('category', c.display_name) !== undefined">
              <span class="lused" :class="{ over: usedOf('category', c.display_name) > (limitOf('category', c.display_name) ?? 0) }">
                {{ t('settings.limits.used', { used: usedOf('category', c.display_name), max: limitOf('category', c.display_name) }) }}
              </span>
              <button class="btn ghost small" @click="clearLimit(c.display_name)">{{ t('settings.categories.clearLimit') }}</button>
            </template>
          </div>
          <button class="btn ghost small danger" @click="delCat(c)">{{ t('settings.categories.delete') }}</button>
        </div>
        <div v-if="cats.length === 0" class="empty">{{ t('settings.categories.empty') }}</div>
      </section>

      <section class="card s-lists">
        <h3>{{ t('settings.section.lists') }}</h3>
        <div class="tabs">
          <button class="pill" :class="{ on: tab === 'apps' }" @click="tab = 'apps'">{{ t('settings.lists.tab.apps') }}</button>
          <button class="pill" :class="{ on: tab === 'sites' }" @click="tab = 'sites'">{{ t('settings.lists.tab.sites') }}</button>
        </div>
        <input v-model="q" class="list-search" :placeholder="t('settings.lists.search')" />

        <template v-if="tab === 'apps'">
          <div v-for="a in filteredApps" :key="a.id" class="item-row" :class="{ ignored: isIgnored('app', a.id) }">
            <div class="item-main">
              <code class="owid">{{ a.id }}</code>
              <template v-if="editingApp === a.id">
                <input v-model="editAppName" class="edit-input" />
                <button class="btn primary small" @click="saveEditApp">✓</button>
                <button class="btn ghost small" @click="editingApp = null">✕</button>
              </template>
              <template v-else>
                <b>{{ a.display }}</b>
                <span v-if="isIgnored('app', a.id)" class="ignored-tag">{{ t('settings.lists.ignored') }}</span>
              </template>
            </div>
            <div class="item-actions">
              <template v-if="!isIgnored('app', a.id)">
                <button class="btn ghost small" @click="startEditApp(a)">{{ t('settings.lists.rename') }}</button>
                <button v-if="a.overridden" class="btn ghost small" @click="revertApp(a)">{{ t('settings.lists.revert') }}</button>
                <template v-if="rowCat && rowCat.kind === 'app' && rowCat.target === a.id">
                  <input v-model="rowCatName" class="edit-input narrow" :placeholder="t('settings.lists.categoryPlaceholder')" />
                  <button class="btn primary small" @click="saveRowCat">✓</button>
                  <button class="btn ghost small" @click="rowCat = null">✕</button>
                </template>
                <button v-else class="btn ghost small" @click="rowCat = { kind: 'app', target: a.id }">{{ t('settings.lists.addToCategory') }}</button>
                <input v-model.number="appLimitInput[a.id]" type="number" min="1" class="limit-input" :placeholder="t('settings.lists.limitPlaceholder')" />
                <button class="btn ghost small" @click="setAppLimit(a)">{{ t('settings.lists.setLimit') }}</button>
                <template v-if="limitOf('app', a.id) !== undefined">
                  <span class="lused" :class="{ over: usedOf('app', a.id) > (limitOf('app', a.id) ?? 0) }">
                    {{ t('settings.limits.used', { used: usedOf('app', a.id), max: limitOf('app', a.id) }) }}
                  </span>
                  <button class="btn ghost small" @click="clearLimit(a.id)">{{ t('settings.lists.clearLimit') }}</button>
                </template>
                <button class="btn ghost small danger" @click="removeTarget('app', a.id)">{{ t('settings.lists.remove') }}</button>
              </template>
              <button v-else class="btn ghost small" @click="restoreTarget('app', a.id)">{{ t('settings.lists.restore') }}</button>
            </div>
          </div>
          <div v-if="filteredApps.length === 0" class="empty">
            {{ knownApps.length ? t('settings.lists.noResults') : t('settings.lists.empty.apps') }}
          </div>
        </template>

        <template v-else>
          <div v-for="x in filteredSites" :key="x.site" class="item-row" :class="{ ignored: isIgnored('site', x.site) }">
            <div class="item-main">
              <code class="owid">{{ x.site }}</code>
              <template v-if="editingSite === x.site">
                <input v-model="editSiteName" class="edit-input" />
                <button class="btn primary small" @click="saveEditSite">✓</button>
                <button class="btn ghost small" @click="editingSite = null">✕</button>
              </template>
              <template v-else>
                <b>{{ x.display }}</b>
                <span v-if="isIgnored('site', x.site)" class="ignored-tag">{{ t('settings.lists.ignored') }}</span>
              </template>
            </div>
            <div class="item-actions">
              <template v-if="!isIgnored('site', x.site)">
                <button class="btn ghost small" @click="startEditSite(x)">{{ t('settings.lists.rename') }}</button>
                <button v-if="x.overridden" class="btn ghost small" @click="revertSite(x)">{{ t('settings.lists.revert') }}</button>
                <template v-if="rowCat && rowCat.kind === 'site' && rowCat.target === x.site">
                  <input v-model="rowCatName" class="edit-input narrow" :placeholder="t('settings.lists.categoryPlaceholder')" />
                  <button class="btn primary small" @click="saveRowCat">✓</button>
                  <button class="btn ghost small" @click="rowCat = null">✕</button>
                </template>
                <button v-else class="btn ghost small" @click="rowCat = { kind: 'site', target: x.site }">{{ t('settings.lists.addToCategory') }}</button>
                <input v-model.number="siteLimitInput[x.site]" type="number" min="1" class="limit-input" :placeholder="t('settings.lists.limitPlaceholder')" />
                <button class="btn ghost small" @click="setSiteLimit(x)">{{ t('settings.lists.setLimit') }}</button>
                <template v-if="limitOf('site', x.site) !== undefined">
                  <span class="lused" :class="{ over: usedOf('site', x.site) > (limitOf('site', x.site) ?? 0) }">
                    {{ t('settings.limits.used', { used: usedOf('site', x.site), max: limitOf('site', x.site) }) }}
                  </span>
                  <button class="btn ghost small" @click="clearLimit(x.site)">{{ t('settings.lists.clearLimit') }}</button>
                </template>
                <button class="btn ghost small danger" @click="removeTarget('site', x.site)">{{ t('settings.lists.remove') }}</button>
              </template>
              <button v-else class="btn ghost small" @click="restoreTarget('site', x.site)">{{ t('settings.lists.restore') }}</button>
            </div>
          </div>
          <div v-if="filteredSites.length === 0" class="empty">
            {{ knownSites.length ? t('settings.lists.noResults') : t('settings.lists.empty.sites') }}
          </div>
        </template>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-page { display: flex; flex-direction: column; gap: 0.9rem; flex: 1; min-height: 0; }
.s-head { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 0 0.2rem; }
.s-title { font-size: 1.05rem; font-weight: 900; }
.s-body { flex: 1; display: flex; flex-direction: column; gap: 14px; min-width: 0; overflow-y: auto; padding-inline-end: 0.3rem; }
.s-body .card { padding: 1.1rem 1.2rem; max-width: 760px; }
.s-body .card h3 { font-size: 1rem; margin-bottom: 0.7rem; }
.s-body .card h4 { font-size: 0.85rem; font-weight: 800; margin-bottom: 0.4rem; color: var(--ink-muted); }
.row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.theme-toggle { display: flex; gap: 5px; flex-wrap: wrap; }
.hint { color: var(--ink-muted); font-size: 0.8rem; margin-bottom: 0.7rem; }
.add-form { display: flex; gap: 0.5rem; margin-bottom: 0.7rem; flex-wrap: wrap; }
.add-form input, .add-form select {
  background: var(--surface-soft); border: 1px solid var(--border); border-radius: 8px;
  padding: 0.45rem 0.6rem; color: var(--ink); font-family: inherit; font-size: 0.85rem; min-width: 0;
}
.add-form input { flex: 1 1 180px; }
.suggest-wrap { position: relative; flex: 1 1 180px; }
.suggest-wrap input { width: 100%; }
.suggest {
  position: absolute; top: calc(100% + 4px); left: 0; right: 0; z-index: 10;
  list-style: none; margin: 0; padding: 0.3rem; background: var(--surface-soft);
  border: 1px solid var(--border); border-radius: 8px; box-shadow: var(--shadow-sm);
  max-height: 200px; overflow-y: auto;
}
.suggest li {
  padding: 0.35rem 0.55rem; border-radius: 6px; cursor: pointer;
  font-size: 0.82rem; color: var(--ink); display: flex; gap: 0.4rem; align-items: center;
}
.suggest li:hover { background: var(--accent); color: var(--bg); }
.suggest code { font-size: 0.75rem; opacity: 0.8; }
.cat-row, .item-row {
  display: flex; gap: 0.6rem; align-items: center; padding: 0.55rem 0;
  border-bottom: 1px solid var(--border); flex-wrap: wrap;
}
.cat-row > code { font-size: 0.78rem; }
.cname { flex: 1; min-width: 0; }
.row-limit { display: flex; gap: 0.4rem; align-items: center; }
.limit-input {
  width: 76px; background: var(--surface-soft); border: 1px solid var(--border); border-radius: 8px;
  padding: 0.3rem 0.5rem; color: var(--ink); font-family: inherit; font-size: 0.82rem;
}
.tabs { display: flex; gap: 5px; margin-bottom: 0.6rem; }
.list-search {
  width: 100%; background: var(--surface-soft); border: 1px solid var(--border);
  border-radius: 8px; padding: 0.4rem 0.6rem; color: var(--ink); font-family: inherit;
  font-size: 0.85rem; margin-bottom: 0.4rem;
}
.item-row { align-items: flex-start; flex-direction: column; gap: 0.4rem; }
.item-row.ignored { opacity: 0.6; }
.item-main { display: flex; gap: 0.6rem; align-items: center; width: 100%; min-width: 0; }
.item-main b { flex: 1; min-width: 0; }
.item-actions { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.owid { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.78rem; }
.edit-input { background: var(--surface-soft); border: 1px solid var(--border); border-radius: 8px;
  padding: 0.3rem 0.5rem; color: var(--ink); font-family: inherit; font-size: 0.85rem; flex: 1; min-width: 120px; }
.edit-input.narrow { flex: 0 1 140px; }
.ignored-tag { color: var(--ink-muted); font-size: 0.72rem; font-weight: 700; }
.lused { color: var(--ink-muted); font-size: 0.82rem; }
.lused.over { color: var(--danger); font-weight: 700; }
.btn.small { padding: 0.25rem 0.7rem; font-size: 0.75rem; }
.btn.danger { color: var(--danger); border-color: var(--danger); }
.pill.small { font-size: 0.7rem; padding: 0.15rem 0.55rem; }
.arrow { color: var(--ink-muted); }
.empty { color: var(--ink-muted); font-size: 0.85rem; padding: 0.6rem 0; }
</style>
