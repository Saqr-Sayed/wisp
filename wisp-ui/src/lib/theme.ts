// ponytail: localStorage key deliberately kept as 'salmonella-theme' (internal,
// invisible to users; no migration on rebrand to Wisp). Do not rename.
export type ThemeMode = 'system' | 'light' | 'dark'

export function currentMode(): ThemeMode {
  const stored = localStorage.getItem('salmonella-theme')
  return stored === 'light' || stored === 'dark' ? stored : 'system'
}

function prefersDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

function apply() {
  document.documentElement.setAttribute('data-theme', currentMode() === 'system' ? (prefersDark() ? 'dark' : 'light') : currentMode())
}

export function setMode(m: ThemeMode) {
  localStorage.setItem('salmonella-theme', m)
  apply()
}

export function listenSystemTheme() {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (currentMode() === 'system') apply()
  })
}
