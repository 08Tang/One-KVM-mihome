<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { ChevronLeft, Power, Zap, Thermometer, Monitor, RotateCcw } from 'lucide-vue-next'
import { toast } from 'vue-sonner'
import {
  getMiHomeDeviceStatus,
  controlMiHomeDevice,
  type MiHomeDeviceStatus,
} from '@/api/mihome'
import { useMiHome } from '@/composables/useMiHome'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()

// Use shared composable for config and devices
const { config, devices, initialized, initError, init } = useMiHome()

// Device status cache - fetched on demand
const deviceStatuses = ref<Record<string, MiHomeDeviceStatus>>({})
const selectedDeviceIndex = ref(0)
const scrollContainer = ref<HTMLElement | null>(null)
const isRefreshing = ref(false)

// Polling timer
let pollTimer: ReturnType<typeof setInterval> | null = null

const selectedDevice = computed(() => devices.value[selectedDeviceIndex.value])
const selectedDeviceStatus = computed(() => {
  if (!selectedDevice.value) return null
  return deviceStatuses.value[selectedDevice.value.did] || null
})

// Device list scrolling
const canScrollLeft = computed(() => {
  if (!scrollContainer.value) return false
  return scrollContainer.value.scrollLeft > 0
})

const scrollLeft = () => {
  if (scrollContainer.value) {
    scrollContainer.value.scrollBy({ left: -126, behavior: 'smooth' })
  }
}

// Mouse wheel to horizontal scroll
const handleWheel = (e: WheelEvent) => {
  if (scrollContainer.value && devices.value.length > 2) {
    e.preventDefault()
    // Convert vertical wheel to horizontal scroll
    scrollContainer.value.scrollBy({ left: e.deltaY > 0 ? 126 : -126, behavior: 'smooth' })
  }
}

// Mouse drag to scroll
let isDragging = false
let startX = 0
let scrollLeftStart = 0

const handleMouseDown = (e: MouseEvent) => {
  if (!scrollContainer.value || devices.value.length <= 2) return
  isDragging = true
  startX = e.pageX - scrollContainer.value.offsetLeft
  scrollLeftStart = scrollContainer.value.scrollLeft
  scrollContainer.value.style.cursor = 'grabbing'
}

const handleMouseMove = (e: MouseEvent) => {
  if (!isDragging || !scrollContainer.value) return
  e.preventDefault()
  const x = e.pageX - scrollContainer.value.offsetLeft
  const walk = (x - startX) * 1.5
  scrollContainer.value.scrollLeft = scrollLeftStart - walk
}

const handleMouseUp = () => {
  if (!scrollContainer.value) return
  isDragging = false
  scrollContainer.value.style.cursor = 'grab'
}

// Select device
const selectDevice = async (index: number) => {
  selectedDeviceIndex.value = index
  // Fetch status immediately when switching devices
  if (selectedDevice.value && !deviceStatuses.value[selectedDevice.value.did]) {
    await fetchDeviceStatus(selectedDevice.value.did)
  }
}

// Read-only properties (displayed in 2 columns)
const readonlyProps = computed(() => {
  if (!selectedDeviceStatus.value) return []
  const onlineStatus = {
    key: '_online',
    name: '设备状态',
    display: selectedDevice.value?.isOnline ? '在线' : '离线',
    type: 'str' as const,
    readonly: true,
    value: selectedDevice.value?.isOnline ? 'online' : 'offline',
  }
  const props = Object.entries(selectedDeviceStatus.value.status)
    .filter(([_, prop]) => prop.readonly)
    .map(([key, prop]) => ({ key, ...prop }))
  return [onlineStatus, ...props]
})

// Switch properties (boolean controls)
const switchProps = computed(() => {
  if (!selectedDeviceStatus.value) return []
  return Object.entries(selectedDeviceStatus.value.status)
    .filter(([_, prop]) => !prop.readonly && prop.type === 'bool')
    .map(([key, prop]) => {
      // 根据 display 字段判断开关状态（更可靠）
      const display = (prop.display || '').toLowerCase()
      // 匹配各种开启状态：开启、打开、通电、通电开机、正常等
      let boolValue = display.includes('开启') || 
                       display.includes('打开') || 
                       display.includes('通电') ||
                       display.includes('正常') ||
                       display.includes('on') ||
                       display.includes('true')
      
      // 开机卡的指示灯 (indicate_off) 显示反了，反过来判断
      if (key === 'indicate_off') {
        boolValue = !boolValue
      }
      
      return { key, ...prop, boolValue }
    })
})

// Select properties (dropdown controls)
const selectProps = computed(() => {
  if (!selectedDeviceStatus.value) return []
  return Object.entries(selectedDeviceStatus.value.status)
    .filter(([_, prop]) => !prop.readonly && prop.type === 'select')
    .map(([key, prop]) => ({ key, ...prop }))
})

