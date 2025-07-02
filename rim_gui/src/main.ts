import { createApp } from 'vue';
import App from './App.vue';
import { router } from './router';
import theme from './theme';
import 'virtual:uno.css';
import i18n from './i18n';

// disable context menu on right click
document.addEventListener('contextmenu', event => event.preventDefault());

createApp(App)
    .use(router)
    .use(theme)
    .use(i18n)
    .mount('#app');
