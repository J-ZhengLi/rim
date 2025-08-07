<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';

const router = useRouter();
const { t } = useI18n();
const menu = computed(() => {
  const index = router.options.routes.findIndex(
    (route) => route.name === 'Installer'
  );
  return router.options.routes[index].children || [];
});
const titles = computed<PageMeta[]>(() => {
  let nonRepeatingTitles: PageMeta[] = [];
  
  menu.value.forEach(async (item, idx) => {
    if (idx !== 0) {
      const label = item.meta?.title as string;
      const order = item.meta?.order as number;
      const localized = t(label.split(':')[0]);

      const existingTitle = nonRepeatingTitles.find((m) => m.title === localized);
      if (existingTitle) {
        existingTitle.orders.push(order);
      } else {
        nonRepeatingTitles.push({ title: localized, orders: [order] });
      }
    }
  });

  return nonRepeatingTitles;
});

interface PageMeta {
  title: string,
  orders: number[],
}

function menuItemActive(orders: number[]) {
  const isFocus = orders.includes(router.currentRoute.value.meta.order as number);
  return {
    'c-secondary': !isFocus,
    'indicator-bar-active': isFocus,
  };
}
</script>

<template>
  <div class="indicator-bar">
    <div
      v-for="item in titles"
      :class="{ ...menuItemActive(item.orders) }"
      transition="all 0.3s"
    >
    {{ item.title }}
    </div>
  </div>
</template>

<style scoped>
.indicator-bar {
  margin-inline: 4vh;
  margin-top: 1.2vh;
  width: 100%;
  display: flex;
  text-align: center;
  font-size: 2.5vh;
}
.indicator-bar-active {
  --uno: 'text-3.2vh c-active';
  padding-bottom: 1.5vh;
  border-bottom: 2px solid #5e7ce0;
}
.indicator-bar>* {
  flex: 1;
}
</style>