// Icon for read-only properties
function readonlyIcon(prop: { key: string }) {
  switch (prop.key) {
    case '_online': return Power
    case 'power_watts': return Zap
    case 'temperature': return Thermometer
    case 'status': return Monitor
    default: return Power
  }
}

// Icon color for read-only properties
function readonlyIconColor(prop: { key: string; value: any }) {
  if (prop.key === '_online') {
    return prop.value === 'online' ? 'text-green-600' : 'text-slate-500'
  }
  if (typeof prop.value === 'number') {
    if (prop.key === 'power_watts') return prop.value > 0 ? 'text-green-600' : 'text-slate-500'
    if (prop.key === 'temperature') return prop.value > 50 ? 'text-red-600' : 'text-green-600'
  }
  return 'text-green-600'
}

// Handle switch change
const handleSwitchChange = async (key: string, value: boolean) => {
  if (!selectedDevice.value || !config.value) return
  
  try {
    const prop = selectedDeviceStatus.value?.status[key]
    let action = key
    let controlValue: any = value
    
    // Handle inverted properties
    // For inverted properties, we need to send the opposite value to the API
    // indicate_off is inverted: true=关闭, false=开启
    // 注意：显示已经反过来了，所以这里不需要再反转
    if (prop?.inverted) {
      controlValue = !value
    }
    
    // For boot card (cddz), the action for 'on' is 'on' with value true/false
    if (selectedDevice.value.model.includes('cddz') && key === 'on') {
      action = 'on'
      controlValue = value
    }
    
    await controlMiHomeDevice(config.value.api_key, selectedDevice.value.did, action, controlValue)
    
    // 控制成功后立即刷新设备状态
    await fetchDeviceStatus(selectedDevice.value.did)
    
    toast.success('控制成功', { description: `${selectedDevice.value.name} 的 ${prop?.name || key} 已${value ? '开启' : '关闭'}` })
  } catch (error: any) {
    toast.error('控制失败', { description: error.message })
  }
}

// Handle select change
const handleSelectChange = async (key: string, value: any) => {
  if (!selectedDevice.value || !config.value) return
  
  try {
    const numValue = parseInt(value)
    await controlMiHomeDevice(config.value.api_key, selectedDevice.value.did, key, numValue)
    
    // 控制成功后立即刷新设备状态
    await fetchDeviceStatus(selectedDevice.value.did)
    
    toast.success('设置成功', { description: `${selectedDevice.value.name} 的 ${key} 已设置为 ${value}` })
  } catch (error: any) {
    toast.error('设置失败', { description: error.message })
  }
}

// Handle boot card reboot
const handleReboot = async (type: 'normal' | 'force') => {
  if (!selectedDevice.value || !config.value) return
  
  try {
    const value = type === 'normal' ? 0 : 1
    await controlMiHomeDevice(config.value.api_key, selectedDevice.value.did, 'reboot', value)
    
    // 控制成功后立即刷新设备状态
    await fetchDeviceStatus(selectedDevice.value.did)
    
    toast.success('重启命令已发送', { description: `${selectedDevice.value.name} 正在${type === 'normal' ? '正常' : '强制'}重启` })
  } catch (error: any) {
    toast.error('重启失败', { description: error.message })
  }
}

// Fetch device status
const fetchDeviceStatus = async (did: string) => {
  if (!config.value) return
  
  try {
    const status = await getMiHomeDeviceStatus(config.value.api_key, did)
    deviceStatuses.value[did] = status
  } catch (error: any) {
    console.error('获取设备状态失败:', error)
  }
}

// Refresh current device status
const refreshDevices = async () => {
  if (!selectedDevice.value || isRefreshing.value) return
  
  isRefreshing.value = true
  try {
    await fetchDeviceStatus(selectedDevice.value.did)
  } finally {
    isRefreshing.value = false
  }
}

// Start polling
const startPolling = () => {
  if (pollTimer) return
  
  pollTimer = setInterval(async () => {
    if (selectedDevice.value && !isRefreshing.value) {
      await fetchDeviceStatus(selectedDevice.value.did)
    }
  }, 5000)
}

// Stop polling
const stopPolling = () => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

// Get device icon type
const getDeviceIconType = (device: any): 'plug' | 'bootcard' | 'default' => {
  const model = device.model?.toLowerCase() || ''
  const name = device.name?.toLowerCase() || ''

  // Boot card first (contains cddz or pc01w)
  if (model.includes('cddz') || model.includes('pc01w') || name.includes('开机卡')) return 'bootcard'
  // Then plug (contains cuco or plug or 插座)
  if (model.includes('cuco') || model.includes('plug') || name.includes('插座')) return 'plug'
  return 'default'
}

