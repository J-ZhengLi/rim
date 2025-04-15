import { createApp } from 'vue';
import Notification from './Notification.vue';
import { router } from './router';
import theme from './theme';
import 'virtual:uno.css';

// disable context menu on right click
document.addEventListener('contextmenu', event => event.preventDefault());

const app = createApp(Notification);
app.use(router);
app.use(theme);
app.mount('#notification-popup');
