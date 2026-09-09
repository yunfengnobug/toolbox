<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { getPlatform, checkAdmin, getThermalStatus, setCoolingMode, setFanPercent } from '../lib/commands'
import type { ThermalStatus, ToastMessage } from '../lib/types'
import ToastList from '../components/Toast.vue'

const status = ref<ThermalStatus | null>(null)
const loading = ref(true)
const applying = ref(false)
const autoRefresh = ref(true)
const isAdmin = ref(true)
const platform = ref('')
const toasts = ref<ToastMessage[]>([])
const pendingFan = ref<Record<string, number>>({})

let timer: ReturnType<typeof setInterval> | null = null
const fanTimers: Record<string, ReturnType<typeof setTimeout>> = {}

const adminWarningText = computed(() => {
  switch (platform.value) {
    case 'macos':
      return '当前未以 root 身份运行，调节风扇可能不可用。请使用 sudo 运行应用。'
    case 'linux':
      return '当前未以 root 身份运行，PWM 调速可能不可用。请使用 sudo 运行应用。'
    default:
      return '当前未以管理员身份运行，散热模式可能无法写入。请关闭软件后右键选择「以管理员身份运行」。'
  }
})

const modes = [
  {
    id: 'quiet',
    name: '静音',
    desc: '优先降频，风扇更少介入',
  },
  {
    id: 'balanced',
    name: '均衡',
    desc: '主动散热，性能与噪音平衡',
  },
  {
    id: 'performance',
    name: '性能',
    desc: '更积极转风扇，减少降频',
  },
]

function formatBytes(bytes: number) {
  if (!bytes) return '—'
  return (bytes / 1024 / 1024 / 1024).toFixed(1) + ' GB'
}

function formatTemp(temp: number | null | undefined) {
  if (temp == null || Number.isNaN(temp)) return '—'
  return temp.toFixed(1) + '°C'
}

function tempTone(temp: number | null | undefined) {
  if (temp == null) return 'muted'
  if (temp >= 85) return 'hot'
  if (temp >= 70) return 'warm'
  return 'cool'
}

function sensorWidth(temp: number) {
  return Math.min(100, Math.max(6, (temp / 100) * 100)) + '%'
}

function fanValue(id: string, fallback: number | null) {
  if (pendingFan.value[id] != null) return pendingFan.value[id]
  return fallback ?? 40
}

function policyLabel(policy: string) {
  if (policy === 'passive') return '被动（先降频）'
  if (policy === 'active') return '主动（先转风扇）'
  return '未知'
}

async function loadStatus(silent = false) {
  if (!silent) loading.value = true
  try {
    status.value = await getThermalStatus()
  } catch (e: unknown) {
    if (!silent) showToast('读取硬件状态失败: ' + String(e), 'error')
  } finally {
    loading.value = false
  }
}

async function applyMode(mode: string) {
  applying.value = true
  try {
    const msg = await setCoolingMode(mode)
    showToast(msg, 'success')
    await loadStatus(true)
  } catch (e: unknown) {
    showToast('切换失败: ' + String(e), 'error')
  } finally {
    applying.value = false
  }
}

function onFanInput(id: string, event: Event) {
  const value = Number((event.target as HTMLInputElement).value)
  pendingFan.value = { ...pendingFan.value, [id]: value }
  if (fanTimers[id]) clearTimeout(fanTimers[id])
  fanTimers[id] = setTimeout(() => {
    void commitFan(id, value)
  }, 350)
}

async function commitFan(id: string, percent: number) {
  try {
    const msg = await setFanPercent(id, percent)
    showToast(msg, 'success')
    await loadStatus(true)
  } catch (e: unknown) {
    showToast('调速失败: ' + String(e), 'error')
  }
}

function showToast(message: string, type: ToastMessage['type']) {
  const toastId = Date.now() + Math.random()
  toasts.value.push({ id: toastId, message, type })
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== toastId)
  }, 3500)
}

