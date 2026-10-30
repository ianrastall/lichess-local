import {defineConfig} from '@playwright/test';
export default defineConfig({testDir:'./tests',workers:1,use:{baseURL:'http://127.0.0.1:5198',viewport:{width:1360,height:850}},webServer:{command:'node ../tools/serve.mjs',url:'http://127.0.0.1:5198',reuseExistingServer:!process.env.CI},reporter:'list'});
