<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

const update = ref<Update | null>(null)
const visible = ref(false)
const downloading = ref(false)
const downloaded = ref(0)
const contentLength = ref(0)
const error = ref('')

const progress = computed(() => {
  if (contentLength.value <= 0) return 0
  return Math.round((downloaded.value / contentLength.value) * 100)
})

onMounted(async () => {
  try {
    const result = await check()
    if (result) {
      update.value = result
      visible.value = true
    }
  } catch (e) {
    console.error('Update check failed:', e)
  }
})

function dismiss() {
  visible.value = false
}

async function install() {
  if (!update.value) return
  downloading.value = true
  error.value = ''
  try {
    await update.value.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          contentLength.value = event.data.contentLength ?? 0
          break
        case 'Progress':
          downloaded.value += event.data.chunkLength
          break
        case 'Finished':
          break
      }
    })
    await relaunch()
  } catch (e: any) {
    error.value = e?.message ?? String(e)
    downloading.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="update-overlay" @click.self="dismiss">
      <div class="update-dialog">
        <div class="update-dialog__header">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
          <span>发现新版本</span>
        </div>

        <div class="update-dialog__body">
          <p class="update-dialog__version">
            版本 <strong>{{ update?.version }}</strong> 已可用
          </p>
          <div v-if="update?.body" class="update-dialog__notes">
            {{ update.body }}
          </div>

          <div v-if="downloading" class="update-dialog__progress">
            <div class="progress-bar">
              <div class="progress-bar__fill" :style="{ width: progress + '%' }" />
            </div>
            <span class="progress-bar__text">{{ progress }}%</span>
          </div>

          <p v-if="error" class="update-dialog__error">{{ error }}</p>
        </div>

        <div class="update-dialog__footer">
          <button class="btn" :disabled="downloading" @click="dismiss">稍后再说</button>
          <button class="btn btn--primary" :disabled="downloading" @click="install">
            <template v-if="downloading">下载中...</template>
            <template v-else>立即更新</template>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
