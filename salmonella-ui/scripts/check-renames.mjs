import assert from 'node:assert'
import fs from 'node:fs'

// r1: dbus.ts يصدّر الدوال والأنواع الجديدة
const dbts = fs.readFileSync('src/lib/dbus.ts', 'utf8')
for (const s of ['getKnownApps', 'getKnownSites', 'getSiteOverrides', 'setSiteOverride', 'removeSiteOverride', 'KnownApp', 'KnownSite']) {
  assert.ok(dbts.includes(s), 'r1: dbus.ts مفقود: ' + s)
}

// r2: db.rs يحتوي دوال السجل والمواقع
const dbrc = fs.readFileSync('../salmonella-core/src/db.rs', 'utf8')
for (const s of ['site_overrides', 'get_known_apps', 'get_known_sites', 'site_friendly_name', 'set_site_override', 'remove_site_override', 'apply_app_rename', 'site_friendly']) {
  assert.ok(dbrc.includes(s), 'r2: db.rs ناقص: ' + s)
}

// r3: dbus_api.rs يعرّضها
const api = fs.readFileSync('../daemon/src/dbus_api.rs', 'utf8')
for (const s of ['get_known_apps', 'get_known_sites', 'get_site_overrides', 'set_site_override', 'remove_site_override']) {
  assert.ok(api.includes(s), 'r3: dbus_api.rs ناقص: ' + s)
}

console.log('all rename checks passed')
