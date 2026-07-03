<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Home } from 'lucide-vue-next'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
}>()

const { t } = useI18n()
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-[600px] max-h-[90vh] overflow-hidden flex flex-col p-0">
      <DialogHeader class="px-6 pt-6 shrink-0">
        <DialogTitle class="flex items-center gap-2">
          <Home class="h-5 w-5" />
          {{ t('mihome.title') }}
        </DialogTitle>
      </DialogHeader>

      <div class="flex-1 min-h-0 px-6 pb-6">
        <iframe
          src="/mihome"
          class="w-full border rounded-lg"
          :style="{ height: '70vh', maxHeight: '500px' }"
          frameborder="0"
          allow="clipboard-read; clipboard-write"
        />
      </div>
    </DialogContent>
  </Dialog>
</template>