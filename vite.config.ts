import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import tailwindcss from '@tailwindcss/vite'

const host = process.env['TAURI_DEV_HOST']

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [solid(), tailwindcss()],

  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@player': fileURLToPath(new URL('./tauri-plugin-player/guest-js', import.meta.url)),
    },
  },

  // Tauri expects a fixed port and must fail loudly if it is taken.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    // Only set when running on a physical device via TAURI_DEV_HOST —
    // `exactOptionalPropertyTypes` forbids assigning an explicit undefined.
    ...(host ? { hmr: { protocol: 'ws', host, port: 1421 } } : {}),
    watch: {
      ignored: ['**/src-tauri/**', '**/tauri-plugin-player/android/**'],
    },
  },

  // Android WebView baseline: Chromium 100+ ships with recent Play System WebView.
  build: {
    target: 'es2022',
    minify: process.env['TAURI_ENV_DEBUG'] ? false : 'esbuild',
    sourcemap: !!process.env['TAURI_ENV_DEBUG'],
  },
})
