<template>
    <div class="help-layout">
        <!-- Side Panel -->
        <aside class="sidebar">
            <ul>
                <li v-for="doc in docs" :key="doc.name" @click="selectDoc(doc.name)"
                    :class="{ active: doc.name === currentDoc }">
                    {{ doc.title }}
                </li>
            </ul>
        </aside>

        <!-- Main Panel -->
        <main class="main-content">
            <article @click="handleExternalLinks" v-html="renderedContent" class="markdown-content" />
        </main>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watchEffect } from 'vue';
import { useI18n } from 'vue-i18n';
import { marked } from 'marked'
import { shell } from '@tauri-apps/api';
import hljs from 'highlight.js'
import 'highlight.js/styles/atom-one-dark.css';

const { locale } = useI18n();

const docMap = {
    'en-US': [
        { name: 'intro', title: 'Introduction' },
        { name: 'install', title: 'Setup' },
    ],
    'zh-CN': [
        { name: 'intro', title: '介绍' },
        { name: 'install', title: '首次安装' },
    ],
};
type SupportedLocale = keyof typeof docMap;

const localeWithFallback = computed<SupportedLocale>(() => {
    return (Object.keys(docMap)).includes(locale.value) ? (locale.value as SupportedLocale) : 'en-US';
});
const docs = computed(() => docMap[localeWithFallback.value]);
const currentDoc = ref<string>(docs.value[0].name);

const selectDoc = (name: string) => {
    currentDoc.value = name
};

// Markdown rendering
const rawContent = ref('');
const renderedContent = ref('');
marked.use({
  async: false,
  renderer: {
    code({ text, lang }) {
      const validLang = lang && hljs.getLanguage(lang)
      const highlighted = validLang
        ? hljs.highlight(text, { language: lang }).value
        : hljs.highlightAuto(text).value
      return `<pre><code class="hljs language-${lang}">${highlighted}</code></pre>`
    },
    image({href, title, text}) {
      // If relative, prepend your own base path
      if (href && !href.startsWith('http') && !href.startsWith('/')) {
        const resolved = `/help/en-US/${href}` // dynamically set this per doc
        return `<img src="${resolved}" alt="${text}" title="${title || ''}" />`
      }
      return `<img src="${href}" alt="${text}" title="${title || ''}" />`
    },
  },
});

const fetchMarkdown = async () => {
    const path = `/help/${locale.value}/${currentDoc.value}.md`;
    try {
        const res = await fetch(path);
        const text = await res.text();
        console.log(text);
        rawContent.value = text;
        renderedContent.value = await marked.parse(text);
    } catch (err) {
        console.error('Failed to load:', path);
        renderedContent.value = `<p style="color:red">Failed to load ${path}</p>`;
    }
};

function handleExternalLinks(event: MouseEvent) {
    const isExternalLink = (href: string | null) => {
        return !!href && /^(https?|ftp|mailto|tel):/i.test(href);
    };

    // Traverse DOM to find the nearest <a> tag
    let target = event.target as HTMLElement;
    while (target && target.tagName !== 'A') {
        target = target.parentElement!;
        if (!target) return;
    }

    const href = target.getAttribute('href');
    if (isExternalLink(href)) {
        event.preventDefault();
        shell.open(href!); // Open in system browser
    }
}

watchEffect(fetchMarkdown);
</script>

<style scoped>
.help-layout {
    display: flex;
    width: 85vw;
    height: 75vh;
}

.sidebar {
    width: 10rem;
    transition: width 0.3s ease;
    border-right: 1px solid #ccc;
    padding: 1rem;
    box-sizing: border-box;
}

.sidebar ul {
    list-style: none;
    padding: 0;
}

.sidebar li {
    cursor: pointer;
    padding: 0.5rem 0;
    --uno: 'c-regular';
}

.sidebar li.active {
    font-weight: bold;
    --uno: 'c-active';
}

.sidebar li:hover {
    text-decoration: underline;
}

.main-content {
    flex: 1;
    overflow-y: auto;
    padding-inline: 1.5rem;
    margin-top: 0px;
}
</style>

<style>
.markdown-content h1,
.markdown-content h2,
.markdown-content h3 {
    margin-top: 1.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 2px solid #eee;
}

.markdown-content pre {
  border-radius: 8px;
  overflow-x: auto;
}

/* Inline code only (not inside <pre>) */
.markdown-content :not(pre) > code {
  background-color: #f0f0f0c5;
  color: #d83378;
  font-size: 0.95em;
  padding: 0.1em 0.5em;
}

.markdown-content blockquote {
  border-left: 4px solid #d3d3d3;
  padding: 0.5em 1em;
  background-color: #ffffffda;
  color: #555;
  margin: 1.5em 0;
}
</style>
