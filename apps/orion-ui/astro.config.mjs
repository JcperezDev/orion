import { defineConfig } from 'astro/config';
import node from '@astrojs/node';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  output: 'server',
  adapter: node({ mode: 'standalone' }),
  server: { port: 7338, host: '127.0.0.1' },
  vite: {
    plugins: [tailwindcss()],
    server: {
      proxy: {
        '/api': { target: 'http://127.0.0.1:7337', changeOrigin: true },
        '/health': { target: 'http://127.0.0.1:7337', changeOrigin: true },
      },
    },
  },
});
