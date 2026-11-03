import { inject, provide, ref, type InjectionKey, type Ref } from 'vue'
import type { Participant } from '@/types/video-call'

export interface MembersPanelContext {
  participants: Ref<Participant[]>
  isOpen: Ref<boolean>
  toggle: () => void
  close: () => void
}

export const MEMBERS_PANEL_KEY: InjectionKey<MembersPanelContext> = Symbol('members-panel')

export function provideMembersPanel(participants: Ref<Participant[]>): MembersPanelContext {
  const isOpen = ref(false)
  const ctx: MembersPanelContext = {
    participants,
    isOpen,
    toggle: () => {
      isOpen.value = !isOpen.value
    },
    close: () => {
      isOpen.value = false
    },
  }

  provide(MEMBERS_PANEL_KEY, ctx)
  return ctx
}

export function useMembersPanel(): MembersPanelContext {
  const ctx = inject(MEMBERS_PANEL_KEY)
  if (!ctx) {
    throw new Error('useMembersPanel() must be called inside a room view that provides it')
  }
  return ctx
}