// Watch for dialog open/close
watch(() => props.open, async (isOpen) => {
  if (isOpen) {
    // Dialog opened - fetch status for selected device
    if (selectedDevice.value && !deviceStatuses.value[selectedDevice.value.did]) {
      await fetchDeviceStatus(selectedDevice.value.did)
    }
    startPolling()
  } else {
    // Dialog closed - stop polling
    stopPolling()
  }
})

// Component mounted
onMounted(async () => {
  // Initialize shared state (config + devices)
  await init()
  
  if (initError.value) {
    toast.error('初始化失败', { description: initError.value })
    return
  }
  
  if (!initialized.value) {
    return
  }
  
  // Only fetch status if dialog is already open on mount
  if (props.open && selectedDevice.value && !deviceStatuses.value[selectedDevice.value.did]) {
    await fetchDeviceStatus(selectedDevice.value.did)
    startPolling()
  }
})

// Component unmounted
onUnmounted(() => {
  stopPolling()
})
</script>

<template>
  <div class="p-2.5 space-y-2.5">
    <div class="flex items-center justify-between">
      <p class="text-xs text-muted-foreground">{{ t('mihome.description') }}</p>
      <div class="flex items-center gap-2">
        <span v-if="isRefreshing" class="text-muted-foreground" style="font-size: 8px !important;">{{ t('mihome.refreshing') }}</span>
        <RotateCcw
          class="h-3 w-3 text-muted-foreground cursor-pointer hover:text-foreground transition-colors"
          :class="{ 'animate-spin': isRefreshing }"
          @click="refreshDevices"
        />
      </div>
    </div>

    <!-- Device selection area -->
    <div class="relative overflow-hidden">
      <!-- Left scroll button -->
      <Button
        v-if="devices.length > 2 && canScrollLeft"
        variant="ghost"
        size="sm"
        class="absolute left-0 top-1/2 -translate-y-1/2 z-10 h-6 w-6 bg-background/80 backdrop-blur-sm"
        @click="scrollLeft"
      >
        <ChevronLeft class="h-3 w-3" />
      </Button>

      <!-- Device list - TabsList style -->
      <div
        ref="scrollContainer"
        class="bg-muted text-muted-foreground items-center rounded-lg p-[3px] h-8 flex overflow-x-auto scrollbar-hide"
        :class="{ 
          'grid grid-cols-2 justify-center': devices.length === 2,
          'justify-start cursor-grab': devices.length > 2,
          'justify-center': devices.length <= 2
        }"
        :style="devices.length > 2 ? 'display: flex; width: 100%;' : ''"
        @wheel="handleWheel"
        @mousedown="handleMouseDown"
        @mousemove="handleMouseMove"
        @mouseup="handleMouseUp"
        @mouseleave="handleMouseUp"
      >
        <button
          v-for="(device, index) in devices"
          :key="device.did"
          class="inline-flex items-center justify-center gap-1.5 rounded-md border border-transparent px-2 py-1 font-medium whitespace-nowrap transition-[color,box-shadow] h-7 text-xs"
          :class="[
            selectedDeviceIndex === index
              ? 'bg-background dark:bg-input/30 text-foreground dark:text-foreground shadow-sm'
              : 'text-foreground dark:text-muted-foreground hover:text-foreground',
            !device.isOnline ? 'opacity-60' : ''
          ]"
          :style="devices.length > 2 ? 'flex: 0 0 126px; width: 126px;' : 'flex: 1;'"
          @click="selectDevice(index)"
        >
          <!-- Online status indicator -->
          <span
            class="w-1.5 h-1.5 rounded-full flex-shrink-0"
            :class="device.isOnline ? 'bg-green-500' : 'bg-gray-400'"
          />
          <!-- Plug icon -->
          <svg v-if="getDeviceIconType(device) === 'plug'" class="h-3 w-3" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path d="M204 372.8V280.8c0-4.8 4-8.8 8.8-8.8H824c4.8 0 8.8 4 8.8 8.8v92c0 16.8 13.6 30.4 30.4 30.4 16.8 0 30.4-13.6 30.4-30.4V280.8c0-38.4-31.2-69.6-69.6-69.6H212.8c-38.4 0-69.6 31.2-69.6 69.6v92c0 16.8 13.6 30.4 30.4 30.4s30.4-13.6 30.4-30.4z m628-26.4c0 173.6-140.8 313.6-313.6 313.6-173.6 0-313.6-140.8-313.6-313.6 0-16.8-13.6-30.4-30.4-30.4S144 329.6 144 346.4c0 207.2 168 375.2 375.2 375.2s375.2-168 375.2-375.2c0-16.8-13.6-30.4-30.4-30.4-18.4 0-32 13.6-32 30.4zM291.2 44.8v169.6c0 16.8 13.6 30.4 30.4 30.4 16.8 0 30.4-13.6 30.4-30.4V44.8c0-16.8-13.6-30.4-30.4-30.4-16.8-0.8-30.4 13.6-30.4 30.4z m390.4-2.4v169.6c0 16.8 13.6 30.4 30.4 30.4 16.8 0 30.4-13.6 30.4-30.4V42.4c0-16.8-13.6-30.4-30.4-30.4-16-0.8-30.4 13.6-30.4 30.4zM486.4 691.2v296c0 16.8 13.6 30.4 30.4 30.4 16.8 0 30.4-13.6 30.4-30.4v-296c0-16.8-13.6-30.4-30.4-30.4-16 0-30.4 13.6-30.4 30.4z" fill="currentColor"/></svg>
          <!-- Boot card icon -->
          <svg v-else-if="getDeviceIconType(device) === 'bootcard'" class="h-3 w-3" viewBox="0 0 1275 1024" xmlns="http://www.w3.org/2000/svg"><path d="M1166.536 0H109.363a109.363 109.363 0 0 0-109.363 109.363v607.328a109.363 109.363 0 0 0 109.363 109.363h437.451v125.038h-121.028a36.454 36.454 0 1 0 0 72.909h423.963a36.454 36.454 0 0 0 0-72.909h-118.841V826.054h435.628a109.363 109.363 0 0 0 109.363-109.363V109.363A109.363 109.363 0 0 0 1166.536 0zM667.842 951.091h-60.15V826.054h60.15zM1202.99 716.691a36.454 36.454 0 0 1-36.454 36.454H109.363a36.454 36.454 0 0 1-36.454-36.454V109.363a36.454 36.454 0 0 1 36.454-36.454h1057.173a36.454 36.454 0 0 1 36.454 36.454z" fill="currentColor"/></svg>
          <!-- Default icon -->
          <Monitor v-else class="h-3 w-3" />
          {{ device.name }}
        </button>
      </div>
    </div>

    <Separator />

    <!-- Read-only properties (2 columns) -->
    <div v-if="readonlyProps.length > 0" class="grid grid-cols-2 gap-2">
      <div
        v-for="prop in readonlyProps"
        :key="prop.key"
        class="flex min-w-0 items-center gap-2 rounded-md border bg-muted/40 px-2 py-1.5"
      >
        <component :is="readonlyIcon(prop)" :class="['h-4 w-4 shrink-0', readonlyIconColor(prop)]" />
        <div class="min-w-0">
          <p class="truncate text-[11px] leading-none text-muted-foreground">{{ prop.name }}</p>
          <p class="mt-1 truncate text-xs font-medium leading-none">{{ prop.display }}</p>
        </div>
      </div>
    </div>

    <!-- Switch properties -->
    <div v-if="switchProps.length > 0" class="space-y-1">
      <div
        v-for="prop in switchProps"
        :key="prop.key"
        class="flex items-center justify-between rounded-md border bg-muted/40 px-2.5 py-1.5"
      >
        <span class="text-xs">{{ prop.name }}</span>
        <Switch
          :model-value="prop.boolValue"
          @update:model-value="handleSwitchChange(prop.key, $event)"
        />
      </div>
    </div>

    <!-- Boot card specific buttons (normal reboot, force reboot) -->
    <div v-if="selectedDevice?.model?.toLowerCase().includes('cddz')" class="space-y-1">
      <Button
        variant="outline"
        size="sm"
        class="w-full justify-start gap-2 h-7 text-xs"
        @click="handleReboot('normal')"
      >
        <RotateCcw class="h-3 w-3" />
        正常重启
      </Button>
      <Button
        variant="outline"
        size="sm"
        class="w-full justify-start gap-2 h-7 text-xs text-orange-600 hover:text-orange-700 hover:bg-orange-50 dark:hover:bg-orange-950"
        @click="handleReboot('force')"
      >
        <RotateCcw class="h-3 w-3" />
        强制重启
      </Button>
    </div>

    <!-- Select properties -->
    <div v-if="selectProps.length > 0" class="space-y-1">
      <div
        v-for="prop in selectProps"
        :key="prop.key"
        class="flex items-center justify-between rounded-md border bg-muted/40 px-2.5 py-1.5"
      >
        <span class="text-xs">{{ prop.name }}</span>
        <Select
          :model-value="String(prop.value)"
          @update:model-value="handleSelectChange(prop.key, $event)"
        >
          <SelectTrigger class="w-[122px] h-7 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="(label, value) in (prop as any).options || {}"
              :key="value"
              :value="value"
            >
              {{ label }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  </div>
</template>

<style scoped>
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
