import { computed, nextTick, ref, watch } from 'vue'
import type { CSSProperties } from 'vue'
import { useElementSize } from '@vueuse/core'

export type VideoScaleMode = 'fit' | 'actual'

export interface VideoSize {
  width: number
  height: number
}

export function useVideoScaling() {
  const workspaceRef = ref<HTMLDivElement | null>(null)
  const scaleMode = ref<VideoScaleMode>('fit')
  const sourceSize = ref<VideoSize | null>(null)
  const { width: workspaceWidth, height: workspaceHeight } = useElementSize(workspaceRef)

  const sourceSizeAvailable = computed(() => sourceSize.value !== null)
  const effectiveScaleMode = computed<VideoScaleMode>(() => (
    scaleMode.value === 'actual' && sourceSizeAvailable.value ? 'actual' : 'fit'
  ))

  const fittedSize = computed<VideoSize | null>(() => {
    const source = sourceSize.value
    if (!source || workspaceWidth.value <= 0 || workspaceHeight.value <= 0) return null

    const scale = Math.min(
      workspaceWidth.value / source.width,
      workspaceHeight.value / source.height,
    )
    return {
      width: Math.max(1, Math.floor(source.width * scale)),
      height: Math.max(1, Math.floor(source.height * scale)),
    }
  })

  const stageClass = computed(() => effectiveScaleMode.value === 'actual'
    ? 'h-max w-max min-h-full min-w-full'
    : 'h-full w-full'
  )

  const containerStyle = computed<CSSProperties>(() => {
    const size = effectiveScaleMode.value === 'actual' ? sourceSize.value : fittedSize.value
    if (size) {
      return {
        width: `${size.width}px`,
        height: `${size.height}px`,
      }
    }

    return {
      width: '100%',
      height: '100%',
      minHeight: '120px',
    }
  })

  function updateSourceSize(width: number, height: number) {
    if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) return

    const nextSize = { width: Math.round(width), height: Math.round(height) }
    if (sourceSize.value?.width === nextSize.width && sourceSize.value.height === nextSize.height) return
    sourceSize.value = nextSize
  }

  function clearSourceSize() {
    sourceSize.value = null
  }

  function setScaleMode(mode: VideoScaleMode) {
    if (mode === 'actual' && !sourceSizeAvailable.value) return
    scaleMode.value = mode
  }

  watch(
    [effectiveScaleMode, () => sourceSize.value?.width, () => sourceSize.value?.height],
    async ([mode]) => {
      if (mode !== 'actual') return
      await nextTick()
      workspaceRef.value?.scrollTo({ left: 0, top: 0 })
    },
  )

  return {
    workspaceRef,
    scaleMode,
    sourceSize,
    sourceSizeAvailable,
    stageClass,
    containerStyle,
    updateSourceSize,
    clearSourceSize,
    setScaleMode,
  }
}
