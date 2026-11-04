import {defineConfig} from 'vite';
export default defineConfig({base:process.env.STUDY_WEB_BASE || '/study-platform/',build:{outDir:'dist'},test:{environment:'jsdom',include:['src/**/*.test.ts','src/**/*.test.tsx']}} as any);
