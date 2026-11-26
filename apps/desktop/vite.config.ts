import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  plugins: [react(), tailwindcss(), {
    name: 'local-input-probe',
    configureServer(server) {
      if (process.env.VITE_INPUT_PROBE !== 'true') return;
      server.middlewares.use('/__input-probe', (request, response) => {
        if (request.method !== 'POST') { response.statusCode = 405; response.end(); return; }
        let body = '';
        request.on('data', chunk => { body += chunk; if (body.length > 20000) request.destroy(); });
        request.on('end', () => {
          const data = JSON.parse(body);
          writeFileSync(fileURLToPath(new URL('../../docs/input-smoke-result.json', import.meta.url)), JSON.stringify(data,null,2));
          response.end('saved');
        });
      });
    },
  }],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: { target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'es2022' },
});
