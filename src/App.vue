<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { catalog } from './catalog'
import {
  copyMcpToProject,
  getAppState,
  installCatalogToProject,
  installMcp,
  removeMcp,
  setMcpEnabled,
} from './api'
import type { AppState, CatalogItem, McpServer } from './types'
import { translations, useLang } from './i18n'
import type { Lang } from './i18n'

const appWindow = getCurrentWindow()
function winClose() { appWindow.close() }
function winMinimize() { appWindow.minimize() }
async function winMaximize() {
  if (await appWindow.isMaximized()) appWindow.unmaximize()
  else appWindow.maximize()
}
function winDrag(e: MouseEvent) {
  if (e.buttons === 1) appWindow.startDragging()
}

const lang = ref<Lang>(useLang())
const t = computed(() => translations[lang.value])
function setLang(l: Lang) {
  lang.value = l
  localStorage.setItem('lang', l)
}

type Selection =
  | { kind: 'installed'; item: McpServer }
  | { kind: 'catalog'; item: CatalogItem }
  | null

const state = ref<AppState>({ configs: [], servers: [] })
const loading = ref(false)
const message = ref('')
const error = ref('')
const search = ref('')
const selected = ref<Selection>(null)
const projectPath = ref('')

const activeConfig = computed(() => state.value.configs.find((config) => config.exists) ?? state.value.configs[0])

const filteredServers = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return state.value.servers
  return state.value.servers.filter((server) => {
    return [server.name, server.server_type, server.scope, server.url, ...(server.command ?? []), ...server.environment_keys]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(q))
  })
})

const filteredCatalog = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return catalog
  return catalog.filter((item) => {
    return [item.name, item.title, item.provider, item.description, ...item.tags]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(q))
  })
})

const selectedTitle = computed(() => {
  if (!selected.value) return '—'
  return selected.value.kind === 'installed' ? selected.value.item.name : selected.value.item.title
})

const selectedMeta = computed(() => {
  if (!selected.value) return []
  if (selected.value.kind === 'installed') {
    const item = selected.value.item
    return [item.server_type, item.enabled ? t.value.active.toLowerCase() : t.value.passive.toLowerCase(), item.scope].filter(Boolean)
  }
  return [selected.value.item.provider, ...selected.value.item.tags]
})

function commandText(command?: string[]) {
  return command?.join(' ') || '—'
}

function selectInstalled(item: McpServer) {
  selected.value = { kind: 'installed', item }
}

function selectCatalog(item: CatalogItem) {
  selected.value = { kind: 'catalog', item }
}

