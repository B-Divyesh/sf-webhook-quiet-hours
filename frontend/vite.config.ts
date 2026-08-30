import { defineConfig } from 'vite';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { readdir, readFile, writeFile } from 'node:fs/promises';

async function listFiles(directory: string, prefix = ''): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(entries.map((entry) => entry.isDirectory()
    ? listFiles(resolve(directory, entry.name), `${prefix}${entry.name}/`)
    : Promise.resolve(`${prefix}${entry.name}`)));
  return files.flat();
}

const serviceWorkerManifest = () => ({
  name: 'versioned-service-worker-manifest',
  async closeBundle() {
    const output = resolve(__dirname, '../dist');
    const serviceWorkerPath = resolve(output, 'sw.js');
    const files = (await listFiles(output))
      .filter((file) => file !== 'sw.js')
      .map((file) => `/${file}`)
      .sort();
    const shell = ['/', '/demo', '/privacy', '/terms', ...files.filter((file) => file !== '/index.html')];
    const version = createHash('sha256').update(shell.join('\n')).digest('hex').slice(0, 12);
    const source = await readFile(serviceWorkerPath, 'utf8');
    await writeFile(serviceWorkerPath, source
      .replace('__CACHE_VERSION__', version)
      .replace('__PRECACHE_ASSETS__', JSON.stringify(shell)));
  },
});

export default defineConfig({
  root: resolve(__dirname),
  publicDir: resolve(__dirname, 'public'),
  build: {
    outDir: resolve(__dirname, '../dist'),
    emptyOutDir: true,
    target: 'es2022',
    sourcemap: false,
  },
  plugins: [serviceWorkerManifest()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8080',
      '/hooks': 'http://localhost:8080',
      '/health': 'http://localhost:8080',
    },
  },
});
