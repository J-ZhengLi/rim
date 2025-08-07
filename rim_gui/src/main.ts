import { createApp } from 'vue';
import App from './App.vue';
import { router } from './router';
import theme from './theme';
import 'virtual:uno.css';
import { invokeCommand } from './utils';
import { getOrInitI18n } from './i18n';

async function setup() {
    const locale = await invokeCommand('get_locale') as string;
    const i18n = getOrInitI18n(locale);

    createApp(App)
        .use(router)
        .use(theme)
        .use(i18n)
        .mount('#app');
}

// disable context menu on right click
document.addEventListener('contextmenu', event => event.preventDefault());

setup();
