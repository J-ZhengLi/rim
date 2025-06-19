import type { App } from 'vue';
import BaseButton from '../components/BaseButton.vue';
import BaseInput from '../components/BaseInput.vue';
import BaseCheckBox from '../components/BaseCheckBox.vue';
import BaseProgress from '../components/BaseProgress.vue';
import BaseRadio from '../components/BaseRadio.vue';
import BaseTag from '@/components/BaseTag.vue';
import Titlebar from '../components/Titlebar.vue';
import LoadingMask from '../components/LoadingMask.vue';
import BaseToast from '@/components/BaseToast.vue';
import BaseCard from '@/components/BaseCard.vue';
import Background from '@/components/Background.vue';
import Inputton from '@/components/Inputton.vue';
import PageNavButtons from '@/components/PageNavButtons.vue';
import BaseDetails from '@/components/BaseDetails.vue';

export default {
  install(app: App) {
    app.component('base-button', BaseButton);
    app.component('base-input', BaseInput);
    app.component('base-check-box', BaseCheckBox);
    app.component('base-progress', BaseProgress);
    app.component('base-radio', BaseRadio);
    app.component('base-tag', BaseTag);
    app.component('titlebar', Titlebar);
    app.component('loading-mask', LoadingMask);
    app.component('base-toast', BaseToast);
    app.component('base-card', BaseCard);
    app.component('background', Background);
    app.component('inputton', Inputton);
    app.component('page-nav-buttons', PageNavButtons);
    app.component('base-details', BaseDetails);
  },
};
