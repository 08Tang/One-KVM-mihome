import { request } from './request'
import { sha256 } from 'js-sha256'

export interface MiHomeDevice {
  did: string
  name: string
  model: string
  mac: string
  ip: string
  added_time: string
  isOnline: boolean
}

export interface MiHomeDeviceStatus {
  did: string
  name: string
  model: string
  type: string
  icon: string
  order: string[]
  status: Record<string, MiHomeProperty>
}

export interface MiHomeProperty {
  value: any
  display: string
  name: string
  type: 'bool' | 'int' | 'float' | 'str' | 'select'
  readonly: boolean
  inverted: boolean
  options?: Record<string, string>
}

export interface MiHomeConfig {
  enabled: boolean
  api_url: string
  api_key: string
}

function generateSign(did: string, action: string, value: string, timestamp: string, salt: string): string {
  const signString = `${did}${action}${value}${timestamp}${salt}`
  return sha256(signString)
}

export async function getMiHomeConfig(): Promise<MiHomeConfig> {
  return request<MiHomeConfig>('/config/mihome')
}

export async function updateMiHomeConfig(config: MiHomeConfig): Promise<MiHomeConfig> {
  return request<MiHomeConfig>('/config/mihome', {
    method: 'PATCH',
    body: JSON.stringify(config),
  })
}

export async function checkMiHomeServer(apiUrl: string): Promise<boolean> {
  const response = await fetch(`/api/mihome/check?api_url=${encodeURIComponent(apiUrl)}`, {
    method: 'GET',
    credentials: 'include',
  })

  if (!response.ok) {
    return false
  }

  const data = await response.json()
  return data.valid === true
}

export async function getMiHomeDevices(apiKey: string): Promise<{ devices: MiHomeDevice[] }> {
  const timestamp = Math.floor(Date.now() / 1000).toString()
  const did = ''
  const action = ''
  const value = ''
  const sign = generateSign(did, action, value, timestamp, apiKey)

  const params = new URLSearchParams({
    type: 'devices',
    did,
    action,
    value,
    timestamp,
    sign,
  })

  const response = await fetch(`/api/mihome/api?${params.toString()}`, {
    method: 'GET',
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: '获取设备列表失败' }))
    throw new Error(error.message || '获取设备列表失败')
  }

  return response.json()
}

export async function getMiHomeDeviceStatus(apiKey: string, did: string): Promise<MiHomeDeviceStatus> {
  const timestamp = Math.floor(Date.now() / 1000).toString()
  const action = ''
  const value = ''
  const sign = generateSign(did, action, value, timestamp, apiKey)

  const params = new URLSearchParams({
    type: 'device_status',
    did,
    action,
    value,
    timestamp,
    sign,
  })

  const response = await fetch(`/api/mihome/api?${params.toString()}`, {
    method: 'GET',
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: '获取设备状态失败' }))
    throw new Error(error.message || '获取设备状态失败')
  }

  return response.json()
}

export async function controlMiHomeDevice(
  apiKey: string,
  did: string,
  action: string,
  value: any = null
): Promise<{ success: boolean; message: string }> {
  const timestamp = Math.floor(Date.now() / 1000).toString()
  const valueStr = value !== null ? JSON.stringify(value) : ''
  const sign = generateSign(did, action, valueStr, timestamp, apiKey)

  const params = new URLSearchParams({
    type: 'device_control',
    did,
    action,
    value: valueStr,
    timestamp,
    sign,
  })

  const response = await fetch(`/api/mihome/api?${params.toString()}`, {
    method: 'POST',
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: '控制设备失败' }))
    throw new Error(error.message || '控制设备失败')
  }

  return response.json()
}