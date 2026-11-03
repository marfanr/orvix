import { inject, provide, ref, type InjectionKey, type Ref } from 'vue'

export interface ConfigPanelContext {
  isOpen: Ref<boolean>
  toggle: () => void
  close: () => void
}

export const CONFIG_PANEL_KEY: InjectionKey<ConfigPanelContext> = Symbol('config-panel')

export function provideConfigPanel(): ConfigPanelContext {
  const isOpen = ref(false)
  const ctx: ConfigPanelContext = {
    isOpen,
    toggle: () => {
      isOpen.value = !isOpen.value
    },
    close: () => {
      isOpen.value = false
    },
  }

  provide(CONFIG_PANEL_KEY, ctx)
  return ctx
}



export function useConfigPanel(): ConfigPanelContext {
  const ctx = inject(CONFIG_PANEL_KEY)
  if (!ctx) {
    throw new Error('useConfigPanel() must be called inside a room view that provides it')
  }
  return ctx
}