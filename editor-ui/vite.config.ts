import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // Disable crossorigin attribute — wry custom protocol doesn't need CORS mode
    crossOriginLoading: false,
  },
  server: {
    port: 5173,
  },
});
