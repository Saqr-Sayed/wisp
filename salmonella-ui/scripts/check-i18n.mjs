import assert from 'node:assert'
import ar from '../src/i18n/ar.json' with { type: 'json' }
import en from '../src/i18n/en.json' with { type: 'json' }
import { t, locale, setLocale } from '../src/lib/i18n.ts'

const aKeys = Object.keys(ar).sort()
const eKeys = Object.keys(en).sort()

assert.equal(eKeys.length, aKeys.length, 'i4: تماثل عدد المفاتيح')
for (const k of aKeys) assert.ok(eKeys.includes(k), `i4: مفتاح ${k} مفقود في en`)
for (const k of eKeys) assert.ok(aKeys.includes(k), `i4: مفتاح ${k} مفقود في ar`)

setLocale('ar'); assert.equal(t('overview.today'), 'اليوم', 'i1: ar')
setLocale('en'); assert.equal(t('overview.today'), 'Today', 'i1: en')
setLocale('en'); assert.equal(t('nope.missing'), 'nope.missing', 'i2: مفتاح مجهول يُرجع نفسه')

setLocale('en'); assert.equal(t('settings.limits.used', { used: 3, max: 60 }), '3 / 60 min', 'i3: params')
setLocale('ar'); assert.equal(t('settings.limits.used', { used: 3, max: 60 }), '3 / 60 دقيقة', 'i3: ar params')
console.log('all i18n checks passed')
