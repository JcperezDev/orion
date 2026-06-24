/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        background: 'var(--background)',
        foreground: 'var(--foreground)',
        surface: 'var(--surface)',
        'surface-2': 'var(--surface-2)',
        'surface-3': 'var(--surface-3)',
        border: 'var(--border)',
        'border-strong': 'var(--border-strong)',
        subtle: 'var(--subtle)',
        text: 'var(--text)',
        'text-dim': 'var(--text-dim)',
        'accent-blue': 'var(--accent-blue)',
        'accent-blue-bg': 'var(--accent-blue-bg)',
        'accent-green': 'var(--accent-green)',
        'accent-green-bg': 'var(--accent-green-bg)',
        'accent-purple': 'var(--accent-purple)',
        'accent-purple-bg': 'var(--accent-purple-bg)',
        'accent-amber': 'var(--accent-amber)',
        'accent-amber-bg': 'var(--accent-amber-bg)',
        'accent-red': 'var(--accent-red)',
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'SF Mono', 'Fira Code', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
}
