import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { createLogger, defineConfig } from 'vite';

/**
 * Vite logs its own stack trace for every failed proxy attempt, on top of whatever the proxy's error
 * handler does, and there is no option to turn that off. Since a missing bridge is an expected state
 * here - not a fault - collapse those into one readable line and let everything else through.
 */
function quietBridgeLogger() {
  const logger = createLogger();
  const passThrough = logger.error;
  let warned = false;

  logger.error = (msg, options) => {
    if (msg.includes('http proxy error')) {
      if (!warned) {
        warned = true;
        logger.warn(
          `  The iPhone Inspector app is not answering on port ${process.env.PORT ?? 3820}. Start it with "cargo run"; the UI reconnects on its own.`,
        );
      }
      return;
    }
    passThrough(msg, options);
  };

  return logger;
}

export default defineConfig({
  // Relative, so the built bundle works wherever it is served from - the local service at any port,
  // or opened straight from disk - with no absolute asset paths baked in. Routing is hash-based for
  // the same reason; see App.tsx.
  base: './',
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
  plugins: [react(), tailwindcss()],
  customLogger: quietBridgeLogger(),
  // In development Vite serves the frontend and forwards API calls to the local USB bridge, so both
  // halves sit on one origin and the browser never makes a cross-origin request. The production
  // build is served by that same server, where the proxy is not involved at all.
  server: {
    proxy: {
      '/api': {
        target: `http://127.0.0.1:${process.env.PORT ?? 3820}`,
        changeOrigin: false,
        /**
         * Answer for the bridge when it is not up, instead of letting the proxy throw.
         *
         * The app polls every few seconds by design, including while the Rust app is stopped or
         * rebuilding, so each poll would otherwise print another ECONNREFUSED stack trace and bury
         * the dev output.
         *
         * A 503 with the same shape the API uses gives the UI its "service not running" state,
         * which is exactly what it should show, and keeps one quiet line in the terminal.
         */
        configure: proxy => {
          proxy.on('error', (_err, _req, res) => {
            // Websocket upgrades hand back a raw socket, which has no writeHead.
            if (!('writeHead' in res) || res.headersSent) {
              res.destroy?.();
              return;
            }
            res.writeHead(503, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'iPhone Inspector is not running.' }));
          });
        },
      },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
