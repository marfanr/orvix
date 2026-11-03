import { inject, type InjectionKey, provide, ref, type Ref } from 'vue'
import type { ChatMessage } from '@/types/video-call.ts'

export interface MessageContext {
  messages: Ref<ChatMessage[]>
  isOpen: Ref<boolean>
  toggle: () => void
  close: () => void
}

export const MESSAGE_KEY: InjectionKey<MessageContext> = Symbol('messages')

export function provideMessages(messages: Ref<ChatMessage[]>): MessageContext {
  const isOpen = ref(false)
  const ctx: MessageContext = {
    messages,
    isOpen,
    toggle: () => {
      isOpen.value = !isOpen.value
    },
    close: () => {
      isOpen.value = false
    },
  }

  provide(MESSAGE_KEY, ctx)
  return ctx
}

export function useMessages(): MessageContext {
  const ctx = inject(MESSAGE_KEY)
  if (!ctx) {
    throw new Error('useMessages() must be called inside a view that provides it')
  }
  return ctx
}

export function formatMessageTime(date: Date) {
  return new Intl.DateTimeFormat('id-ID', {
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}