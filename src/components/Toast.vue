<script setup lang="ts">
import type { ToastMessage } from '../lib/types'

defineProps<{
  messages: ToastMessage[]
}>()

defineEmits<{
  dismiss: [id: number]
}>()
</script>

<template>
  <div class="toast-container">
    <div
      v-for="msg in messages"
      :key="msg.id"
      class="toast"
      :class="'toast--' + msg.type"
      @click="$emit('dismiss', msg.id)"
    >
      {{ msg.message }}
    </div>
  </div>
</template>
