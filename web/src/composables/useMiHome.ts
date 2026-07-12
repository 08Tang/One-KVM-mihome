import { ref, computed, watch } from 'vue'
import { getMiHomeConfig, getMiHomeDevices, type MiHomeDevice, type MiHomeConfig } from '@/api/mihome'

// Module-level shared state - persists across component mount/unmount
const config = ref<MiHomeConfig | null>(null)
const devices = ref<MiHomeDevice[]>([])
const initialized = ref(false)
const initError = ref<string | null>(null)
const configLoaded = ref(false)
let initPromise: Promise<void> | null = null
let devicePollTimer: ReturnType<typeof setInterval> | null = null

const DEVICE_POLL_INTERVAL = 60000 // 60 seconds

const mihomeEnabled = computed(() => {
  return config.value?.enabled ?? false
})

// Watch enabled state to start/stop polling
watch(mihomeEnabled, (enabled) => {
  if (enabled) {
    startDevicePolling()
  } else {
    stopDevicePolling()
  }
})

function startDevicePolling() {
  if (devicePollTimer) return
  
  devicePollTimer = setInterval(async () => {
    if (!config.value?.enabled || !config.value.api_key) return
    
    try {
      const response = await getMiHomeDevices(config.value.api_key)
      devices.value = response.devices
    } catch (error) {
      console.error('刷新设备列表失败:', error)
    }
  }, DEVICE_POLL_INTERVAL)
}

function stopDevicePolling() {
  if (devicePollTimer) {
    clearInterval(devicePollTimer)
    devicePollTimer = null
  }
}

export function useMiHome() {
  // Load config only (without fetching devices)
  const loadConfig = async () => {
    try {
      config.value = await getMiHomeConfig()
      configLoaded.value = true
      
      if (!config.value.enabled) {
        // If disabled, clear devices and initialized state
        devices.value = []
        initialized.value = false
        initError.value = null
      } else {
        // If enabled, start polling
        startDevicePolling()
      }
      
      return config.value
    } catch (error: any) {
      initError.value = error.message || '加载配置失败'
      throw error
    }
  }

  // Initialize: load config + fetch devices if enabled
  const init = async () => {
    // Return existing promise if already initializing
    if (initPromise) return initPromise

    // Already initialized successfully
    if (initialized.value) return

    initPromise = (async () => {
      try {
        initError.value = null
        
        // Load config if not loaded yet
        if (!configLoaded.value) {
          config.value = await getMiHomeConfig()
          configLoaded.value = true
        }

        if (!config.value?.enabled) {
          initError.value = '米家功能未启用'
          return
        }

        const response = await getMiHomeDevices(config.value.api_key)
        devices.value = response.devices
        initialized.value = true
        
        // Start polling after successful initialization
        startDevicePolling()
      } catch (error: any) {
        initError.value = error.message || '初始化失败'
      } finally {
        initPromise = null
      }
    })()

    return initPromise
  }

  // Reset all state (when disabling the feature)
  const reset = () => {
    stopDevicePolling()
    initialized.value = false
    initError.value = null
    initPromise = null
    config.value = null
    devices.value = []
    configLoaded.value = false
  }

  // Re-fetch devices (after config change)
  const refreshDevices = async () => {
    if (!config.value?.enabled) {
      devices.value = []
      initialized.value = false
      return
    }

    try {
      const response = await getMiHomeDevices(config.value.api_key)
      devices.value = response.devices
      initialized.value = true
      initError.value = null
      
      // Ensure polling is running
      startDevicePolling()
    } catch (error: any) {
      initError.value = error.message || '获取设备列表失败'
      initialized.value = false
      throw error
    }
  }

  return {
    config,
    devices,
    initialized,
    initError,
    configLoaded,
    mihomeEnabled,
    loadConfig,
    init,
    reset,
    refreshDevices,
  }
}
