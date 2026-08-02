import { ref, type Ref } from 'vue'

/** WebSocket connection state */
export interface WsConnectionState {
  connected: Ref<boolean>
  reconnectAttempts: Ref<number>
  networkError: Ref<boolean>
  networkErrorMessage: Ref<string | null>
}

/** Create a new WebSocket connection state */
export function createWsConnectionState(): WsConnectionState {
  return {
    connected: ref(false),
    reconnectAttempts: ref(0),
    networkError: ref(false),
    networkErrorMessage: ref(null),
  }
}

/** Reset connection state to initial values */
export function resetWsConnectionState(state: WsConnectionState) {
  state.connected.value = false
  state.reconnectAttempts.value = 0
  state.networkError.value = false
  state.networkErrorMessage.value = null
}

/** Build WebSocket URL from current location */
export function buildWsUrl(path: string): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${protocol}//${window.location.host}${path}`
}

/** Default reconnect delay in milliseconds */
export const WS_RECONNECT_DELAY = 3000

export type StreamKind = 'video' | 'audio'

export interface StreamDeviceLostEventData {
  kind: StreamKind
  device: string
  reason: string
}

export type StreamState =
  | 'uninitialized'
  | 'ready'
  | 'streaming'
  | 'no_signal'
  | 'device_lost'
  | 'device_busy'
  | 'error'

export type StreamSignalReason =
  | 'no_signal'
  | 'no_cable'
  | 'no_sync'
  | 'out_of_range'
  | 'recovering'
  | 'device_lost'
  | 'config_changing'
  | 'mode_switching'
  | 'audio_device_lost'
  | 'audio_reconnecting'
  | 'uvc_usb_error'
  | 'uvc_capture_stall'

export interface StreamStateChangedEventData {
  kind?: StreamKind
  state: StreamState
  device?: string | null
  reason?: StreamSignalReason | null
  next_retry_ms?: number | null
}

/** WebSocket ready states */
export const WS_STATE = {
  CONNECTING: WebSocket.CONNECTING,
  OPEN: WebSocket.OPEN,
  CLOSING: WebSocket.CLOSING,
  CLOSED: WebSocket.CLOSED,
} as const
