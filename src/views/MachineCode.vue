<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  getPlatform,
  checkAdmin,
  getAllIdentifiers,
  modifyIdentifier,
  generateRandomValue,
  backupIdentifiers,
  restoreIdentifiers,
} from '../lib/commands'
import type { IdentifierInfo, ToastMessage } from '../lib/types'
import IdentifierCard from '../components/IdentifierCard.vue'
import ActionBar from '../components/ActionBar.vue'
import ToastList from '../components/Toast.vue'

const identifiers = ref<IdentifierInfo[]>([])
const loading = ref(true)
const toasts = ref<ToastMessage[]>([])
const isAdmin = ref(true)
const platform = ref('')

const adminWarningText = computed(() => {
  switch (platform.value) {
    case 'macos':
      return '当前未以 root 身份运行，部分功能可能不可用。请使用 sudo 运行应用。'
    case 'linux':
      return '当前未以 root 身份运行，部分功能可能不可用。请使用 sudo 运行应用。'
    default:
      return '当前未以管理员身份运行，部分功能可能不可用。请关闭软件后右键选择「以管理员身份运行」。'
  }
})

async function loadIdentifiers() {
  loading.value = true
  try {
    identifiers.value = await getAllIdentifiers()
  } catch (e: unknown) {
    showToast('加载机器码失败: ' + String(e), 'error')
  } finally {
    loading.value = false
  }
}

async function handleModify(id: string, newValue: string) {
  try {
    await modifyIdentifier(id, newValue)
    showToast('修改成功，正在重新读取...', 'success')
    await loadIdentifiers()
  } catch (e: unknown) {
    showToast('修改失败: ' + String(e), 'error')
  }
}

async function handleGenerateRandom(id: string): Promise<string> {
  try {
    return await generateRandomValue(id)
  } catch (e: unknown) {
    showToast('生成随机值失败: ' + String(e), 'error')
    return ''
  }
}

async function handleRandomizeAll() {
  const modifiable = identifiers.value.filter((i) => i.modifiable)
  let successCount = 0
  for (const item of modifiable) {
    try {
      const newValue = await generateRandomValue(item.id)
      await modifyIdentifier(item.id, newValue)
      successCount++
    } catch (_) {
      /* continue */
    }
  }
  showToast(`一键随机完成 (${successCount}/${modifiable.length})`, successCount > 0 ? 'success' : 'error')
  await loadIdentifiers()
}

async function handleBackup() {
  try {
    const path = await backupIdentifiers()
    showToast('备份成功: ' + path, 'success')
  } catch (e: unknown) {
    showToast('备份失败: ' + String(e), 'error')
  }
}

async function handleRestore() {
  try {
    await restoreIdentifiers()
    showToast('还原成功', 'success')
    await loadIdentifiers()
  } catch (e: unknown) {
    showToast('还原失败: ' + String(e), 'error')
  }
}

function handleCopy(value: string) {
  navigator.clipboard.writeText(value)
  showToast('已复制到剪贴板', 'info')
}

function showToast(message: string, type: ToastMessage['type']) {
  const id = Date.now() + Math.random()
  toasts.value.push({ id, message, type })
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }, 3500)
}

function dismissToast(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

onMounted(async () => {
  platform.value = await getPlatform()
  isAdmin.value = await checkAdmin()
  await loadIdentifiers()
})
</script>

<template>
  <div class="machine-code">
    <div v-if="!isAdmin" class="admin-warning">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      <span>{{ adminWarningText }}</span>
    </div>

    <ActionBar
      :loading="loading"
      @refresh="loadIdentifiers"
      @randomize-all="handleRandomizeAll"
      @backup="handleBackup"
      @restore="handleRestore"
    />

    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
      <span>正在读取硬件信息...</span>
    </div>

    <div v-else class="card-grid">
      <IdentifierCard
        v-for="item in identifiers"
        :key="item.id"
        :identifier="item"
        :generate-random-fn="handleGenerateRandom"
        @copy="handleCopy"
        @modify="handleModify"
      />
    </div>

    <ToastList :messages="toasts" @dismiss="dismissToast" />
  </div>
</template>
