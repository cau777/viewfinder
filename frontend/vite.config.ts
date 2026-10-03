import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// Backend address for the dev proxy. Override with VITE_API_TARGET if uvicorn
// runs elsewhere (another port, a container, a remote VM...).
const apiTarget = process.env.VITE_API_TARGET ?? 'http://127.0.0.1:8000'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()], // React Fast Refresh = component-level hot reload
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    // HMR runs over a websocket on the dev-server port. If you open the dev
    // server through a reverse proxy / tunnel / container port-map, set
    // VITE_HMR_CLIENT_PORT to the port the *browser* sees.
    hmr: process.env.VITE_HMR_CLIENT_PORT
      ? { clientPort: Number(process.env.VITE_HMR_CLIENT_PORT) }
      : true,
    // Some filesystems (WSL2 on /mnt/c, network shares, some VM mounts) don't
    // emit inotify events; VITE_USE_POLLING=1 falls back to polling.
    watch: process.env.VITE_USE_POLLING ? { usePolling: true, interval: 200 } : undefined,
    // Same-origin in dev: browser -> Vite -> FastAPI. No CORS config needed.
    proxy: {
      '/api': { target: apiTarget, changeOrigin: true },
    },
  },
  build: {
    outDir: 'dist', // FastAPI serves this directory in production
    emptyOutDir: true,
  },
})
