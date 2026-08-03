<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  setLimit, removeLimit, eventDuration,
  setNameOverride, removeNameOverride,
  getSetting, setSetting,
  getKnownApps, getKnownSites, setSiteOverride, removeSiteOverride,
  listIgnored, ignoreTarget, unignoreTarget,
  archiveTarget, unarchiveTarget, getArchived,
  getCategories, getCategoryMembers, addCategory, renameCategory, setCategoryColor,
  addCategoryMember, deleteCategoryMember, deleteCategory, setCategoryCache,
  type LogEntry, type KnownApp, type KnownSite, type CategoryInfo,
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
  await Promise.all([refreshKnown(), refreshIgnored(), refreshCategories(), refreshArchived()])
})
async function setLang(v: 'auto' | 'ar' | 'en') {
  lang.value = v
  await setSetting('language', v)
  setLocale(v)
}

const usedMap = computed(() => {
  const m = new Map<string, number>()
  for (const l of props.todayLogs) {
    if (l.event_type === 'system') continue
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

const knownApps = ref<KnownApp[]>([])
const knownSites = ref<KnownSite[]>([])
async function refreshKnown() {
  knownApps.value = await getKnownApps()
  knownSites.value = await getKnownSites()
}
async function clearLimit(target: string) {
  await removeLimit(target)
  emit('changed')
}

// ── الفئات ─────────────────────────────────────────────
const cats = ref<CategoryInfo[]>([])
const openMembers = ref<number | null>(null)
const membersOf = ref<Record<number, { kind: string; target: string }[]>>({})
const editCatName = ref('')
const editingCat = ref<number | null>(null)
const memberKind = ref<'app' | 'site'>('app')
const memberTarget = ref('')
const addCatName = ref('')
const addCatColor = ref('#8a7f6e')
const showAddCat = ref(false)

async function refreshCategories() {
  cats.value = await getCategories()
  setCategoryCache(cats.value)
  await Promise.all(cats.value.map(async c => {
    membersOf.value[c.id] = (await getCategoryMembers(c.id)).map(([k, t]) => ({ kind: k, target: t }))
  }))
}
function startEditCat(c: CategoryInfo) { editCatName.value = c.name; editingCat.value = c.id }
async function saveCatName(c: CategoryInfo) {
  const name = editCatName.value.trim()
  if (!name) return
  await renameCategory(c.id, name)
  await refreshCategories()
  editingCat.value = null
}
async function setCatColor(c: CategoryInfo, color: string) {
  await setCategoryColor(c.id, color)
  await refreshCategories()
}
function toggleMembers(id: number) { openMembers.value = openMembers.value === id ? null : id }
async function addMember(c: CategoryInfo) {
  const target = memberTarget.value.trim()
  if (!target) return
  await addCategoryMember(c.id, memberKind.value, target)
  memberTarget.value = ''
  await refreshCategories()
}
async function delMember(c: CategoryInfo, m: { kind: string; target: string }) {
  await deleteCategoryMember(m.kind, m.target)
  await refreshCategories()
}
async function delCat(c: CategoryInfo) {
  await deleteCategory(c.id)
  await refreshCategories()
}
async function addNewCategory() {
  const name = addCatName.value.trim()
  if (!name) return
  await addCategory(name, addCatColor.value)
  addCatName.value = ''
  showAddCat.value = false
  await refreshCategories()
}

// ── التطبيقات والمواقع ───────────────────────────────
const tab = ref<'apps' | 'sites'>('apps')
const q = ref('')
const ignored = ref(new Set<string>()) // `kind:target` (المواقع بحروف صغيرة)
async function refreshIgnored() {
  const rows = await listIgnored()
  ignored.value = new Set(rows.map(([k, trg]) => `${k}:${k === 'site' ? trg.toLowerCase() : trg}`))
}
function isIgnored(kind: 'app' | 'site', target: string) {
  return ignored.value.has(`${kind}:${kind === 'site' ? target.toLowerCase() : target}`)
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

const archived = ref<[string, string][]>([])
const archivedOpen = ref(false)
async function refreshArchived() {
  archived.value = await getArchived()
}
async function archiveTarget2(kind: 'app' | 'site', target: string) {
  await archiveTarget(kind, target)
  await Promise.all([refreshArchived(), refreshKnown()])
}
async function restoreArchived(kind: 'app' | 'site', target: string) {
  await unarchiveTarget(kind, target)
  await Promise.all([refreshArchived(), refreshKnown()])
}
function toggleArchived() { archivedOpen.value = !archivedOpen.value }
const archivedApps = computed(() => archived.value.filter(([k]) => k === 'app'))
const archivedSites = computed(() => archived.value.filter(([k]) => k === 'site'))
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
      <svg class="s-logo" viewBox="0 0 24 24" width="20" height="20" aria-hidden="true"><defs><linearGradient id="rod-g2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ff8fb2"/><stop offset="1" stop-color="#e13057"/></linearGradient></defs><g transform="rotate(-12 12 12)"><g fill="none" stroke="#ff8fb2" stroke-width="0.7" stroke-linecap="round"><path d="M16.6 10.2c1.3-.5 2.2.2 3.4-.2"/><path d="M16.6 12c1.4-.2 2.3.5 3.7.1"/><path d="M16.6 13.8c1.3.4 2.2-.2 3.4.5"/></g><rect x="1.6" y="6.9" width="21.2" height="10.2" rx="5.1" fill="#fff" opacity=".9"/><rect x="2.6" y="7.9" width="19.2" height="8.2" rx="4.1" fill="url(#rod-g2)"/><circle cx="12" cy="12" r="4.2" fill="#faf6ef"/><g stroke="#e94560" stroke-width="0.6" stroke-linecap="round"><line x1="12" y1="7.8" x2="12" y2="9"/><line x1="16.2" y1="12" x2="15" y2="12"/><line x1="12" y1="16.2" x2="12" y2="15"/><line x1="7.8" y1="12" x2="9" y2="12"/></g><g stroke="#e94560" stroke-width="0.7" stroke-linecap="round"><line x1="12" y1="12" x2="13.9" y2="10.9"/><line x1="12" y1="12" x2="10.7" y2="11.3"/></g><circle cx="12" cy="12" r="0.8" fill="#e94560"/></g></svg>
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
        <p class="hint warn">{{ t('settings.categories.reclassifyNotice') }}</p>

        <div class="add-form">
          <button class="btn primary small" @click="showAddCat = !showAddCat">
            {{ t('settings.categories.new') }}
          </button>
          <template v-if="showAddCat">
            <input v-model="addCatName" class="edit-input" :placeholder="t('settings.categories.name')" />
            <input v-model="addCatColor" type="color" class="color-dot" />
            <button class="btn primary small" @click="addNewCategory">✓</button>
          </template>
        </div>

        <div v-for="c in cats" :key="c.id" class="cat-row">
          <input type="color" :value="c.color" class="color-dot"
                 @input="(e) => setCatColor(c, (e.target as HTMLInputElement).value)" />
          <template v-if="editingCat === c.id">
            <input v-model="editCatName" class="edit-input" />
            <button class="btn primary small" @click="saveCatName(c)">✓</button>
            <button class="btn ghost small" @click="editingCat = null">✕</button>
          </template>
          <template v-else>
            <b class="cname">{{ c.name }}</b>
            <button class="btn ghost small" @click="startEditCat(c)">{{ t('settings.categories.rename') }}</button>
          </template>
          <div class="row-limit">
            <button class="btn ghost small" @click="toggleMembers(c.id)">
              {{ t('settings.categories.members') }} ({{ membersOf[c.id]?.length ?? 0 }})
            </button>
            <button v-if="c.is_deletable === 1" class="btn ghost small danger" @click="delCat(c)">
              {{ t('settings.categories.delete') }}
            </button>
          </div>
          <div v-if="openMembers === c.id" class="member-panel">
            <div class="m-head">
              <select v-model="memberKind">
                <option value="app">{{ t('settings.categories.kind.app') }}</option>
                <option value="site">{{ t('settings.categories.kind.site') }}</option>
              </select>
              <input v-model="memberTarget" list="member-suggestions"
                     :placeholder="t('settings.categories.placeholder.target')" />
              <datalist id="member-suggestions">
                <option v-for="a in knownApps" :key="a.id" :value="a.id" />
                <option v-for="x in knownSites" :key="x.site" :value="x.site" />
              </datalist>
              <button class="btn primary small" @click="addMember(c)">{{ t('settings.categories.addMember') }}</button>
            </div>
            <ul v-if="(membersOf[c.id] || []).length" class="m-list">
              <li v-for="m in membersOf[c.id]" :key="m.kind + m.target">
                <span class="pill small">{{ m.kind }}</span>
                <code>{{ m.target }}</code>
                <button class="btn ghost small danger" @click="delMember(c, m)">🗑</button>
              </li>
            </ul>
          </div>
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
                <input v-model.number="appLimitInput[a.id]" type="number" min="1" class="limit-input" :placeholder="t('settings.lists.limitPlaceholder')" />
                <button class="btn ghost small" @click="setAppLimit(a)">{{ t('settings.lists.setLimit') }}</button>
                <template v-if="limitOf('app', a.id) !== undefined">
                  <span class="lused" :class="{ over: usedOf('app', a.id) > (limitOf('app', a.id) ?? 0) }">
                    {{ t('settings.limits.used', { used: usedOf('app', a.id), max: limitOf('app', a.id) }) }}
                  </span>
                  <button class="btn ghost small" @click="clearLimit(a.id)">{{ t('settings.lists.clearLimit') }}</button>
                </template>
                <button class="btn ghost small" @click="archiveTarget2('app', a.id)">{{ t('settings.lists.archive') }}</button>
              </template>
              <template v-else>
                <button class="btn ghost small" @click="restoreTarget('app', a.id)">{{ t('settings.lists.restore') }}</button>
                <template v-if="limitOf('app', a.id) !== undefined">
                  <span class="lused">{{ t('settings.limits.used', { used: usedOf('app', a.id), max: limitOf('app', a.id) }) }}</span>
                  <button class="btn ghost small" @click="clearLimit(a.id)">{{ t('settings.lists.clearLimit') }}</button>
                </template>
              </template>
            </div>
          </div>
          <div v-if="filteredApps.length === 0" class="empty">
            {{ knownApps.length ? t('settings.lists.noResults') : t('settings.lists.empty.apps') }}
          </div>
          <div class="archived-block">
            <button class="archived-head" @click="toggleArchived">
              <span class="chevron" :class="{ open: archivedOpen }">▸</span>
              {{ t('settings.lists.archivedSection') }} ({{ archivedApps.length }})
            </button>
            <div v-if="archivedOpen" class="archived-list">
              <div v-for="[kind, target] in archivedApps" :key="target" class="item-row">
                <div class="item-main">
                  <code class="owid">{{ target }}</code>
                  <b>{{ target }}</b>
                </div>
                <div class="item-actions">
                  <button class="btn ghost small" @click="restoreArchived('app', target)">{{ t('settings.lists.restore') }}</button>
                </div>
              </div>
              <div v-if="archivedApps.length === 0" class="empty">{{ t('settings.lists.emptyArchived') }}</div>
            </div>
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
                <input v-model.number="siteLimitInput[x.site]" type="number" min="1" class="limit-input" :placeholder="t('settings.lists.limitPlaceholder')" />
                <button class="btn ghost small" @click="setSiteLimit(x)">{{ t('settings.lists.setLimit') }}</button>
                <template v-if="limitOf('site', x.site) !== undefined">
                  <span class="lused" :class="{ over: usedOf('site', x.site) > (limitOf('site', x.site) ?? 0) }">
                    {{ t('settings.limits.used', { used: usedOf('site', x.site), max: limitOf('site', x.site) }) }}
                  </span>
                  <button class="btn ghost small" @click="clearLimit(x.site)">{{ t('settings.lists.clearLimit') }}</button>
                </template>
                <button class="btn ghost small" @click="archiveTarget2('site', x.site)">{{ t('settings.lists.archive') }}</button>
              </template>
              <template v-else>
                <button class="btn ghost small" @click="restoreTarget('site', x.site)">{{ t('settings.lists.restore') }}</button>
                <template v-if="limitOf('site', x.site) !== undefined">
                  <span class="lused">{{ t('settings.limits.used', { used: usedOf('site', x.site), max: limitOf('site', x.site) }) }}</span>
                  <button class="btn ghost small" @click="clearLimit(x.site)">{{ t('settings.lists.clearLimit') }}</button>
                </template>
              </template>
            </div>
          </div>
          <div v-if="filteredSites.length === 0" class="empty">
            {{ knownSites.length ? t('settings.lists.noResults') : t('settings.lists.empty.sites') }}
          </div>
          <div class="archived-block">
            <button class="archived-head" @click="toggleArchived">
              <span class="chevron" :class="{ open: archivedOpen }">▸</span>
              {{ t('settings.lists.archivedSection') }} ({{ archivedSites.length }})
            </button>
            <div v-if="archivedOpen" class="archived-list">
              <div v-for="[kind, target] in archivedSites" :key="target" class="item-row">
                <div class="item-main">
                  <code class="owid">{{ target }}</code>
                  <b>{{ target }}</b>
                </div>
                <div class="item-actions">
                  <button class="btn ghost small" @click="restoreArchived('site', target)">{{ t('settings.lists.restore') }}</button>
                </div>
              </div>
              <div v-if="archivedSites.length === 0" class="empty">{{ t('settings.lists.emptyArchived') }}</div>
            </div>
          </div>
        </template>
      </section>

    </div>
  </div>
</template>

<style scoped>
.settings-page { display: flex; flex-direction: column; gap: 0.9rem; flex: 1; min-height: 0; }
.s-head { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 0 0.2rem; }
.s-logo { flex-shrink: 0; }
.s-title { font-size: 1.05rem; font-weight: 900; }
.s-body { flex: 1; display: flex; flex-direction: column; gap: 14px; min-width: 0; overflow-y: auto; padding-inline-end: 0.3rem; }
.s-body .card { padding: 1.1rem 1.2rem; width: 100%; }
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
  border-bottom: 1px solid var(--border);
}
.cat-row > code { font-size: 0.78rem; }
.color-dot { width: 26px; height: 26px; padding: 0; border: 1px solid var(--border); border-radius: 6px; background: none; cursor: pointer; flex-shrink: 0; }
.cname { flex: 1; min-width: 0; }
.cat-row .row-limit { margin-inline-start: auto; }
.row-limit { display: flex; gap: 0.4rem; align-items: center; }
.member-panel { flex-basis: 100%; display: flex; flex-direction: column; gap: 0.5rem; padding: 0.6rem 0.8rem; border: 1px solid var(--border); border-radius: 8px; background: var(--surface-soft); }
.m-head { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
.m-head select, .m-head input { background: var(--surface-soft); border: 1px solid var(--border); border-radius: 8px; padding: 0.4rem 0.55rem; color: var(--ink); font-family: inherit; font-size: 0.82rem; }
.m-head input { flex: 1 1 180px; }
.m-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.35rem; }
.m-list li { display: flex; gap: 0.5rem; align-items: center; font-size: 0.82rem; }
.m-list code { background: var(--surface-soft); border: 1px solid var(--border); border-radius: 6px; padding: 0.2rem 0.45rem; font-size: 0.75rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0; }
.hint.warn { color: var(--danger); font-weight: 600; }
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
.item-row { align-items: center; justify-content: space-between; gap: 0.8rem; }
.item-row.ignored { opacity: 0.6; }
.item-main { display: flex; gap: 0.6rem; align-items: center; flex: 1; min-width: 0; }
.item-main b { flex: 0 1 auto; }
.item-actions { display: flex; gap: 0.4rem; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
.owid { flex: 0 1 auto; max-width: 320px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.78rem; }
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
.archived-block { margin-top: 0.8rem; border-top: 1px solid var(--border); padding-top: 0.5rem; }
.archived-head { display: flex; align-items: center; gap: 0.4rem; background: none; border: none; color: var(--ink-muted); font-size: 0.85rem; font-weight: 700; cursor: pointer; padding: 0.2rem; }
.archived-head .chevron { transition: transform 150ms; font-size: 0.75rem; }
.archived-head .chevron.open { transform: rotate(90deg); }
.archived-list { display: flex; flex-direction: column; }
</style>
