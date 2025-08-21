<script setup lang="ts">
import { invokeCommand, KitItem, managerConf, ManagerOperation } from '@/utils';
import { computed, onMounted, ref } from 'vue';
import { event } from '@tauri-apps/api';
import { useCustomRouter } from '@/router';
import { CliPayload, UpdatePayload } from '@/utils/types/payloads';

const { routerPush } = useCustomRouter();

const installedKit = computed(() => managerConf.getInstalled());
const availableKits = computed(() => managerConf.getKits());
const latestToolkitUrl = ref('');

const displayFormat = ref<'list' | 'card'>('list');

function uninstall() {
  managerConf.setOperation(ManagerOperation.UninstallToolkit);
  routerPush('/manager/uninstall');
}

async function install(url: string) {
  const toolkit = await invokeCommand('get_toolkit_from_url', { url: url }) as KitItem;
  await managerConf.setCurrent(toolkit);
  managerConf.setOperation(ManagerOperation.Update);
  routerPush('/manager/change');
}

function onCardClick(url: string) {
  if (displayFormat.value === 'card') install(url)
}

onMounted(async () => {
  event.listen('change-view', (event) => {
    let payload = event.payload as CliPayload;
    if (payload.command === 'Uninstall') {
      managerConf.setOperation(ManagerOperation.UninstallToolkit);
    }
    routerPush(payload.path);
  });

  event.listen('toolkit:update-available', (event) => {
    let payload = event.payload as UpdatePayload[];
    let maybeUrl = payload[1].data;
    if (maybeUrl) {
      latestToolkitUrl.value = maybeUrl;
    }
  });
});
</script>

<template>
  <div>
    <section>
      <div class="info-label" mb="1rem">{{ $t('current_toolkit') }}</div>
      <base-card v-if="installedKit" flex="~ justify-between items-center" ml="1rem" mr="1.2rem">
        <div flex="~ col">
          <span class="toolkit-name">{{ installedKit?.name }}</span>
          <span class="toolkit-version">{{ installedKit?.version }}</span>
          <span>{{ installedKit?.desc }}</span>
        </div>
        <div flex="~ justify-end" w="25%">
          <base-button w="45%" theme="secondary" @click="uninstall">{{ $t('uninstall') }}</base-button>
        </div>
      </base-card>
      <base-card v-else text="center" ml="1rem" mr="1.2rem">
        <p text="regular">{{ $t('no_toolkit_installed') }}</p>
      </base-card>
    </section>

    <section h="55%">
      <div class="info-label" mt="3vh" mb="1.5vh" flex="~ justify-between">
        {{ $t('available_toolkit') }}
        <!-- display format selector -->
        <div class="format-icons" mt="5px">
          <label :class="{ selected: displayFormat === 'card' }">
            <input type="radio" value="card" v-model="displayFormat" />
            <svg viewBox="0 0 24 24" class="icon">
              <rect x="0" y="0" width="6" height="6" />
              <rect x="9" y="0" width="6" height="6" />
              <rect x="0" y="9" width="6" height="6" />
              <rect x="9" y="9" width="6" height="6" />
            </svg>
          </label>
          <label :class="{ selected: displayFormat === 'list' }">
            <input type="radio" value="list" v-model="displayFormat" />
            <svg viewBox="0 0 24 24" class="icon">
              <rect x="0" y="0" width="15" height="3" />
              <rect x="0" y="6" width="15" height="3" />
              <rect x="0" y="12" width="15" height="3" />
            </svg>
          </label>
        </div>
      </div>

      <div :class="['toolkit-list', displayFormat]">
        <base-card class="toolkit-item" v-for="toolkit in availableKits" :key="toolkit.manifestURL"
          :interactive="displayFormat === 'card'" @click="onCardClick(toolkit.manifestURL)">
          <div flex="~ col">
            <span class="toolkit-name">
              {{ toolkit.name }}

              <div class="latest-indicator" v-if="toolkit.manifestURL === latestToolkitUrl">New</div>
            </span>
            <span class="toolkit-version">{{ toolkit.version }}</span>
            <span mt="1rem" c-regular>{{ toolkit.desc }}</span>
          </div>
          <div class="button-container" v-if="displayFormat === 'list'">
            <base-button class="button" theme="primary" @click="install(toolkit.manifestURL)">{{ $t('install')
              }}</base-button>
          </div>
        </base-card>
      </div>
    </section>
  </div>
</template>

<style lang="css" scoped>
.toolkit-name {
  --uno: 'c-regular';
  font-weight: bold;
  font-size: clamp(20px, 2.6vh, 35px);
}

.latest-indicator {
  background-color: red;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, .6), 0 12px 16px rgba(0, 0, 0, .12);
  border-radius: 20vh;
  color: white;
  text-align: center;
  width: 6vw;
  font-size: 2.3vh;
  margin-left: 1rem;
}

.toolkit-version {
  --uno: 'c-regular';
  font-weight: 600;
  margin-top: 1rem;
  font-size: 2.2vh;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.format-icons {
  display: flex;
  gap: 0.5rem;
}

.format-icons label {
  cursor: pointer;
  opacity: 0.3;
  transition: opacity 0.2s;
  display: flex;
  align-items: center;
}

.format-icons label.selected,
.format-icons label:hover {
  opacity: 1;
}

.format-icons input {
  display: none;
}

.icon {
  width: 24px;
  height: 24px;
  fill: #666;
}

.toolkit-list {
  height: 100%;
  padding: 1rem;
  overflow-y: auto;
  box-sizing: border-box;
  scrollbar-gutter: stable;
}

.toolkit-list.list .toolkit-item {
  display: flex;
  justify-content: space-between;
  margin-bottom: 3vh;
  align-items: center;
}

.toolkit-list.list .toolkit-item .toolkit-name {
  display: flex;
  justify-content: left;
}

.toolkit-list.list .toolkit-item .button-container {
  width: 25%;
  text-align: end;
}

.toolkit-list.list .toolkit-item .button-container .button {
  width: 45%;
}

.toolkit-list.card {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 2rem;
}

.toolkit-list.card .toolkit-item {
  padding: 5%;
  max-height: 180px;
  display: flex;
  flex-direction: column;
  text-align: center;
  justify-content: center;
}

.toolkit-list.card .toolkit-item .toolkit-name {
  display: flex;
  justify-content: center;
}

.toolkit-list.card .toolkit-item * {
  cursor: pointer;
}
</style>
