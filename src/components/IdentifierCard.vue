<script setup lang="ts">
import { ref } from 'vue'
import type { IdentifierInfo } from '../lib/types'

const props = defineProps<{
  identifier: IdentifierInfo
  generateRandomFn: (id: string) => Promise<string>
}>()

const emit = defineEmits<{
  copy: [value: string]
  modify: [id: string, newValue: string]
}>()

const customValue = ref('')
const isGenerating = ref(false)

async function onRandom() {
  isGenerating.value = true
  try {
    const value = await props.generateRandomFn(props.identifier.id)
    if (value) customValue.value = value
  } finally {
    isGenerating.value = false
  }
}

function onApply() {
  const val = customValue.value.trim()
  if (val) {
    emit('modify', props.identifier.id, val)
    customValue.value = ''
  }
}
</script>

<template>
  <div class="id-card" :class="{ 'id-card--modifiable': identifier.modifiable }">
    <div class="id-card__header">
      <div class="id-card__title">
        <span class="id-card__label">{{ identifier.label }}</span>
        <span class="id-card__desc">{{ identifier.description }}</span>
      </div>
      <span
        class="id-card__badge"
        :class="identifier.modifiable ? 'id-card__badge--mod' : 'id-card__badge--ro'"
      >
        {{ identifier.modifiable ? '可修改' : '只读' }}
      </span>
    </div>

    <div class="id-card__body">
      <div class="id-card__value-row">
        <code class="id-card__value">{{ identifier.value }}</code>
        <button class="btn-icon" title="复制" @click="emit('copy', identifier.value)">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
            <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
          </svg>
        </button>
      </div>

      <div v-if="identifier.modifiable" class="id-card__actions">
        <div class="id-card__input-group">
          <input
            v-model="customValue"
            class="id-card__input"
            :placeholder="'输入新的 ' + identifier.label"
            @keyup.enter="onApply"
          />
          <button
            class="btn btn--primary"
            title="随机生成"
            :disabled="isGenerating"
            @click="onRandom"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="16 3 21 3 21 8"/>
              <line x1="4" y1="20" x2="21" y2="3"/>
              <polyline points="21 16 21 21 16 21"/>
              <line x1="15" y1="15" x2="21" y2="21"/>
              <line x1="4" y1="4" x2="9" y2="9"/>
            </svg>
            随机
          </button>
          <button
            class="btn btn--success"
            :disabled="!customValue.trim()"
            @click="onApply"
          >
            应用
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