async function refresh() {
  loading.value = true
  error.value = ''
  try {
    state.value = await getAppState()
    if (!selected.value && state.value.servers[0]) selectInstalled(state.value.servers[0])
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function run(action: () => Promise<unknown>, ok: string) {
  loading.value = true
  message.value = ''
  error.value = ''
  try {
    const result = await action()
    if (typeof result === 'object' && result && 'configs' in result && 'servers' in result) {
      state.value = result as AppState
    } else {
      await refresh()
    }
    message.value = ok
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function toggleInstalled(item: McpServer) {
  await run(() => setMcpEnabled(item.source_path, item.name, !item.enabled), t.value.updated(item.name))
  const refreshed = state.value.servers.find((s) => s.name === item.name && s.source_path === item.source_path)
  if (refreshed) selectInstalled(refreshed)
}

async function deleteInstalled(item: McpServer) {
  if (!confirm(t.value.confirmRemove(item.name))) return
  await run(() => removeMcp(item.source_path, item.name), t.value.removed(item.name))
  selected.value = state.value.servers[0] ? { kind: 'installed', item: state.value.servers[0] } : null
}

async function installToGlobal(item: CatalogItem) {
  if (!activeConfig.value) throw new Error(t.value.noConfig)
  await run(() => installMcp(activeConfig.value.path, item.name, item.definition), t.value.addedGlobal(item.title))
  const installed = state.value.servers.find((s) => s.name === item.name)
  if (installed) selectInstalled(installed)
}

async function addInstalledToProject(item: McpServer) {
  if (!projectPath.value.trim()) { error.value = t.value.noProjectPath; return }
  const target = await copyMcpToProject(item.source_path, item.name, projectPath.value.trim())
  message.value = t.value.addedProject(item.name, target)
}

async function addCatalogToProject(item: CatalogItem) {
  if (!projectPath.value.trim()) { error.value = t.value.noProjectPath; return }
  const target = await installCatalogToProject(projectPath.value.trim(), item.name, item.definition)
  message.value = t.value.addedProject(item.title, target)
}

onMounted(refresh)
</script>

<template>
  <div class="titlebar" @mousedown="winDrag">
    <span class="titlebar-title">OpenMCP</span>
    <div class="titlebar-controls" @mousedown.stop>
      <button class="titlebar-btn" @click="winMinimize" title="Küçült">
        <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
      </button>
      <button class="titlebar-btn" @click="winMaximize" title="Büyüt">
        <svg width="9" height="9" viewBox="0 0 9 9" fill="none"><rect x="0.5" y="0.5" width="8" height="8" rx="1" stroke="currentColor"/></svg>
      </button>
      <button class="titlebar-btn close" @click="winClose" title="Kapat">
        <svg width="10" height="10" viewBox="0 0 10 10"><line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
      </button>
    </div>
  </div>

  <main class="shell">
    <aside class="sidebar">

      <div class="sidebar-actions">
        <button class="primary" type="button" @click="refresh" :disabled="loading">
          {{ loading ? t.refreshing : t.refresh }}
        </button>
        <div class="lang-switcher">
          <button
            v-for="l in (['tr', 'en', 'de'] as const)"
            :key="l"
            class="lang-btn"
            :class="{ active: lang === l }"
            type="button"
            @click="setLang(l)"
          >{{ l.toUpperCase() }}</button>
        </div>
      </div>

      <p class="section-label">{{ t.config }}</p>
      <div v-for="config in state.configs" :key="config.path" class="config-row" :class="{ missing: !config.exists }">
        <span>{{ config.label }}</span>
        <small>{{ config.exists ? `${config.mcp_count} MCP` : t.missing }}</small>
      </div>

      <p class="section-label" style="margin-top: 18px;">{{ t.installedMcps }}</p>
      <button
        v-for="server in filteredServers"
        :key="`${server.source_path}:${server.name}`"
        class="list-item"
        :class="{ active: selected?.kind === 'installed' && selected.item.name === server.name, disabled: !server.enabled }"
        type="button"
        @click="selectInstalled(server)"
      >
        <span class="status-dot" :class="{ off: !server.enabled }"></span>
        <span>
          <strong>{{ server.name }}</strong>
          <small>{{ server.server_type }} · {{ server.scope }}</small>
        </span>
      </button>
      <p v-if="!filteredServers.length" class="empty">{{ t.noInstalled }}</p>
    </aside>

    <section class="content">
      <header class="topbar">
        <div>
          <h1>{{ t.pageTitle }}</h1>
          <p>{{ t.pageDesc }}</p>
        </div>
        <input v-model="search" class="search" :placeholder="t.searchPlaceholder" />
      </header>

      <div v-if="message" class="notice success">{{ message }}</div>
      <div v-if="error" class="notice error">{{ error }}</div>

      <section class="catalog-grid">
        <button
          v-for="item in filteredCatalog"
          :key="item.name"
          class="catalog-card"
          :class="{ active: selected?.kind === 'catalog' && selected.item.name === item.name }"
          type="button"
          @click="selectCatalog(item)"
        >
          <span>{{ item.provider }}</span>
          <strong>{{ item.title }}</strong>
          <p>{{ item.description }}</p>
          <div class="tags">
            <small v-for="tag in item.tags" :key="tag">{{ tag }}</small>
          </div>
        </button>
      </section>
    </section>

    <aside class="details">
      <div class="details-card">
        <div class="details-head">
          <div>
            <span class="eyebrow">{{ t.details }}</span>
            <h2>{{ selectedTitle }}</h2>
          </div>
          <span v-if="selected?.kind === 'installed'" class="pill" :class="{ off: !selected.item.enabled }">
            {{ selected.item.enabled ? t.active : t.passive }}
          </span>
        </div>

        <div class="meta">
          <span v-for="item in selectedMeta" :key="item">{{ item }}</span>
        </div>

        <template v-if="selected?.kind === 'installed'">
          <label>{{ t.command }}</label>
          <pre>{{ commandText(selected.item.command) }}</pre>
          <label>{{ t.url }}</label>
          <pre>{{ selected.item.url || '—' }}</pre>
          <label>{{ t.envKeys }}</label>
          <div class="tags roomy">
            <small v-for="key in selected.item.environment_keys" :key="key">{{ key }}</small>
            <small v-if="!selected.item.environment_keys.length">{{ t.none }}</small>
          </div>
          <label>{{ t.source }}</label>
          <pre>{{ selected.item.source_path }}</pre>
          <div class="actions">
            <button type="button" @click="toggleInstalled(selected.item)">
              {{ selected.item.enabled ? t.disable : t.enable }}
            </button>
            <button type="button" class="danger" @click="deleteInstalled(selected.item)">{{ t.remove }}</button>
          </div>
        </template>

        <template v-else-if="selected?.kind === 'catalog'">
          <p class="description">{{ selected.item.description }}</p>
          <label>{{ t.installCommand }}</label>
          <pre>{{ commandText(selected.item.definition.command) }}{{ selected.item.definition.url ?? '' }}</pre>
          <p v-if="selected.item.installNote" class="hint">{{ selected.item.installNote }}</p>
          <div class="actions">
            <button type="button" @click="installToGlobal(selected.item)">{{ t.installGlobal }}</button>
          </div>
        </template>

        <template v-else>
          <p class="description">{{ t.selectHint }}</p>
        </template>
      </div>

      <div class="details-card project-box">
        <span class="eyebrow">{{ t.projectBinding }}</span>
        <h3>{{ t.addToProject }}</h3>
        <input v-model="projectPath" placeholder="C:\Users\...\my-project" />
        <button v-if="selected?.kind === 'installed'" type="button" @click="addInstalledToProject(selected.item)">
          {{ t.useInProject }}
        </button>
        <button v-else-if="selected?.kind === 'catalog'" type="button" @click="addCatalogToProject(selected.item)">
          {{ t.installToProject }}
        </button>
        <p>{{ t.projectNote }} <code>opencode.json</code></p>
      </div>
    </aside>
  </main>
</template>
