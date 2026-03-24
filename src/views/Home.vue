<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

const tools = [
  {
    id: 'machine-code',
    name: '机器码修改',
    desc: '查看和修改硬件标识符',
    icon: 'fingerprint',
    route: '/machine-code',
  },
]
</script>

<template>
  <div class="home">
    <div class="home__header">
      <h2 class="home__title">全部工具</h2>
      <span class="home__count">{{ tools.length }} 个工具</span>
    </div>
    <div class="tool-grid">
      <div
        v-for="tool in tools"
        :key="tool.id"
        class="tool-card"
        @click="router.push(tool.route)"
      >
        <div class="tool-card__icon">
          <svg v-if="tool.icon === 'fingerprint'" width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 12C2 6.5 6.5 2 12 2a10 10 0 0 1 8 4"/>
            <path d="M5 19.5C5.5 18 6 15 6 12c0-3.5 2.5-6 6-6 3 0 5.5 2 6 5"/>
            <path d="M10 12c0 4-1 8-3 10"/>
            <path d="M14 12c0 2.5-.5 5-1.5 7.5"/>
            <path d="M18 11c0 4-1.5 8-4 11"/>
            <path d="M22 12c0 4-2 9-5 12"/>
          </svg>
        </div>
        <div class="tool-card__info">
          <span class="tool-card__name">{{ tool.name }}</span>
          <span class="tool-card__desc">{{ tool.desc }}</span>
        </div>
        <svg class="tool-card__arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </div>
    </div>
  </div>
</template>
