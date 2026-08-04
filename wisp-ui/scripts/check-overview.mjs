import assert from 'node:assert'
import { weekStart, weeksCount, sameDowSum, hourScale, weekRangeLabel } from '../src/lib/overview.ts'

const DAY = 86400
const now = new Date()
now.setHours(12, 0, 0, 0) // راسِ على الظهر: لا يقطع بداية الأسبوع مهما كانت الساعة الحالية
const nowSec = Math.floor(now.getTime() / 1000)

const logs = [{ start_time: nowSec - 3600 }, { start_time: nowSec - 7200 }]
assert.equal(weeksCount(logs), 1, 'a1: أسبوع واحد ببيانات يوم واحد')

const eight = Array.from({ length: 8 }, (_, i) => ({ start_time: nowSec - i * 7 * DAY }))
assert.equal(weeksCount(eight), 8, 'a2: 8 أسابيع متميزة')
assert.equal(weeksCount([...eight, { start_time: nowSec - 9 * 7 * DAY }]), 8, 'a3: سقف 8')

const sat = new Date(2026, 6, 25)
const mixed = [
  { start_time: new Date(2026, 6, 25, 10).getTime() / 1000, duration: 3600 },
  { start_time: new Date(2026, 6, 26, 10).getTime() / 1000, duration: 7200 },
  { start_time: new Date(2026, 6, 25, 15).getTime() / 1000, duration: 1800 },
]
assert.equal(sameDowSum(mixed, sat), 5400, 'a4: نفس يوم الأسبوع المحلي فقط')
assert.equal(sameDowSum(mixed, new Date(2026, 6, 26)), 7200, 'a5: الأحد وحده')

assert.equal(hourScale(2.4, 1.1), 3, 'a6: سقف يشمل الأعمدة')
assert.equal(hourScale(0.2, 1.7), 2, 'a7: سقف يشمل المتوسط')
assert.equal(hourScale(0, 0), 1, 'a8: حد أدنى 1')

assert.equal(weekRangeLabel(new Date(2026, 6, 25), new Date(2026, 6, 31), true), 'هذا الأسبوع', 'a9: الحالي')
assert.equal(weekRangeLabel(new Date(2026, 6, 25), new Date(2026, 6, 31), false), '25 – 31 يوليو', 'a10: شهر مشترك')
assert.equal(weekRangeLabel(new Date(2026, 5, 29), new Date(2026, 6, 5), false), '29 يونيو – 5 يوليو', 'a11: شهران')

assert.equal(weekStart(new Date(2026, 6, 25, 10).getTime() / 1000), new Date(2026, 6, 25).getTime(), 'a12: بداية الأسبوع')

console.log('all overview checks passed')
