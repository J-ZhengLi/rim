<script setup lang="ts">
import { invokeLabel } from '@/utils';
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

const router = useRouter();
const menu = computed(() => {
  const index = router.options.routes.findIndex(
    (route) => route.name === 'Installer'
  );
  return router.options.routes[index].children || [];
});
const titles = ref<PageMeta[]>([]);

interface PageMeta {
  title: string,
  orders: number[],
}

function menuItemActive(orders: number[]) {
  const isFocus = orders.includes(router.currentRoute.value.meta.order as number);
  return {
    'text-3.2vh c-active': isFocus,
  };
}

onMounted(() => {
  menu.value.forEach(async (item, idx) => {
    if (idx !== 0) {
      const label = item.meta?.title as string;
      const order = item.meta?.order as number;
      const localized = await invokeLabel(label.split(':')[0]);

      const existingTitle = titles.value.find((m) => m.title === localized);
      if (existingTitle) {
        existingTitle.orders.push(order);
      } else {
        titles.value.push({ title: localized, orders: [order] });
      }
    }
  });
})
</script>

<template>
  <div class="indicator-bar">
    <div
      v-for="item in titles"
      :class="{ ...menuItemActive(item.orders) }"
      transition="all 0.3s"
      c="secondary"
    >
    {{ item.title }}
    </div>
  </div>
</template>

<style scoped>
.indicator-bar {
  margin-top: 2vh;
  width: 100%;
  display: flex;
  text-align: center;
  font-size: 2.6vh;
}
.indicator-bar>* {
  flex: 1;
}
</style>