import react from '@vitejs/plugin-react'
import { defineConfig, loadEnv } from 'vite'
import { fileURLToPath } from 'node:url'

// Backend address for the dev proxy. Override with VITE_API_TARGET if uvicorn
// runs elsewhere (another port, a container, a remote VM...).
const localApiTarget = process.env.VITE_API_TARGET ?? 'http://127.0.0.1:8000'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, fileURLToPath(new URL('..', import.meta.url)), ['CARTO_', 'JUPYTER_'])
  // `make dev-remote`: proxy /api to the API running on the JupyterHub server (through jupyter-server-proxy
  // on port 8000), authenticated with the hub API token in .env. The grid needs more memory than most laptops have.
  const remote = process.env.VIEWFINDER_REMOTE_API === '1'
  const apiTarget = remote ? `${env.JUPYTER_SERVER?.replace(/\/$/, '')}/user/${env.JUPYTER_USER}/proxy/8000` : localApiTarget
  if (remote && !env.JUPYTER_API_TOKEN) throw new Error('make dev-remote needs JUPYTER_SERVER, JUPYTER_USER and JUPYTER_API_TOKEN in .env')
  return {
  define: { __CARTO_API_KEY__: JSON.stringify(process.env.CARTO_API_KEY ?? env.CARTO_API_KEY ?? '') },
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
      '/api': {
        target: apiTarget,
        changeOrigin: true,
        headers: remote ? { Authorization: `token ${env.JUPYTER_API_TOKEN}` } : undefined,
      },
    },
  },
  build: {
    outDir: 'dist', // FastAPI serves this directory in production
    emptyOutDir: true,
  },
  }
})
