import type { CustomTheme } from './api';

export interface Theme {
  name: string;
  bg: string;
  side: string;
  acc: string;
  txt: string;
  brd: string;
}

export const THEMES: Theme[] = [
  { name: 'Tokyonight', bg: '#1a1b26', side: '#16161e', acc: '#7aa2f7', txt: '#c0caf5', brd: '#414868' },
  { name: 'Catppuccin', bg: '#1e1e2e', side: '#181825', acc: '#cba6f7', txt: '#cdd6f4', brd: '#313244' },
  { name: 'One Dark', bg: '#282c34', side: '#21252b', acc: '#61afef', txt: '#abb2bf', brd: '#3e4451' },
  { name: 'Dracula', bg: '#282a36', side: '#21222c', acc: '#bd93f9', txt: '#f8f8f2', brd: '#44475a' },
  { name: 'Nord', bg: '#2e3440', side: '#272c36', acc: '#88c0d0', txt: '#eceff4', brd: '#3b4252' },
  { name: 'Gruvbox', bg: '#282828', side: '#1d2021', acc: '#fabd2f', txt: '#ebdbb2', brd: '#3c3836' },
  { name: 'Rose Pine', bg: '#191724', side: '#1f1d2e', acc: '#c4a7e7', txt: '#e0def4', brd: '#2a2739' },
  { name: 'Kanagawa', bg: '#1f1f28', side: '#16161d', acc: '#7e9cd8', txt: '#dcd7ba', brd: '#2a2a37' },
  { name: 'Everforest', bg: '#2d353b', side: '#272e33', acc: '#a7c080', txt: '#d3c6aa', brd: '#3d484d' },
  { name: 'Monokai', bg: '#272822', side: '#1e1f1c', acc: '#f92672', txt: '#f8f8f2', brd: '#3e3d32' },
  { name: 'Synthwave', bg: '#262335', side: '#1d1927', acc: '#ff7edb', txt: '#ffffff', brd: '#3b3557' },
  { name: 'Custom', bg: '#0d0d0f', side: '#111114', acc: '#534AB7', txt: '#e8e6e0', brd: '#2a2a2e' },
];

const UI_FONTS: Record<string, string> = {
  sans: '-apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif',
  inter: '"Inter", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
  geist: '"Geist", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
  plex: '"IBM Plex Sans", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
};

const CODE_FONTS: Record<string, string> = {
  mono: 'ui-monospace, "JetBrains Mono", "Fira Code", monospace',
  jetbrains: '"JetBrains Mono", ui-monospace, monospace',
  fira: '"Fira Code", ui-monospace, monospace',
  cascadia: '"Cascadia Code", ui-monospace, monospace',
};

export function applyTheme(t: Theme | CustomTheme): void {
  const root = document.documentElement.style;
  root.setProperty('--color-background-primary', t.bg);
  root.setProperty('--color-background-secondary', t.side);
  root.setProperty('--color-border-primary', t.brd);
  root.setProperty('--color-border-secondary', t.brd);
  root.setProperty('--color-border-tertiary', t.brd);
  root.setProperty('--color-text-primary', t.txt);
  root.setProperty('--color-text-secondary', t.txt);
  root.setProperty('--color-accent', t.acc);
  root.setProperty('--color-accent-soft', hexToRgba(t.acc, 0.08));
}

export function applyColorScheme(scheme: string): void {
  const root = document.documentElement.style;
  switch (scheme) {
    case 'light':
      root.setProperty('--color-background-primary', '#ffffff');
      root.setProperty('--color-background-secondary', '#f7f7f8');
      root.setProperty('--color-border-tertiary', '#e5e5e8');
      root.setProperty('--color-text-primary', '#1a1a1a');
      root.setProperty('--color-text-secondary', '#4a4a4a');
      root.setProperty('--color-text-tertiary', '#8a8a8a');
      break;
    case 'amoled':
      root.setProperty('--color-background-primary', '#000000');
      root.setProperty('--color-background-secondary', '#0a0a0a');
      root.setProperty('--color-border-tertiary', '#1a1a1a');
      break;
    case 'dark':
    case 'system':
    default:
      root.setProperty('--color-background-primary', '#1a1b26');
      root.setProperty('--color-background-secondary', '#16161e');
      root.setProperty('--color-border-tertiary', '#2f334d');
      root.setProperty('--color-text-primary', '#c0caf5');
      root.setProperty('--color-text-secondary', '#a9b1d6');
      root.setProperty('--color-text-tertiary', '#565f89');
      break;
  }
}

export function applyUiFont(font: string): void {
  const f = UI_FONTS[font] ?? UI_FONTS.sans;
  document.documentElement.style.setProperty('--font-ui', f);
  document.body.style.fontFamily = f;
}

export function applyCodeFont(font: string): void {
  const f = CODE_FONTS[font] ?? CODE_FONTS.mono;
  document.documentElement.style.setProperty('--font-mono', f);
}

function hexToRgba(hex: string, alpha: number): string {
  const m = hex.replace('#', '').match(/.{1,2}/g);
  if (!m || m.length < 3) return `rgba(83, 74, 183, ${alpha})`;
  const [r, g, b] = m.map((h) => parseInt(h, 16));
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function findTheme(name: string): Theme | undefined {
  return THEMES.find((t) => t.name === name);
}
