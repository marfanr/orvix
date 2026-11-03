import { inject, provide, ref, type InjectionKey, type Ref } from 'vue'

export interface MoreFeaturesPanelContext {
  isOpen: Ref<boolean>
  toggle: () => void
  close: () => void
}

export const MORE_FEATURES_PANEL_KEY: InjectionKey<MoreFeaturesPanelContext> =
  Symbol('more-features-panel')

export function provideMoreFeaturesPanel(): MoreFeaturesPanelContext {
  const isOpen = ref(false)
  const ctx: MoreFeaturesPanelContext = {
    isOpen,
    toggle: () => {
      isOpen.value = !isOpen.value
    },
    close: () => {
      isOpen.value = false
    },
  }

  provide(MORE_FEATURES_PANEL_KEY, ctx)
  return ctx
}

export function useMoreFeaturesPanel(): MoreFeaturesPanelContext {
  const ctx = inject(MORE_FEATURES_PANEL_KEY)
  if (!ctx) {
    throw new Error('useMoreFeaturesPanel() must be called inside a room view that provides it')
  }
  return ctx
}
