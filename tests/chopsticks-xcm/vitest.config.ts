import { defineConfig } from 'vitest/config'

export default defineConfig({
    test: {
        hookTimeout: 300000,
        testTimeout: 300000,
    }
})
