<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

const navItems = [
  { name: '首页', route: '/', icon: 'home' },
  { name: '机器码修改', route: '/machine-code', icon: 'fingerprint' },
]
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar__brand" @click="router.push('/')">
        <div class="sidebar__logo">
          <svg width="22" height="22" viewBox="0 0 256 256" fill="none">
            <path d="M168 58a50 50 0 0 0-55 11.3 50 50 0 0 0-11.3 55L60.3 165.7a8 8 0 0 0 0 11.3l18.7 18.7a8 8 0 0 0 11.3 0L131.7 154.3a50 50 0 0 0 55-11.3 50 50 0 0 0 11.3-55l-27 27-17-4.2-4.2-17 27-27z" fill="none" stroke="currentColor" stroke-width="16" stroke-linejoin="round" stroke-linecap="round"/>
          </svg>
        </div>
        <span class="sidebar__title">工具箱</span>
      </div>

      <nav class="sidebar__nav">
        <a
          v-for="item in navItems"
          :key="item.route"
          class="nav-item"
          :class="{ 'nav-item--active': route.path === item.route }"
          @click="router.push(item.route)"
        >
          <svg v-if="item.icon === 'home'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
          </svg>
          <svg v-else-if="item.icon === 'fingerprint'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 12C2 6.5 6.5 2 12 2a10 10 0 0 1 8 4"/>
            <path d="M5 19.5C5.5 18 6 15 6 12c0-3.5 2.5-6 6-6 3 0 5.5 2 6 5"/>
            <path d="M10 12c0 4-1 8-3 10"/>
            <path d="M14 12c0 2.5-.5 5-1.5 7.5"/>
            <path d="M18 11c0 4-1.5 8-4 11"/>
            <path d="M22 12c0 4-2 9-5 12"/>
          </svg>
          <span>{{ item.name }}</span>
        </a>
      </nav>
    </aside>

    <main class="main-content">
      <router-view />
    </main>
  </div>
</template>
