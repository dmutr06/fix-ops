import { defineConfig, devices } from "@playwright/test";

const port = 8090;

export default defineConfig({
    testDir: "e2e",
    timeout: 120_000,
    workers: 1,
    reporter: [["list"]],
    use: { baseURL: `http://127.0.0.1:${port}`, trace: "retain-on-failure" },
    projects: [
        { name: "desktop", use: { ...devices["Desktop Chrome"] } },
        {
            name: "phone",
            use: {
                ...devices["Desktop Chrome"],
                viewport: { width: 360, height: 740 },
                isMobile: true,
                hasTouch: true,
            },
        },
    ],
    webServer: {
        command: "sh e2e/server.sh",
        url: `http://127.0.0.1:${port}/health`,
        timeout: 300_000,
        reuseExistingServer: false,
        env: { FIXOPS_LISTEN_ADDR: `127.0.0.1:${port}` },
    },
});
