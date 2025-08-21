<template>
    <div class="info-panel">
        <base-card class="org-info-card">
            <img class="org-logo" src="/logo.png" />
            <div class="org-name">{{ labels.logoText }}</div>
            <base-button theme="primary" w="15vw" @click="openOfficialSite">
                {{ $t('official_site') }}
            </base-button>
        </base-card>
        <base-card class="rim-info-card">
            <b c-regular>{{ labels.appName }} (RIM)</b>
            <span c-regular>{{ rimVersion }}</span>
            <base-button mt="1rem" @click="showContributors = true">{{ $t('contributors') }}</base-button>
        </base-card>
        <base-panel :show="showContributors" :clickToHide="false">
            <div class="contributors">
                <h2>{{ $t('contributors') }}</h2>
                <div v-if="loading">{{ $t('loading') }}...</div>

                <div v-else class="grid">
                    <a v-for="c in contributors" :key="c.login" :href="c.html_url" class="avatar" target="_blank"
                        :title="c.login">
                        <img :src="c.avatar_url" :alt="c.login" />
                    </a>
                </div>

                <base-button theme="primary" mt="1rem" @click="showContributors = false">{{ $t('close') }}</base-button>
            </div>
        </base-panel>
    </div>
</template>

<script setup lang="ts">
import { invokeCommand } from '@/utils';
import { getAppNameWithVersion } from '@/utils/common';
import { shell } from '@tauri-apps/api';
import { onMounted, ref } from 'vue';

// contributors info
type Contributor = { login: string; avatar_url: string; html_url: string };
const contributors = ref<Contributor[]>([]);
const showContributors = ref(false);
const loading = ref(false);
// contributors cache
// TODO: Remove local storage cache after uninstallation
const CACHE_KEY = 'rim_contributors'
const CACHE_EXPIRY_MS = 24 * 60 * 60 * 1000 // 24 hours

const labels = ref<Record<string, string>>({});
const rimVersion = ref('');

async function openOfficialSite() {
    const url = await invokeCommand('get_home_page_url') as string;
    shell.open(url);
}

async function refreshLabels() {
    labels.value.logoText = await invokeCommand('get_build_cfg_locale_str', { key: 'logo_text' }) as string;

    const nameAndVer = await getAppNameWithVersion();
    labels.value.appName = nameAndVer[0];
    rimVersion.value = nameAndVer[1];
}

async function fetchContributors() {
    loading.value = true;

    const cached = localStorage.getItem(CACHE_KEY);
    if (cached) {
        try {
            const { contributorData, timestamp } = JSON.parse(cached);
            const now = Date.now();
            if (now - timestamp < CACHE_EXPIRY_MS) {
                contributors.value = contributorData;
                loading.value = false;
                return;
            }
        } catch (e) {
            console.warn('failed when loading contributor cache, using API call instead.');
        }
    }

    try {
        const response = await fetch('https://api.github.com/repos/J-ZhengLi/rim/contributors');
        if (!response.ok) throw new Error('Failed to fetch contributors');
        const contributorData = await response.json();
        contributors.value = contributorData;
        // cache data to reduce API call & speedup loading
        localStorage.setItem(CACHE_KEY, JSON.stringify({
            contributorData,
            timestamp: Date.now(),
        }));
    } catch (err: any) {
        console.error(err.message);
    } finally {
        loading.value = false;
    }
};

onMounted(async () => {
    await refreshLabels();
    await fetchContributors();
});
</script>

<style lang="css" scoped>
.info-panel {
    width: 70vw;
    height: 70vh;
    box-sizing: border-box;
}

.org-info-card {
    margin: 1vh 2vw;
    height: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}

.org-logo {
    width: 10vw;
}

.org-name {
    font-weight: bolder;
    font-size: clamp(1em, 4.5vh, 40px);
    --uno: 'c-regular';
    letter-spacing: 8px;
    padding: 5px;
    transform: translateX(0.2rem);
    margin-block: 3vh;
}

.rim-info-card {
    margin: 3vh 2vw;
    height: 30%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}

.contributors {
    text-align: center;
}

.grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
    gap: 1rem;
    justify-items: center;
    max-width: 600px;
    margin: 1rem auto 0;
}

.avatar img {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    transition: transform 0.2s, box-shadow 0.2s;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
}

.avatar:hover img {
    transform: scale(1.1);
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.15);
}
</style>
