import { pathToFileURL, fileURLToPath } from 'node:url';
const { chromium: realChromium } = await import(pathToFileURL(process.env.TSUNORU_TEST_PLAYWRIGHT).href);
export const chromium = {
  launch(options) {
    const launch = realChromium.launch({ ...options, executablePath: fileURLToPath(new URL('./stalled-calendar-browser.sh', import.meta.url)) });
    const timer = setTimeout(() => console.log('pending_browser_launch=true'), 200);
    return launch.finally(() => clearTimeout(timer));
  },
};
