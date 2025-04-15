import { createApp } from 'vue';
import App from './App.vue';
import { router } from './router';
import theme from './theme';
import 'virtual:uno.css';

// disable context menu on right click
document.addEventListener('contextmenu', event => event.preventDefault());

const app = createApp(App);
app.use(router);
app.use(theme);
app.mount('#app');
