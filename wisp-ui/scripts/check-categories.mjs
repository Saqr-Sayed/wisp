import assert from 'node:assert'
import fs from 'node:fs'

// تحقق بنيوي: ملف dbus.ts يُصدّر الدوال الجديدة
const src = fs.readFileSync('src/lib/dbus.ts', 'utf8')
for (const s of ['listCustomCategories', 'addCustomCategory', 'removeCustomCategory', 'CustomCategory']) {
  assert.ok(src.includes(s), 'c1: دالة أو نوع مفقود: ' + s)
}

// تحقق بنيوي: db.rs يحتوي دوال التصنيفات
const dbrc = fs.readFileSync('../wisp-core/src/db.rs', 'utf8')
for (const s of ['custom_categories', 'list_custom_categories', 'add_custom_category', 'remove_custom_category', 'match_custom_category']) {
  assert.ok(dbrc.includes(s), 'c2: db.rs ناقص: ' + s)
}

console.log('all categories checks passed')
