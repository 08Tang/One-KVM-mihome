<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import { Loader2, Mic, MicOff, RefreshCw } from 'lucide-vue-next'

import { Button } from '@/components/ui/button'
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select'
import { getMicrophone } from '@/composables/useMicrophone'

const { t } = useI18n()
const microphone = getMicrophone()

const stateText = computed(() => t(`actionbar.micState.${microphone.state.value}`))
const errorText = computed(() => {
  const code = microphone.errorCode.value
  return code ? t(`actionbar.micError.${code}`) : ''
})

function showError() {
  if (errorText.value) toast.error(errorText.value)
}

async function toggle() {
  await microphone.toggle()
  showError()
}

async function requestPermission() {
  const granted = await microphone.refreshInputDevices(true)
  if (!granted) showError()
}

async function selectDevice(value: unknown) {
  await microphone.selectInputDevice(String(value ?? ''))
  showError()
}
</script>

<template>
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <h5 class="text-xs font-medium text-muted-foreground">
        {{ t('actionbar.microphoneTransfer') }}
      </h5>
      <Button
        v-if="microphone.permissionGranted.value"
        variant="ghost"
        size="icon-xs"
        :aria-label="t('common.refresh')"
        :disabled="microphone.loadingDevices.value || microphone.busy.value"
        @click="microphone.refreshInputDevices()"
      >
        <RefreshCw :class="['size-3.5', microphone.loadingDevices.value && 'animate-spin']" />
      </Button>
    </div>

    <Button
      v-if="!microphone.permissionGranted.value"
      variant="outline"
      size="sm"
      class="w-full"
      :disabled="microphone.loadingDevices.value || microphone.busy.value"
      @click="requestPermission"
    >
      <Loader2 v-if="microphone.loadingDevices.value" class="animate-spin" />
      <Mic v-else />
      {{ t('actionbar.grantMicrophonePermission') }}
    </Button>

    <div v-else>
      <NativeSelect
        :model-value="microphone.selectedDeviceId.value"
        :disabled="microphone.loadingDevices.value || microphone.busy.value || microphone.inputDevices.value.length === 0"
        size="sm"
        class="w-full text-xs"
        @update:model-value="selectDevice"
      >
        <NativeSelectOption v-if="microphone.inputDevices.value.length === 0" value="">
          {{ t('actionbar.noMicrophoneDevices') }}
        </NativeSelectOption>
        <NativeSelectOption
          v-for="device in microphone.inputDevices.value"
          :key="device.deviceId"
          :value="device.deviceId"
          class="text-xs"
        >
          {{ device.label }}
        </NativeSelectOption>
      </NativeSelect>
    </div>

    <Button
      :variant="microphone.active.value ? 'destructive' : 'default'"
      size="sm"
      class="w-full"
      :disabled="microphone.busy.value"
      @click="toggle"
    >
      <Loader2 v-if="microphone.busy.value" class="animate-spin" />
      <MicOff v-else-if="microphone.active.value" />
      <Mic v-else />
      {{ microphone.busy.value
        ? stateText
        : microphone.active.value
          ? t('actionbar.micStop')
          : t('actionbar.micStart') }}
    </Button>
  </div>
</template>