function dismissToast(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

function startTimer() {
  if (timer) clearInterval(timer)
  timer = setInterval(() => {
    if (autoRefresh.value && !applying.value) {
      void loadStatus(true)
    }
  }, 2500)
}

onMounted(async () => {
  platform.value = await getPlatform()
  isAdmin.value = await checkAdmin()
  await loadStatus()
  startTimer()
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
  Object.values(fanTimers).forEach((t) => clearTimeout(t))
})
</script>

<template>
  <div class="thermal">
    <div v-if="!isAdmin" class="admin-warning">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      <span>{{ adminWarningText }}</span>
    </div>

    <div class="action-bar">
      <button class="btn" :disabled="loading" @click="loadStatus()">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="23 4 23 10 17 10"/>
          <polyline points="1 20 1 14 7 14"/>
          <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
        </svg>
        刷新
      </button>
      <button class="btn" :class="{ 'btn--primary': autoRefresh }" @click="autoRefresh = !autoRefresh">
        {{ autoRefresh ? '自动刷新开' : '自动刷新关' }}
      </button>
      <div class="action-spacer"></div>
      <span v-if="status" class="thermal-cpu-name">{{ status.cpu_name }}</span>
    </div>

    <div v-if="loading && !status" class="loading-state">
      <div class="loading-spinner"></div>
      <span>正在读取温度和风扇状态...</span>
    </div>

    <div v-else-if="status" class="thermal-body">
      <div class="thermal-stats">
        <div class="stat-card" :class="'stat-card--' + tempTone(status.cpu_temp)">
          <span class="stat-card__label">CPU 温度</span>
          <span class="stat-card__value">{{ formatTemp(status.cpu_temp) }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-card__label">CPU 占用</span>
          <span class="stat-card__value">{{ status.cpu_usage.toFixed(0) }}%</span>
        </div>
        <div class="stat-card">
          <span class="stat-card__label">内存</span>
          <span class="stat-card__value">{{ formatBytes(status.memory_used) }}</span>
          <span class="stat-card__sub">/ {{ formatBytes(status.memory_total) }}</span>
        </div>
        <div class="stat-card" :class="'stat-card--' + tempTone(status.gpus[0]?.temperature ?? null)">
          <span class="stat-card__label">GPU 温度</span>
          <span class="stat-card__value">{{ status.gpus[0] ? formatTemp(status.gpus[0].temperature) : '—' }}</span>
        </div>
      </div>

      <section v-if="status.modes_available" class="thermal-section">
        <div class="thermal-section__head">
          <h3>散热模式</h3>
          <span class="thermal-section__hint">当前策略：{{ policyLabel(status.cooling.policy) }}</span>
        </div>
        <div class="mode-grid">
          <button
            v-for="mode in modes"
            :key="mode.id"
            class="mode-card"
            :class="{ 'mode-card--active': status.cooling.mode === mode.id }"
            :disabled="applying"
            @click="applyMode(mode.id)"
          >
            <span class="mode-card__name">{{ mode.name }}</span>
            <span class="mode-card__desc">{{ mode.desc }}</span>
          </button>
        </div>
      </section>

      <section class="thermal-section">
        <div class="thermal-section__head">
          <h3>风扇</h3>
          <span class="thermal-section__hint">{{ status.fans.length ? status.fans.length + ' 个' : '未检测到可读取的风扇' }}</span>
        </div>
        <div v-if="!status.fans.length" class="thermal-empty">Windows 很少直接暴露风扇转速。可改用上方散热模式调节，或在 BIOS / 主板软件中设置。</div>
        <div v-else class="fan-list">
          <div v-for="fan in status.fans" :key="fan.id" class="fan-row">
            <div class="fan-row__meta">
              <span class="fan-row__name">{{ fan.name }}</span>
              <span class="fan-row__value">
                <template v-if="fan.rpm != null">{{ fan.rpm }} RPM</template>
                <template v-else-if="fan.percent != null">{{ fan.percent }}%</template>
                <template v-else>转速未知</template>
              </span>
            </div>
            <input
              v-if="fan.controllable"
              class="thermal-range"
              type="range"
              min="20"
              max="100"
              step="5"
              :value="fanValue(fan.id, fan.percent)"
              @input="onFanInput(fan.id, $event)"
            >
            <span v-else class="fan-row__ro">只读</span>
          </div>
        </div>
      </section>

      <section class="thermal-section">
        <div class="thermal-section__head">
          <h3>温度传感器</h3>
        </div>
        <div v-if="!status.sensors.length" class="thermal-empty">没有读到温度传感器。</div>
        <div v-else class="sensor-list">
          <div v-for="sensor in status.sensors" :key="sensor.name" class="sensor-row">
            <div class="sensor-row__top">
              <span>{{ sensor.name }}</span>
              <span :class="'sensor-row__temp sensor-row__temp--' + tempTone(sensor.temperature)">{{ formatTemp(sensor.temperature) }}</span>
            </div>
            <div class="sensor-bar">
              <div
                class="sensor-bar__fill"
                :class="'sensor-bar__fill--' + tempTone(sensor.temperature)"
                :style="{ width: sensorWidth(sensor.temperature) }"
              ></div>
            </div>
          </div>
        </div>
      </section>

      <div v-if="status.notes.length" class="thermal-notes">
        <p v-for="note in status.notes" :key="note">{{ note }}</p>
      </div>
    </div>

    <ToastList :messages="toasts" @dismiss="dismissToast" />
  </div>
</template>
