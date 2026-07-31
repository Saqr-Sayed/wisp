export type Theme = 'light' | 'dark'

export function currentTheme(): Theme {
  return document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light'
}

export function setTheme(t: Theme) {
  document.documentElement.setAttribute('data-theme', t)
  localStorage.setItem('salmonella-theme', t)
}
