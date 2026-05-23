import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Cross-origin isolation lets onnxruntime-web use threaded WASM (SharedArrayBuffer).
const crossOriginIsolation = {
  name: 'cross-origin-isolation',
  configureServer(server: { middlewares: { use: (fn: (req: unknown, res: { setHeader: (k: string, v: string) => void }, next: () => void) => void) => void } }) {
    server.middlewares.use((_req, res, next) => {
      res.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
      res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp')
      next()
    })
  },
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss(), crossOriginIsolation],
  // @jsquash/webp ships its own wasm; let it load untouched by the dep optimizer.
  optimizeDeps: { exclude: ['@jsquash/webp'] },
  test: {
    environment: 'node',
    include: ['tests/**/*.test.ts'],
  },
})
